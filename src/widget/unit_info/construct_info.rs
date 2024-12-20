use std::collections::HashMap;

use crate::{
    info,
    systemd::{self, data::UnitInfo},
};
use log::{debug, error, warn};
use serde::Deserialize;
use time_handling::get_since_and_passed_time;
use zvariant::{DynamicType, OwnedValue, Str, Type, Value};

use super::{time_handling, UnitInfoWriter};

pub(crate) fn fill_all_info(unit: &UnitInfo, unit_writer: &mut UnitInfoWriter) {
    //let mut unit_info_tokens = Vec::new();
    fill_name_description(unit_writer, unit);

    let mut path_exists = unit.pathexists();

    if !path_exists {
        match systemd::get_unit_object_path(unit) {
            Ok(object_path) => {
                info!(
                    "retreived object path for {:?}, object path {:?}",
                    unit.primary(),
                    unit.object_path()
                );

                unit.set_object_path(object_path);
                path_exists = true;
            }
            Err(error) => info!(
                "Fail retreiving object path for {:?}!\nError {:?}",
                unit.primary(),
                error
            ),
        }
    }

    let map = if path_exists {
        match systemd::fetch_system_unit_info_native(unit) {
            Ok(m) => m,
            Err(e) => {
                error!(
                    "Fail to retrieve Unit info for {:?} {:?}",
                    unit.primary(),
                    e
                );
                HashMap::new()
            }
        }
    } else {
        info!(
            "path don't exist for {:?}, object path {:?}",
            unit.primary(),
            unit.object_path()
        );
        let mut map = HashMap::new();
        let value = Value::Str("not loaded".into());

        let owned_value: OwnedValue = value
            .try_to_owned()
            .expect("This method can currently only fail on Unix platforms for Value::Fd variant.");
        map.insert("LoadState".to_owned(), owned_value);
        map
    };

    fill_description(unit_writer, &map, unit);
    fill_load_state(unit_writer, &map);
    fill_dropin(unit_writer, &map);
    fill_active_state(unit_writer, &map);
    fill_docs(unit_writer, &map);
    fill_main_pid(unit_writer, &map, unit);
    fill_tasks(unit_writer, &map);
    fill_memory(unit_writer, &map);
    fill_cpu(unit_writer, &map);
    fill_invocation(unit_writer, &map);
    fill_trigger_timers_calendar(unit_writer, &map);
    fill_trigger_timers_monotonic(unit_writer, &map);
    fill_triggers(unit_writer, &map);
    fill_listen(unit_writer, &map);
    fill_control_group(unit_writer, &map);
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

/* macro_rules! strwriter {
    ($dst:expr, $($arg:tt)*) => {
        if let Err(e) = write!($dst, $($arg)*) {
            warn!("writeln error : {:?}", e)
        }
    };
}

macro_rules! strwriterln {
    ($dst:expr, $($arg:tt)*) => {
        if let Err(e) = writeln!($dst, $($arg)*) {
            warn!("writeln error : {:?}", e)
        }
    };
} */

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

    write_key(unit_writer, "Drop in:");

    let mut is_first = true;
    let mut drops = Vec::new();
    for file_name in drop_in_paths {
        let (first, last) = file_name.rsplit_once('/').unwrap();

        if is_first {
            unit_writer.insert(first);
            unit_writer.new_line();
            is_first = false;
        } else {
            //strwriterln!(text, "{:KEY_WIDTH$} {}", " ", first);
        }
        drops.push((last, file_name));
    }

    if !drops.is_empty() {
        //unit_writer.insert(&format!("{:KEY_WIDTH$} └─", " ")));
        unit_writer.insert(&format!("{:KEY_WIDTH$} └─", " "));

        is_first = true;
        for (d, link) in drops.iter() {
            if !is_first {
                unit_writer.insert(", ");
            }

            unit_writer.hyper_link(d, link);
            is_first = false;
        }
        unit_writer.new_line();
    }
}

fn fill_active_state(unit_writer: &mut UnitInfoWriter, map: &HashMap<String, OwnedValue>) {
    let value = get_value!(map, "ActiveState");
    let state = value_str(value);

    write_key(unit_writer, "Active State:");

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

    if let Some(since) = add_since(map, state) {
        let mut text = String::new();
        text.push_str(" since ");
        text.push_str(&since.0);
        text.push_str("; ");
        text.push_str(&since.1);
        text.push_str(" ago");

        unit_writer.insert(&text);
    }

    unit_writer.new_line();
}

fn get_substate(map: &HashMap<String, OwnedValue>) -> Option<&str> {
    let value = get_value!(map, "SubState", None);
    Some(value_str(value))
}

