use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;

use crate::consts::U64MAX;
use crate::utils::th::{self, TimestampStyle};
use crate::utils::writer::{
    HyperLinkType, UnitInfoWriter, SPECIAL_GLYPH_TREE_BRANCH, SPECIAL_GLYPH_TREE_RIGHT,
    SPECIAL_GLYPH_TREE_SPACE, SPECIAL_GLYPH_TREE_VERTICAL,
};
use crate::widget::preferences::data::PREFERENCES;
use crate::{
    swrite,
    systemd::{
        self,
        data::{UnitInfo, UnitProcess},
    },
};

use log::{debug, warn};
use zvariant::{DynamicType, OwnedValue, Str, Value};

pub(crate) fn fill_all_info(unit: &UnitInfo, unit_writer: &mut UnitInfoWriter) {
    //let mut unit_info_tokens = Vec::new();
    fill_name_description(unit_writer, unit);

    let map = match systemd::fetch_system_unit_info_native(unit) {
        Ok(m) => m,
        Err(e) => {
            warn!(
                "Fails to retrieve Unit info for {:?} {:?}",
                unit.primary(),
                e
            );
            let mut map = HashMap::new();
            let value = Value::Str("not loaded".into());

            let owned_value: OwnedValue = value.try_to_owned().expect(
                "This method can currently only fail on Unix platforms for Value::Fd variant.",
            );
            map.insert("LoadState".to_owned(), owned_value);
            map
        }
    };

    let timestamp_style = PREFERENCES.timestamp_style();

    fill_description(unit_writer, &map, unit);
    fill_follows(unit_writer, &map);
    fill_load_state(unit_writer, &map);
    fill_transient(unit_writer, &map);
    fill_dropin(unit_writer, &map);
    fill_active_state(unit_writer, &map, unit, timestamp_style);
    fill_invocation(unit_writer, &map);
    fill_triggered_by(unit_writer, &map);
    fill_device(unit_writer, &map);
    fill_where(unit_writer, &map);
    fill_what(unit_writer, &map);
    fill_trigger(unit_writer, &map, unit, timestamp_style);
    fill_triggers(unit_writer, &map);
    fill_docs(unit_writer, &map);
    fill_main_pid(unit_writer, &map, unit);
    fill_status(unit_writer, &map);
    fill_error(unit_writer, &map);
    fill_ip(unit_writer, &map);
    fill_io(unit_writer, &map);
    fill_tasks(unit_writer, &map);
    fill_fd_store(unit_writer, &map);
    fill_memory(unit_writer, &map);
    fill_listen(unit_writer, &map);
    fill_cpu(unit_writer, &map);
    fill_control_group(unit_writer, &map, unit);
}

fn fill_name_description(unit_writer: &mut UnitInfoWriter, unit: &UnitInfo) {
    fill_row(unit_writer, "Name:", &unit.primary())
}

const KEY_WIDTH: usize = 15;

macro_rules! get_value {
    ($map:expr, $key:expr) => {
        get_value!($map, $key, ())
    };

    ($map:expr, $key:expr, $dft:expr) => {{
        let Some(value) = $map.get($key) else {
            debug!("Key doesn't exists: {:?}", $key);
            return $dft;
        };
        value
    }};
}

macro_rules! clean_message {
    ($result:expr) => {
        clean_message!($result, "", ())
    };

    ($result:expr,  $log_prefix:expr) => {
        clean_message!($result, $log_prefix, ())
    };

    ($result:expr, $log_prefix:expr, $default_return:expr) => {{
        match $result {
            Ok(ok) => ok,
            Err(e) => {
                log::warn!("{} {:?}", $log_prefix, e);
                return $default_return;
            }
        }
    }};
}

fn write_key(unit_writer: &mut UnitInfoWriter, key_label: &str) {
    let s = format!("{:>KEY_WIDTH$} ", key_label);
    unit_writer.insert(&s);
}

fn fill_row(unit_writer: &mut UnitInfoWriter, key_label: &str, value: &str) {
    let s = format!("{:>KEY_WIDTH$} {}\n", key_label, value);
    unit_writer.insert(&s);
}

fn fill_dropin(unit_writer: &mut UnitInfoWriter, map: &HashMap<String, OwnedValue>) {
    let value = get_value!(map, "DropInPaths");

    let drop_in_paths = get_array_str(value);

    if drop_in_paths.is_empty() {
        return;
    }

    let mut map = BTreeMap::new();

    for file_name in drop_in_paths {
        let (first, last) = match file_name.rsplit_once('/') {
            Some((first, last)) => (first, last),
            None => (file_name, ""),
        };

        let suffixes = map.entry(first).or_insert(Vec::new());
        suffixes.push((last, file_name));
    }

    let mut is_first1 = true;
    for (prefix, suffixes) in map {
        let key_label = if is_first1 {
            is_first1 = false;
            "Drop in:"
        } else {
            ""
        };

        let s = format!("{:>KEY_WIDTH$} {}\n", key_label, prefix);
        unit_writer.insert(&s);

        let mut is_first = true;
        for (d, link) in suffixes.iter() {
            if is_first {
                unit_writer.insert(&format!("{:KEY_WIDTH$} {}", "", SPECIAL_GLYPH_TREE_RIGHT));
                is_first = false;
            } else {
                unit_writer.insert(", ");
            }

            unit_writer.hyperlink(d, link, HyperLinkType::File);
        }
        unit_writer.newline();
    }
}

fn fill_active_state(
    unit_writer: &mut UnitInfoWriter,
    map: &HashMap<String, OwnedValue>,
    unit: &UnitInfo,
    timestamp_style: TimestampStyle,
) {
    let value = get_value!(map, "ActiveState");
    let state = value_to_str(value);

    write_key(unit_writer, "Active:");

    let mut state_text = String::from(state);
    if let Some(substate) = get_substate(map) {
        state_text.push_str(" (");
        state_text.push_str(substate);
        state_text.push(')');
    }

    if state == "active" {
        unit_writer.insert_active(&state_text);
    } else {
        unit_writer.insert(&state_text);
    };

    if let Some(since) = add_since(map, state, timestamp_style) {
        unit_writer.insert(&format!(" since {}; {}\n", since.0, since.1));
        fill_duration(unit_writer, map, unit);
    } else {
        unit_writer.newline();
    }
}

macro_rules! timestamp_is_set {
    ($t:expr) => {
        $t > 0 && $t != U64MAX
    };
}

fn fill_duration(
    unit_writer: &mut UnitInfoWriter,
    map: &HashMap<String, OwnedValue>,
    unit: &UnitInfo,
) {
    let unit_type: systemd::enums::UnitType = unit.unit_type().into();
    if !systemd::enums::UnitType::Target.eq(&unit_type) {
        return;
    }

    let active_enter_timestamp = map
        .get("ActiveEnterTimestamp")
        .map_or(U64MAX, |v| value_to_u64(v));
    let active_exit_timestamp = map
        .get("ActiveExitTimestamp")
        .map_or(U64MAX, |v| value_to_u64(v));

    if timestamp_is_set!(active_enter_timestamp)
        && timestamp_is_set!(active_exit_timestamp)
        && active_exit_timestamp >= active_enter_timestamp
    {
        let duration = active_exit_timestamp - active_enter_timestamp;
        let timespan = th::format_timespan(duration, th::MSEC_PER_SEC);
        fill_row(unit_writer, "Duration:", &timespan);
    }
}

fn get_substate(map: &HashMap<String, OwnedValue>) -> Option<&str> {
    let value = get_value!(map, "SubState", None);
    Some(value_to_str(value))
}

fn add_since(
    map: &HashMap<String, OwnedValue>,
    state: &str,
    timestamp_style: TimestampStyle,
) -> Option<(String, String)> {
    let key = match state {
        "active" | "reloading" | "refreshing" => "ActiveEnterTimestamp",
        "inactive" | "failed" => "InactiveEnterTimestamp",
        "activating" => "InactiveExitTimestamp",
        _ => "ActiveExitTimestamp",
    };

    let value = get_value!(map, key, None);

    let duration = value_to_u64(value);

    if duration != 0 {
        let since = th::get_since_and_passed_time(duration as i64, timestamp_style);
        Some(since)
    } else {
        None
    }
}

fn fill_description(
    unit_writer: &mut UnitInfoWriter,
    map: &HashMap<String, OwnedValue>,
    unit: &UnitInfo,
) {
    let value = get_value!(map, "Description");
    let description = value_to_str(value);
    fill_row(unit_writer, "Description:", description);

    if unit.description().is_empty() && !description.is_empty() {
        unit.set_description(description);
    }
}