fn add_since(map: &HashMap<String, OwnedValue>, state: &str) -> Option<(String, String)> {
    let key = match state {
        "active" => "ActiveEnterTimestamp",
        "inactive" => "InactiveEnterTimestamp",
        _ => "StateChangeTimestamp",
    };

    let value = get_value!(map, key, None);

    let duration = value_u64(value);

    let since = get_since_and_passed_time(duration);

    Some(since)
}

fn fill_description(
    unit_writer: &mut UnitInfoWriter,
    map: &HashMap<String, OwnedValue>,
    unit: &UnitInfo,
) {
    let value = get_value!(map, "Description");
    let description = value_str(value);
    fill_row(unit_writer, "Description:", description);

    if unit.description().is_empty() && !description.is_empty() {
        unit.set_description(description);
    }
}

fn fill_load_state(unit_writer: &mut UnitInfoWriter, map: &HashMap<String, OwnedValue>) {
    let value = get_value!(map, "LoadState");

    write_key(unit_writer, "Loaded:");

    unit_writer.insert(value_str(value));

    let three_param = [
        map.get("FragmentPath"),
        map.get("UnitFileState"),
        map.get("UnitFilePreset"),
    ];

    let mut all_none = true;
    for p in three_param {
        if !p.is_none() {
            if let Value::Str(inner_str) = p.unwrap() as &Value {
                if !inner_str.is_empty() {
                    all_none = false;
                    break;
                }
            }
        }
    }

    if !all_none {
        unit_writer.insert(" (");

        let [path_op, unit_file_state_op, unit_file_preset_op] = three_param;

        let mut pad_left = false;

        if let Some(path) = path_op {
            unit_writer.insert(value_str(path));
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
            unit_writer.insert(" preset: ");
            write_enabled_state(unit_writer, unit_file_preset);
        }

        unit_writer.insert(")");
    }

    unit_writer.new_line();
}

fn write_enabled_state(unit_writer: &mut UnitInfoWriter, unit_file_state: &OwnedValue) {
    let state = value_str(unit_file_state);

    match state {
        "enabled" => unit_writer.insert_active(state),
        "disabled" => unit_writer.insert_disable(state),
        _ => unit_writer.insert(state),
    };
}

fn fill_docs(unit_writer: &mut UnitInfoWriter, map: &HashMap<String, OwnedValue>) {
    let value = get_value!(map, "Documentation");

    let docs = get_array_str(value);

    let mut it = docs.iter();

    if let Some(doc) = it.next() {
        fill_row(unit_writer, "Doc:", doc);
    }

    while let Some(doc) = it.next() {
        let text = format!("{:KEY_WIDTH$} {}\n", " ", doc);
        unit_writer.insert(&text);
    }
}

fn get_array_str<'a>(value: &'a Value<'a>) -> Vec<&'a str> {
    let vec = match value as &Value {
        Value::Array(a) => {
            let mut vec = Vec::with_capacity(a.len());

            let mut it = a.iter();
            while let Some(mi) = it.next() {
                vec.push(value_str(mi));
            }

            vec
        }
        _ => {
            warn!("Wrong zvalue conversion: {:?}", value.signature());
            return Vec::new();
        }
    };
    vec
}

fn fill_memory(unit_writer: &mut UnitInfoWriter, map: &HashMap<String, OwnedValue>) {
    let value = get_value!(map, "MemoryCurrent");

    let memory_current = value_u64(value);
    if memory_current == U64MAX {
        return;
    }

    write_key(unit_writer, "Memory:");

    let value_str = human_bytes(memory_current);

    unit_writer.insert(&value_str);

    let three_param = [
        map.get("MemoryPeak"),
        map.get("MemorySwapPeak"),
        map.get("MemorySwapCurrent"),
    ];

    let mut all_none = true;
    for p in three_param {
        if !p.is_none() {
            all_none = false;
            break;
        }
    }

    if !all_none {
        unit_writer.insert(" (");

        let [peak_op, swap_peak_op, swap_op] = three_param;

        let pad_left = write_mem_param(peak_op, "peak: ", false, unit_writer);
        write_mem_param(swap_peak_op, "swap: ", pad_left, unit_writer);
        write_mem_param(swap_op, "swap peak: ", pad_left, unit_writer);

        unit_writer.insert(")");
    }

    //Memory: 1.9M (peak: 6.2M swap: 224.0K swap peak: 444.0K)

    unit_writer.new_line();
}