fn fill_load_state(unit_writer: &mut UnitInfoWriter, map: &HashMap<String, OwnedValue>) {
    let value = get_value!(map, "LoadState");

    write_key(unit_writer, "Loaded:");

    unit_writer.insert(value_to_str(value));

    let three_param = [
        map.get("FragmentPath"),
        map.get("UnitFileState"),
        map.get("UnitFilePreset"),
    ];

    let mut all_none = true;
    for p in three_param {
        let Some(value) = p else {
            continue;
        };

        if let Value::Str(inner_str) = value as &Value {
            if !inner_str.is_empty() {
                all_none = false;
                break;
            }
        }
    }

    if !all_none {
        unit_writer.insert(" (");

        let [path_op, unit_file_state_op, unit_file_preset_op] = three_param;

        let mut pad_left = false;

        if let Some(path) = path_op {
            let p = value_to_str(path);
            unit_writer.hyperlink(p, p, HyperLinkType::File);
            pad_left = true;
        }

        if let Some(unit_file_state) = unit_file_state_op {
            if pad_left {
                unit_writer.insert("; ");
            }

            write_enabled_state(unit_writer, unit_file_state);

            pad_left = true;
        }

        if let Some(unit_file_preset) = unit_file_preset_op {
            if pad_left {
                unit_writer.insert("; ");
            }
            unit_writer.insert("preset: ");
            write_enabled_state(unit_writer, unit_file_preset);
        }

        unit_writer.insert(")");
    }

    unit_writer.newline();
}

fn write_enabled_state(unit_writer: &mut UnitInfoWriter, unit_file_state: &OwnedValue) {
    let state = value_to_str(unit_file_state);

    match state {
        "enabled" => unit_writer.insert_active(state),
        "disabled" => unit_writer.insert_disable(state),
        _ => unit_writer.insert(state),
    };
}

fn fill_follows(unit_writer: &mut UnitInfoWriter, map: &HashMap<String, OwnedValue>) {
    let value = get_value!(map, "Following");
    let value = value_to_str(value);

    if value.is_empty() {
        return;
    }

    let s = format!("unit currently follows state of {value}");
    fill_row(unit_writer, "Follows:", &s);
}

fn fill_transient(unit_writer: &mut UnitInfoWriter, map: &HashMap<String, OwnedValue>) {
    let value = get_value!(map, "Transient");

    let transient = clean_message!(bool::try_from(value), "Wrong zvalue conversion");

    if transient {
        let value = if transient { "yes" } else { "no" };
        fill_row(unit_writer, "Transient:", value);
    }
}

fn fill_status(unit_writer: &mut UnitInfoWriter, map: &HashMap<String, OwnedValue>) {
    let value = get_value!(map, "StatusText");
    let value = value_to_str(value);

    if !value.is_empty() {
        let s = format!("{:>KEY_WIDTH$} ", "Status:");
        unit_writer.insert(&s);
        unit_writer.insert_status(value);
        unit_writer.newline();
    }
}

fn fill_error(unit_writer: &mut UnitInfoWriter, map: &HashMap<String, OwnedValue>) {
    let status_errno = valop_to_u32(map.get("StatusErrno"), 0);
    let status_bus_error = valop_to_str(map.get("StatusBusError"), "");
    let status_varlink_error = valop_to_str(map.get("StatusVarlinkError"), "");

    if status_errno == 0 && status_bus_error.is_empty() && status_varlink_error.is_empty() {
        return;
    }

    write_key(unit_writer, "Error:");

    let mut prefix = "";

    if status_errno > 0 {
        let mut text = format!("{prefix} {status_errno}");

        if let Some(strerror) = strerror(status_errno as i32) {
            swrite!(text, " ({strerror})");
        }

        unit_writer.insert(&text);
        prefix = "; ";
    }

    if !status_bus_error.is_empty() {
        let text = format!("{prefix} D-Bus: {status_bus_error}");
        unit_writer.insert(&text);
        prefix = "; ";
    }

    if !status_varlink_error.is_empty() {
        let text = format!("{prefix} Varlink: {status_varlink_error}");
        unit_writer.insert(&text);
    }

    unit_writer.newline();
}

fn strerror(err_no: i32) -> Option<String> {
    const ERRNO_BUF_LEN: usize = 1024;
    //let mut str_error = String::with_capacity(1024);
    let mut str_error: Vec<u8> = vec![0; ERRNO_BUF_LEN];
    //let mut str_error = [0; ERRNO_BUF_LEN];
    unsafe {
        let str_error_raw_ptr = str_error.as_mut_ptr() as *mut libc::c_char;
        libc::strerror_r(err_no, str_error_raw_ptr, ERRNO_BUF_LEN);

        let nul_range_end = str_error
            .iter()
            .position(|&c| c == b'\0')
            .unwrap_or(ERRNO_BUF_LEN);

        str_error.truncate(nul_range_end);
        if let Ok(str_error) = String::from_utf8(str_error) {
            Some(str_error)
        } else {
            None
        }
    }
}

fn fill_device(unit_writer: &mut UnitInfoWriter, map: &HashMap<String, OwnedValue>) {
    fill_what_string(unit_writer, map, "SysFSPath", "Device:")
}

fn fill_where(unit_writer: &mut UnitInfoWriter, map: &HashMap<String, OwnedValue>) {
    fill_what_string(unit_writer, map, "Where", "Where:")
}

fn fill_what(unit_writer: &mut UnitInfoWriter, map: &HashMap<String, OwnedValue>) {
    fill_what_string(unit_writer, map, "What", "What:")
}

fn fill_what_string(
    unit_writer: &mut UnitInfoWriter,
    map: &HashMap<String, OwnedValue>,
    key: &str,
    key_label: &str,
) {
    let value = get_value!(map, key);
    let value = value_to_str(value);
    if !value.is_empty() {
        fill_row(unit_writer, key_label, value);
    }
}

fn fill_docs(unit_writer: &mut UnitInfoWriter, map: &HashMap<String, OwnedValue>) {
    let value = get_value!(map, "Documentation");

    let Value::Array(array) = value as &Value else {
        return;
    };

    for idx in 0..array.len() {
        let Ok(Some(val)) = array.get::<Value>(idx) else {
            warn!("Can't get value from array");
            continue;
        };

        let key = if idx == 0 { "Doc:" } else { "" };

        write_key(unit_writer, key);
        let doc = value_to_str(&val);
        insert_doc(unit_writer, doc);
        unit_writer.newline();
    }
}

fn insert_doc(unit_writer: &mut UnitInfoWriter, doc: &str) {
    if doc.starts_with("man:") {
        unit_writer.hyperlink(doc, doc, HyperLinkType::Man);
    } else if doc.starts_with("http") {
        unit_writer.hyperlink(doc, doc, HyperLinkType::Http);
    } else {
        unit_writer.insert(doc);
    }
}

fn get_array_str<'a>(value: &'a Value<'a>) -> Vec<&'a str> {
    match value as &Value {
        Value::Array(a) => {
            let mut vec = Vec::with_capacity(a.len());
            for mi in a.iter() {
                vec.push(value_to_str(mi));
            }
            vec
        }
        _ => {
            warn!("Wrong zvalue conversion: {:?}", value.signature());
            Vec::new()
        }
    }
}

fn fill_fd_store(unit_writer: &mut UnitInfoWriter, map: &HashMap<String, OwnedValue>) {
    let n_fd_store = valop_to_u32(map.get("NFileDescriptorStore"), 0);
    let fd_store_max = valop_to_u32(map.get("FileDescriptorStoreMax"), 0);

    if n_fd_store == 0 && fd_store_max == 0 {
        return;
    }

    write_key(unit_writer, "FD Store:");

    unit_writer.insert(&n_fd_store.to_string());
    unit_writer.insert_grey(&format!(" (limit: {fd_store_max})\n"));
}

fn fill_memory(unit_writer: &mut UnitInfoWriter, map: &HashMap<String, OwnedValue>) {
    let memory_current = valop_to_u64(map.get("MemoryCurrent"), U64MAX);

    if memory_current == U64MAX {
        return;
    }

    write_key(unit_writer, "Memory:");

    let value_str = human_bytes(memory_current);

    unit_writer.insert(&value_str);

    type MemoryManaging = (&'static str, u64, &'static str, fn(u64) -> bool);

    let mut memories: [MemoryManaging; 17] = [
        ("MemoryMin", U64MAX, "min: ", is_zero),
        ("MemoryLow", U64MAX, "low: ", is_zero),
        ("StartupMemoryLow", U64MAX, "low (startup): ", is_zero),
        ("MemoryHigh", U64MAX, "high: ", is_limit_max), //3
        (
            "StartupMemoryHigh",
            U64MAX,
            "high (startup): ",
            is_limit_max,
        ),
        ("MemoryMax", U64MAX, "max: ", is_limit_max), //5
        ("StartupMemoryMax", U64MAX, "max (startup): ", is_limit_max),
        ("MemorySwapMax", U64MAX, "swap max: ", is_limit_max),
        (
            "StartupMemorySwapMax",
            U64MAX,
            "swap max (startup): ",
            is_limit_max,
        ),
        ("MemoryZSwapMax", U64MAX, "zswap max: ", is_limit_max),
        (
            "StartupMemoryZSwapMax",
            U64MAX,
            "zswap max (startup): ",
            is_limit_max,
        ),
        ("MemoryLimit", U64MAX, "limit: ", is_limit_max),
        ("MemoryAvailable", U64MAX, "available: ", is_limit_max),
        ("MemoryPeak", U64MAX, "peak: ", is_limit_max),
        ("MemorySwapCurrent", U64MAX, "swap: ", is_limit_max_or_zero),
        (
            "MemorySwapPeak",
            U64MAX,
            "swap peak: ",
            is_limit_max_or_zero,
        ),
        (
            "MemoryZSwapCurrent",
            U64MAX,
            "zswap: ",
            is_limit_max_or_zero,
        ),
    ];

    for (key, value, _, _) in memories.iter_mut() {
        if let Some(bus_value) = map.get(key as &str) {
            if let Value::U64(converted) = bus_value as &Value {
                *value = *converted;
            }
        }
    }

    let mut is_first = true;
    let mut out = String::with_capacity(100);
    for (key, value, label, not_valid) in memories {
        if not_valid(value)
            || (
                key == "MemoryAvailable"
            && is_limit_max(memories[3].1)//memory_high
            && is_limit_max(memories[5].1)
                //memory_max
            )
        {
            continue;
        }

        if is_first {
            out.push_str(" (");
            is_first = false;
        } else {
            out.push_str(", ");
        }

        out.push_str(label);
        let mem_human = human_bytes(value);
        out.push_str(&mem_human);
    }

    if !is_first {
        out.push(')');
        unit_writer.insert(&out);
    }

    unit_writer.newline();
}