fn write_mem_param<'a>(
    mem_op: Option<&OwnedValue>,
    label: &str,
    pad_left: bool,
    unit_writer: &mut UnitInfoWriter,
) -> bool {
    let Some(mem) = mem_op else {
        return false;
    };

    let mem_num = value_u64(mem);
    if mem_num == U64MAX || mem_num == 0 {
        return false;
    }

    if pad_left {
        unit_writer.insert(" ");
    }

    unit_writer.insert(label);
    let mem_human = human_bytes(mem_num);
    unit_writer.insert(&mem_human);

    true
}

fn fill_main_pid(
    unit_writer: &mut UnitInfoWriter,
    map: &HashMap<String, OwnedValue>,
    unit: &UnitInfo,
) {
    let main_pid = get_main_pid(map);

    if 0 == main_pid {
        // nothing
    } else {
        let exec_val = if let Some(exec) = get_exec(map) {
            exec
        } else {
            &unit.display_name()
        };

        let v = &format!("{} ({})", main_pid, exec_val);
        fill_row(unit_writer, "Main PID:", v)
    }
}

fn get_main_pid(map: &HashMap<String, OwnedValue>) -> u32 {
    let value = get_value!(map, "MainPID", 0);

    if let Value::U32(main_pid) = value as &Value {
        return *main_pid;
    }
    0
}

fn get_exec_full<'a>(map: &'a HashMap<String, OwnedValue>) -> Option<&'a str> {
    let value = get_value!(map, "ExecStart", None);

    if let Value::Array(array) = value as &Value {
        if let Ok(Some(owned_value)) = array.get::<&Value>(0) {
            if let Value::Structure(zstruc) = owned_value {
                if let Some(val_0) = zstruc.fields().get(0) {
                    if let Value::Str(zstr) = val_0 {
                        return Some(zstr);
                    }
                }
            }
        }
    }

    None
}

fn get_exec<'a>(map: &'a HashMap<String, OwnedValue>) -> Option<&'a str> {
    if let Some(exec_full) = get_exec_full(map) {
        if let Some((_pre, last)) = exec_full.rsplit_once('/') {
            return Some(last);
        }
    }
    None
}

fn fill_cpu(unit_writer: &mut UnitInfoWriter, map: &HashMap<String, OwnedValue>) {
    let value = get_value!(map, "CPUUsageNSec");

    let value_u64 = value_u64(value);
    if value_u64 == U64MAX {
        return;
    }

    let value_str = &human_time(value_u64);
    fill_row(unit_writer, "CPU:", value_str)
}

fn fill_tasks(unit_writer: &mut UnitInfoWriter, map: &HashMap<String, OwnedValue>) {
    let value = get_value!(map, "TasksCurrent");

    let value_nb = value_u64(value);

    if value_nb == U64MAX {
        return;
    }

    let mut tasks_info = value_nb.to_string();

    if let Some(value) = map.get("TasksMax") {
        tasks_info.push_str(" (limit: ");
        let value_u64 = value_u64(value);
        tasks_info.push_str(&value_u64.to_string());
        tasks_info.push_str(")");
    }

    fill_row(unit_writer, "Tasks:", &tasks_info)
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

fn fill_trigger_timers_calendar(
    unit_writer: &mut UnitInfoWriter,
    map: &HashMap<String, OwnedValue>,
) {
    let value = get_value!(map, "TimersCalendar");

    let Value::Array(array) = value as &Value else {
        return;
    };

    for idx in 0..array.len() {
        let Ok(Some(val)) = array.get::<Value>(idx) else {
            warn!("Can't get value from array");
            continue;
        };

        match TimersCalendar::try_from(val) {
            Ok(timer) => {
                let timers = format!("{} {}", timer.timer_base, timer.calendar_specification);

                fill_row(unit_writer, "Trigger:", &timers)
            }
            Err(e) => warn!("TimersMonotonic ERROR {:?}", e),
        }
    }
}

#[derive(Clone, Value, OwnedValue)]
struct TimersMonotonic<'a> {
    timer_base: Str<'a>,
    usec_offset: u64,
    elapsation_point: u64,
}

fn fill_trigger_timers_monotonic(
    unit_writer: &mut UnitInfoWriter,
    map: &HashMap<String, OwnedValue>,
) {
    let value = get_value!(map, "TimersMonotonic");

    let Value::Array(array) = value as &Value else {
        return;
    };

    if array.is_empty() {
        return;
    }

    for idx in 0..array.len() {
        let Ok(Some(val)) = array.get::<Value>(idx) else {
            warn!("Can't get value from array at index {idx}");
            continue;
        };

        match TimersMonotonic::try_from(val) {
            Ok(timer) => {
                let string = format!(
                    "{} usec_offset={} elapsation_point={}",
                    timer.timer_base, timer.usec_offset, timer.elapsation_point
                );
                fill_row(unit_writer, "Trigger:", &string);
            }
            Err(e) => warn!("TimersMonotonic ERROR {:?}", e),
        }
    }
}