fn is_limit_max_or_zero(value: u64) -> bool {
    value == 0 || value == U64MAX
}

fn is_zero(value: u64) -> bool {
    value == 0
}

fn is_limit_max(value: u64) -> bool {
    value == U64MAX
}

fn fill_main_pid(
    unit_writer: &mut UnitInfoWriter,
    map: &HashMap<String, OwnedValue>,
    unit: &UnitInfo,
) {
    let main_pid = get_main_pid(map);

    if 0 == main_pid {
        // nothing
        return;
    }

    let exec_val = if let Some(exec) = get_exec(map) {
        exec
    } else {
        unit.display_name()
    };

    let v = &format!("{} ({})", main_pid, exec_val);
    fill_row(unit_writer, "Main PID:", v)
}

fn get_main_pid(map: &HashMap<String, OwnedValue>) -> u32 {
    let value = get_value!(map, "MainPID", 0);

    if let Value::U32(main_pid) = value as &Value {
        return *main_pid;
    }
    0
}

#[derive(Clone, Value, Debug, OwnedValue)]
struct ExecStart<'a> {
    path: Str<'a>,
    argv: Vec<Str<'a>>,
    ignore_errors: bool,

    //TODO check the param naming
    start_time: u64,
    stop_time: u64,
    field6: u64,
    field7: u64,
    field8: u32,
    code: i32,
    status: i32,
}

// Value: Array(Dynamic { child: Structure(Dynamic { fields: [Str, Array(Dynamic { child: Str }), Bool, U64, U64, U64, U64, U32, I32, I32] }) })
fn get_exec_full(map: &HashMap<String, OwnedValue>) -> Option<ExecStart> {
    let value = get_value!(map, "ExecStart", None);

    debug!(
        "ExecStart Signature {:?} Value: {:?}",
        value.value_signature(),
        value
    );

    let Value::Array(array) = value as &Value else {
        return None;
    };

    for idx in 0..array.len() {
        let Ok(Some(val)) = array.get::<Value>(idx) else {
            warn!("Can't get value from array");
            continue;
        };

        let exec_start = clean_message!(ExecStart::try_from(val), "ExecStart", None);

        /*         let array_of_str: Vec<_> = exec_start.argv.iter().map(|s| s.as_str()).collect();

        let cmd_line_joined = array_of_str.join(" "); */

        return Some(exec_start);
    }

    None
}

fn get_exec(map: &HashMap<String, OwnedValue>) -> Option<String> {
    if let Some(exec_full) = get_exec_full(map) {
        if let Some((_pre, last)) = exec_full.path.rsplit_once('/') {
            return Some(last.to_string());
        }
    }
    None
}

fn fill_cpu(unit_writer: &mut UnitInfoWriter, map: &HashMap<String, OwnedValue>) {
    let cpu_usage_nsec = valop_to_u64(map.get("CPUUsageNSec"), U64MAX);

    if cpu_usage_nsec == U64MAX {
        return;
    }

    let value_str = th::format_timespan(cpu_usage_nsec / th::NSEC_PER_USEC, th::USEC_PER_MSEC);
    fill_row(unit_writer, "CPU:", &value_str)
}

fn fill_ip(unit_writer: &mut UnitInfoWriter, map: &HashMap<String, OwnedValue>) {
    let ip_ingress_bytes = valop_to_u64(map.get("IPIngressBytes"), U64MAX);
    let ip_egress_bytes = valop_to_u64(map.get("IPEgressBytes"), U64MAX);

    if ip_ingress_bytes == U64MAX || ip_egress_bytes == U64MAX {
        return;
    }

    fill_row(
        unit_writer,
        "IP:",
        &format!(
            "{} in, {} out",
            human_bytes(ip_ingress_bytes),
            human_bytes(ip_egress_bytes)
        ),
    );
}

fn fill_io(unit_writer: &mut UnitInfoWriter, map: &HashMap<String, OwnedValue>) {
    let io_read_bytes = valop_to_u64(map.get("IOReadBytes"), U64MAX);
    let io_write_bytes = valop_to_u64(map.get("IOWriteBytes"), U64MAX);

    if io_read_bytes == U64MAX || io_write_bytes == U64MAX {
        return;
    }

    fill_row(
        unit_writer,
        "IP:",
        &format!(
            "{} read, {} written",
            human_bytes(io_read_bytes),
            human_bytes(io_write_bytes)
        ),
    );
}

fn fill_tasks(unit_writer: &mut UnitInfoWriter, map: &HashMap<String, OwnedValue>) {
    let tasks_current = valop_to_u64(map.get("TasksCurrent"), U64MAX);

    if tasks_current == U64MAX {
        return;
    }

    write_key(unit_writer, "Tasks:");

    let tasks_info = tasks_current.to_string();
    unit_writer.insert(&tasks_info);

    let tasks_max = valop_to_u64(map.get("TasksMax"), U64MAX);
    if tasks_max != U64MAX {
        unit_writer.insert_grey(&format!(" (limit: {tasks_max})"));
    }

    unit_writer.newline();
    //fill_row(unit_writer, "Tasks:", &tasks_info)
}

fn fill_invocation(unit_writer: &mut UnitInfoWriter, map: &HashMap<String, OwnedValue>) {
    let value = get_value!(map, "InvocationID");

    let Value::Array(array) = value as &Value else {
        return;
    };

    if array.is_empty() {
        return;
    };

    let mut invocation = String::with_capacity(32);
    for idx in 0..array.len() {
        let Ok(Some(val)) = array.get::<Value>(idx) else {
            warn!("Can't get value from array at index {idx}");
            continue;
        };

        let Value::U8(converted) = val else {
            warn!("Can't convert value to u8");
            continue;
        };

        let hexa = format!("{:x}", converted);

        invocation.push_str(&hexa);
    }

    fill_row(unit_writer, "Invocation:", &invocation)
}

#[derive(Clone, Value, OwnedValue)]
struct TimersCalendar<'a> {
    timer_base: Str<'a>,
    calendar_specification: Str<'a>,
    elapsation_point: u64,
}

fn fill_trigger(
    unit_writer: &mut UnitInfoWriter,
    map: &HashMap<String, OwnedValue>,
    unit: &UnitInfo,
    timestamp_style: TimestampStyle,
) {
    let unit_type: systemd::enums::UnitType = unit.unit_type().into();
    if !systemd::enums::UnitType::Timer.eq(&unit_type) {
        return;
    }

    let next_elapse_realtime = map
        .get("NextElapseUSecRealtime")
        .map_or(U64MAX, |v| value_to_u64(v));
    let next_elapse_monotonic = map
        .get("NextElapseUSecMonotonic")
        .map_or(U64MAX, |v| value_to_u64(v));

    let now_realtime = th::now_realtime();
    let now_monotonic = th::now_monotonic();

    let next_elapse = calc_next_elapse(
        now_realtime,
        now_monotonic,
        next_elapse_realtime,
        next_elapse_monotonic,
    );

    let trigger_msg = if timestamp_is_set!(next_elapse) {
        let (first, second) = th::get_since_and_passed_time(next_elapse as i64, timestamp_style);

        format!("{first}; {second}")
    } else {
        "n/a".to_owned()
    };

    fill_row(unit_writer, "Trigger:", &trigger_msg);
}

///from systemd
fn calc_next_elapse(
    now_realtime: u64,
    now_monotonic: u64,
    next_elapse_realtime: u64,
    next_elapse_monotonic: u64,
) -> u64 {
    if timestamp_is_set!(next_elapse_monotonic) {
        let converted = if next_elapse_monotonic > now_monotonic {
            now_realtime + (next_elapse_monotonic - now_monotonic)
        } else {
            now_realtime - (now_monotonic - next_elapse_monotonic)
        };

        if timestamp_is_set!(next_elapse_realtime) {
            converted.min(next_elapse_realtime)
        } else {
            converted
        }
    } else {
        next_elapse_realtime
    }
}

#[derive(Clone, Value, OwnedValue)]
struct TimersMonotonic<'a> {
    timer_base: Str<'a>,
    usec_offset: u64,
    elapsation_point: u64,
}