fn fill_triggers(unit_writer: &mut UnitInfoWriter, map: &HashMap<String, OwnedValue>) {
    let value = get_value!(map, "Triggers");

    let triggers = get_array_str(value);

    if triggers.is_empty() {
        return;
    }

    //TODO add the active state of the triggers

    fill_row(unit_writer, "Triggers:", &triggers.join("\n"))
}

#[derive(Deserialize, Type, PartialEq, Debug)]
struct Struct {
    field1: String,
    field2: String,
}

fn fill_listen(unit_writer: &mut UnitInfoWriter, map: &HashMap<String, OwnedValue>) {
    let value = get_value!(map, "Listen");

    let Value::Array(array) = value as &Value else {
        return;
    };

    let Ok(Some(val_listen_stc)) = array.get::<&Value>(0) else {
        return;
    };

    let Value::Structure(zstruc) = val_listen_stc else {
        return;
    };

    let Some(Value::Str(val_0)) = zstruc.fields().get(0) else {
        return;
    };

    let Some(Value::Str(val_1)) = zstruc.fields().get(1) else {
        return;
    };

    let listen = format!("{} ({})", val_1, val_0);

    fill_row(unit_writer, "Listen:", &listen)
}

fn fill_control_group(unit_writer: &mut UnitInfoWriter, map: &HashMap<String, OwnedValue>) {
    let value = get_value!(map, "ControlGroup");

    let c_group = value_str(value);

    if c_group.is_empty() {
        return;
    }

    const KEY_LABEL: &str = "CGroup:";

    if let Some(exec_full) = get_exec_full(map) {
        let main_pid = get_main_pid(map);

        write_key(unit_writer, KEY_LABEL);

        unit_writer.insert(c_group);
        unit_writer.new_line();

        let s = format!(
            "{:KEY_WIDTH$} └─{} {}",
            " ",
            &main_pid.to_string(),
            exec_full
        );

        unit_writer.insert(&s);
        unit_writer.new_line();
    } else {
        fill_row(unit_writer, KEY_LABEL, c_group)
    }
}

fn value_str<'a>(value: &'a Value<'a>) -> &'a str {
    if let Value::Str(converted) = value as &Value {
        return converted.as_str();
    }
    warn!("Wrong zvalue conversion: {:?}", value);
    ""
}

/// 2^16-1
const U64MAX: u64 = 18_446_744_073_709_551_615;
const SUFFIX: [&str; 9] = ["B", "K", "M", "G", "T", "P", "E", "Z", "Y"];
const UNIT: u64 = 1024;

fn value_u64(value: &Value) -> u64 {
    if let Value::U64(converted) = value {
        return *converted;
    }
    warn!("Wrong zvalue conversion: {:?}", value);
    U64MAX
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
        byte_new = byte_new >> 10;
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

const T_SUFFIX: [&str; 9] = ["ns", "us", "ms", "s", "Ks", "Ms", "Gs", "Ts", "Ps"];
const T_UNIT: f64 = 1000.0;

fn human_time(value: u64) -> String {
    if value <= 0 {
        return "0".to_string();
    }

    let base = (value as f64).log10() / T_UNIT.log10();
    let v = T_UNIT.powf(base - base.floor());

    let mut result: String = if value <= 1_000_000_000 {
        format!("{:.0}", v)
    } else {
        format!("{:.3}", v)
    }
    .trim_end_matches(".0")
    .to_string();

    result.push_str(" ");
    result.push_str(T_SUFFIX[base.floor() as usize]);

    result
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
    fn test2() {
        println!("{}", human_time(0));
        println!("{}", human_time(3));
        //println!("{}", human_time(U64MAX));
        println!("{}", human_time(1024));
        println!("{}", human_time(1_606_848_000));
        println!("{}", human_time(3_235_000));
        println!("{}", human_time(32_235_000));
        println!("{}", human_time(321_235_000));
        println!("{}", human_time(3_234_235_000));
    }

    #[test]
    fn test_timer_mono() {
        let local_result = chrono::TimeZone::timestamp_millis_opt(&Local, 86400000000 as i64);
        let fmt = "%b %d %T %Y";
        let date = match local_result {
            chrono::offset::LocalResult::Single(l) => l.format(fmt).to_string(),
            chrono::offset::LocalResult::Ambiguous(a, _b) => a.format(fmt).to_string(),
            chrono::offset::LocalResult::None => "NONE".to_owned(),
        };

        println!("date {}", date);

        let local_result = chrono::TimeZone::timestamp_millis_opt(&Local, 173787328907 as i64);
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
}