fn fill_triggers(unit_writer: &mut UnitInfoWriter, map: &HashMap<String, OwnedValue>) {
    let value = get_value!(map, "Triggers");

    let triggers = get_array_str(value);

    if triggers.is_empty() {
        return;
    }

    let mut is_first = true;
    for trigger_unit in triggers {
        let key_label = if is_first {
            is_first = false;
            "Trigger:"
        } else {
            ""
        };

        write_key(unit_writer, key_label);

        match systemd::get_unit_active_state(trigger_unit) {
            Ok(state) => {
                unit_writer.insert_state(state);
            }
            Err(e) => {
                warn!("Can't find state of {trigger_unit}, {:?}", e);
                unit_writer.insert(" ");
            }
        };

        unit_writer.insert(" ");
        unit_writer.hyperlink(trigger_unit, trigger_unit, HyperLinkType::Unit);
        unit_writer.newline();
    }
}

//TODO add units states
fn fill_triggered_by(unit_writer: &mut UnitInfoWriter, map: &HashMap<String, OwnedValue>) {
    let value = get_value!(map, "TriggeredBy");

    let triggers = get_array_str(value);

    let mut is_first = true;
    for trigger_unit in triggers {
        let key_label = if is_first {
            is_first = false;
            "TriggeredBy:"
        } else {
            ""
        };

        write_key(unit_writer, key_label);

        match systemd::get_unit_active_state(trigger_unit) {
            Ok(state) => {
                unit_writer.insert_state(state);
            }
            Err(e) => {
                warn!("Can't find state of {trigger_unit}, {:?}", e);
                unit_writer.insert(" ");
            }
        };

        unit_writer.insert(" ");
        unit_writer.hyperlink(trigger_unit, trigger_unit, HyperLinkType::Unit);
        unit_writer.newline();
    }
}

#[derive(Value, OwnedValue)]
struct ListenStruct<'a> {
    listen_type: Str<'a>,
    path: Str<'a>,
}

fn fill_listen(unit_writer: &mut UnitInfoWriter, map: &HashMap<String, OwnedValue>) {
    let value = get_value!(map, "Listen");

    let Value::Array(array) = value as &Value else {
        return;
    };

    for i in 0..array.len() {
        let Ok(Some(val_listen_stc)) = array.get::<Value>(i) else {
            continue;
        };

        let listen_struct = clean_message!(ListenStruct::try_from(val_listen_stc), "Listen info");

        let key = if i == 0 { "Listen:" } else { "" };
        write_key(unit_writer, key);

        let listen = format!("{} ({})\n", listen_struct.path, listen_struct.listen_type);
        unit_writer.insert(&listen);
    }
}

fn fill_control_group(
    unit_writer: &mut UnitInfoWriter,
    map: &HashMap<String, OwnedValue>,
    unit: &UnitInfo,
) {
    let value = get_value!(map, "ControlGroup");

    let c_group = value_to_str(value);

    if c_group.is_empty() {
        return;
    }

    fill_row(unit_writer, "CGroup:", c_group);

    //TODO put in separate thread maybe?
    let mut unit_processes =
        clean_message!(systemd::retreive_unit_processes(unit), "Get processes");

    let main_unit_name = unit.primary();

    // get the main unit first
    if let Some(unit_process_set) = unit_processes.remove(&main_unit_name) {
        for (sub_idx, unit_process) in unit_process_set.iter().enumerate() {
            let is_last = sub_idx == unit_process_set.len() - 1;
            print_process(unit_writer, "", unit_process, is_last);
        }
    }

    for (idx, (_, unit_process_set)) in unit_processes.iter().enumerate() {
        let is_last = idx == unit_processes.len() - 1;
        //let is_first = idx == 0;
        let mut padding = "";

        for (sub_idx, unit_process) in unit_process_set.iter().enumerate() {
            let is_first_sub = sub_idx == 0;
            let is_last_sub = sub_idx == unit_process_set.len() - 1;

            if is_first_sub {
                let glyph = if is_last {
                    SPECIAL_GLYPH_TREE_RIGHT
                } else {
                    SPECIAL_GLYPH_TREE_BRANCH
                };

                unit_writer.insert(&format!("{:KEY_WIDTH$} {}", " ", glyph));
                unit_writer.insert(unit_process.unit_name());
                unit_writer.newline();

                padding = if !is_last {
                    SPECIAL_GLYPH_TREE_VERTICAL
                } else {
                    SPECIAL_GLYPH_TREE_SPACE
                };
            }

            print_process(unit_writer, padding, unit_process, is_last_sub);
        }
    }
}

fn print_process(
    unit_writer: &mut UnitInfoWriter,
    padding: &str,
    unit_process: &UnitProcess,
    is_last_sub: bool,
) {
    let glyph = if !is_last_sub {
        SPECIAL_GLYPH_TREE_BRANCH
    } else {
        SPECIAL_GLYPH_TREE_RIGHT
    };

    unit_writer.insert(&format!("{:KEY_WIDTH$} {}{}", " ", padding, glyph));

    let process_info = format!("{} {}", unit_process.pid, unit_process.name);

    unit_writer.insert_grey(&process_info);
    unit_writer.newline();
}

fn value_to_str<'a>(value: &'a Value<'a>) -> &'a str {
    if let Value::Str(converted) = value as &Value {
        return converted.as_str();
    }
    warn!("Wrong zvalue conversion to String: {:?}", value);
    ""
}

const SUFFIX: [&str; 9] = ["B", "K", "M", "G", "T", "P", "E", "Z", "Y"];
const UNIT: u64 = 1024;

fn value_to_u64(value: &Value) -> u64 {
    if let Value::U64(converted) = value {
        return *converted;
    }
    warn!("Wrong zvalue conversion to u64: {:?}", value);
    U64MAX
}

fn valop_to_u64(value: Option<&OwnedValue>, default: u64) -> u64 {
    let Some(value) = value else {
        return default;
    };

    if let Value::U64(converted) = value as &Value {
        *converted
    } else {
        warn!("Wrong zvalue conversion to u64: {:?}", value);
        default
    }
}

fn valop_to_u32(value: Option<&OwnedValue>, default: u32) -> u32 {
    let Some(value) = value else {
        return default;
    };

    if let Value::U32(converted) = value as &Value {
        *converted
    } else {
        warn!("Wrong zvalue conversion to u32: {:?}", value);
        default
    }
}

fn valop_to_str<'a>(value: Option<&'a OwnedValue>, default: &'a str) -> &'a str {
    let Some(value) = value else {
        return default;
    };

    if let Value::Str(converted) = value as &Value {
        converted
    } else {
        warn!("Wrong zvalue conversion to str: {:?}", value);
        default
    }
}

/// Converts bytes to human-readable values in base 10
fn human_bytes(bytes: u64) -> String {
    let mut base: usize = 0;

    let mut byte_new = bytes;

    loop {
        if UNIT > byte_new {
            break;
        }
        base += 1;
        byte_new >>= 10;
    }

    let pbase = UNIT.pow(base as u32);
    let value = bytes as f64 / pbase as f64;

    let mut human_str = if base == 0 {
        bytes.to_string()
    } else {
        format!("{:.1}", value)
    };

    if let Some(suffix) = SUFFIX.get(base) {
        human_str.push_str(suffix);
    }

    human_str
}

#[cfg(test)]
mod tests {

    use chrono::Local;

    use super::*;
    #[test]
    fn test1() {
        println!("{}", human_bytes(0));
        println!("{}", human_bytes(3));
        println!("{}", human_bytes(18446744073709551615));
        println!("{}", human_bytes(1024));
        println!("{}", human_bytes(1024));
        println!("{}", human_bytes(2048));
        println!("{}", human_bytes(2000));
        println!("{}", human_bytes(1950));

        println!("{}", human_bytes(2_048_000));
    }

    #[test]
    fn test_timer_mono() {
        let local_result = chrono::TimeZone::timestamp_millis_opt(&Local, 86400000000);
        let fmt = "%b %d %T %Y";
        let date = match local_result {
            chrono::offset::LocalResult::Single(l) => l.format(fmt).to_string(),
            chrono::offset::LocalResult::Ambiguous(a, _b) => a.format(fmt).to_string(),
            chrono::offset::LocalResult::None => "NONE".to_owned(),
        };

        println!("date {}", date);

        let local_result = chrono::TimeZone::timestamp_millis_opt(&Local, 173787328907);
        let fmt = "%b %d %T %Y";
        let date = match local_result {
            chrono::offset::LocalResult::Single(l) => l.format(fmt).to_string(),
            chrono::offset::LocalResult::Ambiguous(a, _b) => a.format(fmt).to_string(),
            chrono::offset::LocalResult::None => "NONE".to_owned(),
        };

        println!("date {}", date);
    }

    #[test]
    fn test_invocation() {
        let _a = [
            23, 184, 156, 61, 114, 189, 74, 235, 186, 102, 85, 32, 183, 33, 38, 165,
        ];
        //Invocation: 17b89c3d72bd4aebba665520b72126a5
    }

    #[test]
    fn test_strerror() {
        for i in 0..35 {
            let out = strerror(i);
            println!("Error {i} {:?}", out);
        }
    }
}
