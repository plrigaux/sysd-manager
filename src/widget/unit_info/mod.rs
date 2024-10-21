use std::collections::HashMap;

use crate::{
    systemd::{self, data::UnitInfo},
    widget::unit_file_panel::dosini::Token,
};
use log::{debug, error, warn};
use serde::Deserialize;
use std::fmt::Write;
use time_handling::get_since_and_passed_time;
use zvariant::{DynamicType, OwnedValue, Type, Value};

mod time_handling;

use gtk::{glib, subclass::prelude::ObjectSubclassIsExt};

glib::wrapper! {
    pub struct UnitInfoPanel(ObjectSubclass<imp::UnitInfoPanelImp>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl UnitInfoPanel {
    pub fn new(is_dark: bool) -> Self {
        // Create new window
        let obj: UnitInfoPanel = glib::Object::new();

        obj.set_dark(is_dark);

        obj
    }

    pub fn display_unit_info(&self, unit: &UnitInfo) {
        self.imp().display_unit_info(unit);
    }

    pub fn set_dark(&self, is_dark: bool) {
        self.imp().set_dark(is_dark)
    }
}

mod imp {
    use std::cell::{Cell, RefCell};

    use gtk::{
        glib,
        prelude::*,
        subclass::{
            box_::BoxImpl,
            prelude::*,
            widget::{
                CompositeTemplateCallbacksClass, CompositeTemplateClass,
                CompositeTemplateInitializingExt, WidgetClassExt, WidgetImpl,
            },
        },
        TemplateChild,
    };

    use log::{info, warn};

    use crate::{
        systemd::data::UnitInfo,
        widget::{button_icon::ButtonIcon, info_window::InfoWindow},
    };

    use super::fill_all_info;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/unit_info_panel.ui")]
    pub struct UnitInfoPanelImp {
        #[template_child]
        show_all_button: TemplateChild<ButtonIcon>,

        #[template_child]
        refresh_button: TemplateChild<ButtonIcon>,

        #[template_child]
        unit_info_textview: TemplateChild<gtk::TextView>,

        unit: RefCell<Option<UnitInfo>>,

        is_dark: Cell<bool>,
    }

    #[gtk::template_callbacks]
    impl UnitInfoPanelImp {
        #[template_callback]
        fn refresh_info_clicked(&self, button: &ButtonIcon) {
            info!("button {:?}", button);

            let binding = self.unit.borrow();
            let Some(unit) = binding.as_ref() else {
                warn!("no unit file");
                return;
            };

            self.update_unit_info(&unit)
        }

        #[template_callback]
        fn show_all_clicked(&self, button: &ButtonIcon) {
            info!("button {:?}", button);

            let info_window = InfoWindow::new();

            let binding = self.unit.borrow();
            let Some(unit) = binding.as_ref() else {
                warn!("no unit file");
                return;
            };

            info_window.fill_data(&unit);

            info_window.present();
        }

        pub(crate) fn display_unit_info(&self, unit: &UnitInfo) {
            let _old = self.unit.replace(Some(unit.clone()));

            self.update_unit_info(&unit)
        }

        /// Updates the associated journal `TextView` with the contents of the unit's journal log.
        fn update_unit_info(&self, unit: &UnitInfo) {
            let text = fill_all_info(unit, self.is_dark.get());

            let journal_text: &gtk::TextView = self.unit_info_textview.as_ref();

            let buf = journal_text.buffer();

            buf.set_text(""); // clear text

            let mut start_iter = buf.start_iter();

            buf.insert_markup(&mut start_iter, &text);
        }

        pub(crate) fn set_dark(&self, is_dark: bool) {
            self.is_dark.set(is_dark);
        }
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for UnitInfoPanelImp {
        const NAME: &'static str = "UnitInfoPanel";
        type Type = super::UnitInfoPanel;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            // The layout manager determines how child widgets are laid out.
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for UnitInfoPanelImp {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }
    impl WidgetImpl for UnitInfoPanelImp {}
    impl BoxImpl for UnitInfoPanelImp {}
}

fn fill_all_info(unit: &UnitInfo, is_dark: bool) -> String {
    let mut text = String::new();
    fill_name_description(&mut text, unit);

    let map = match systemd::fetch_system_unit_info_native(&unit) {
        Ok(m) => m,
        Err(e) => {
            error!("Fail to retreive Unit info: {:?}", e);
            HashMap::new()
        }
    };

    fill_description(&mut text, &map);
    fill_load_state(&mut text, &map, is_dark);
    fill_dropin(&mut text, &map);
    fill_active_state(&mut text, &map, is_dark);
    fill_docs(&mut text, &map);
    fill_main_pid(&mut text, &map, unit);
    fill_tasks(&mut text, &map);
    fill_memory(&mut text, &map);
    fill_cpu(&mut text, &map);
    fill_trigger_timers_calendar(&mut text, &map);
    fill_trigger_timers_monotonic(&mut text, &map);
    fill_triggers(&mut text, &map);
    fill_listen(&mut text, &map);
    fill_control_group(&mut text, &map);

    text
}

fn fill_name_description(text: &mut String, unit: &UnitInfo) {
    fill_row(text, "Name:", &unit.primary())
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

macro_rules! strwriter {
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
}

fn write_key(text: &mut String, key_label: &str) {
    strwriter!(text, "{:>KEY_WIDTH$} ", key_label);
}

fn fill_row(text: &mut String, key_label: &str, value: &str) {
    strwriterln!(text, "{:>KEY_WIDTH$} {}", key_label, value);
}

fn fill_dropin(text: &mut String, map: &HashMap<String, OwnedValue>) {
    let value = get_value!(map, "DropInPaths");

    let drop_in_paths = get_array_str(value);

    if drop_in_paths.is_empty() {
        return;
    }

    write_key(text, "Drop in:");

    for s in drop_in_paths {
        let (first, last) = s.rsplit_once('/').unwrap();
        text.push_str(first);
        text.push('\n');

        strwriterln!(text, "{:KEY_WIDTH$} └─{}", " ", last);
    }
}

fn fill_active_state(text: &mut String, map: &HashMap<String, OwnedValue>, is_dark: bool) {
    let value = get_value!(map, "ActiveState");
    let state = value_str(value);

    write_key(text, "Active State:");

    let mut state_text = String::from(state);
    if let Some(substate) = get_substate(map) {
        state_text.push_str(" (");
        state_text.push_str(substate);
        state_text.push(')');
    }

    if state == "active" {
        Token::InfoActive.colorize(&state_text, is_dark, text);
    } else {
        //inactive must be
        text.push_str(&state_text);
    }

    if let Some(since) = add_since(map, state) {
        text.push_str(" since ");
        text.push_str(&since.0);
        text.push_str("; ");
        text.push_str(&since.1);
        text.push_str(" ago");
    }

    strwriterln!(text, "");
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

fn fill_description(text: &mut String, map: &HashMap<String, OwnedValue>) {
    let value = get_value!(map, "Description");
    fill_row(text, "Description:", value_str(value))
}

fn fill_load_state(text: &mut String, map: &HashMap<String, OwnedValue>, is_dark: bool) {
    let value = get_value!(map, "LoadState");

    write_key(text, "Loaded:");

    text.push_str(value_str(value));

    let three_param = [
        map.get("FragmentPath"),
        map.get("UnitFileState"),
        map.get("UnitFilePreset"),
    ];

    let mut all_none = true;
    for p in three_param {
        if !p.is_none() {
            if  let Value::Str(inner_str) = p.unwrap() as &Value {
                if !inner_str.is_empty() {
                    all_none = false;
                    break;
                }
            }
   
        }
    }

    if !all_none {
        text.push_str(" (");

        let [path_op, unit_file_state_op, unit_file_preset_op] = three_param;

        let mut pad_left = false;

        if let Some(path) = path_op {
            text.push_str(value_str(path));
            pad_left = true;
        }

        if let Some(unit_file_state) = unit_file_state_op {
            if pad_left {
                text.push_str("; ");
            }

            write_enabled_state(unit_file_state, is_dark, text);

            pad_left = true;
        }

        if let Some(unit_file_preset) = unit_file_preset_op {
            if pad_left {
                text.push_str("; ");
            }
            text.push_str(" preset: ");
            write_enabled_state(unit_file_preset, is_dark, text);
        }

        text.push(')');
    }

    strwriterln!(text, "");
}

fn write_enabled_state(unit_file_state: &OwnedValue, is_dark: bool, text: &mut String) {
    let state = value_str(unit_file_state);

    match state {
        "enabled" => Token::InfoActive.colorize(state, is_dark, text),
        "disabled" => Token::InfoDisable.colorize(state, is_dark, text),
        _ => text.push_str(value_str(unit_file_state)),
    };
}

fn fill_docs(text: &mut String, map: &HashMap<String, OwnedValue>) {
    let value = get_value!(map, "Documentation");

    let docs = get_array_str(value);

    let mut it = docs.iter();

    if let Some(doc) = it.next() {
        fill_row(text, "Doc:", doc);
    }

    while let Some(doc) = it.next() {
        strwriterln!(text, "{:KEY_WIDTH$} {}", " ", doc);
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
            warn!("Wrong zvalue conversion: {:?}", value.dynamic_signature());
            return Vec::new();
        }
    };
    vec
}

fn fill_memory(text: &mut String, map: &HashMap<String, OwnedValue>) {
    let value = get_value!(map, "MemoryCurrent");

    let memory_current = value_u64(value);
    if memory_current == U64MAX {
        return;
    }

    write_key(text, "Memory:");

    let value_str = &human_bytes(memory_current);

    text.push_str(value_str);

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
        text.push_str(" (");

        let [peak_op, swap_peak_op, swap_op] = three_param;

        let pad_left = write_mem_param(peak_op, "peak: ", false, text);
        write_mem_param(swap_peak_op, "swap: ", pad_left, text);
        write_mem_param(swap_op, "swap peak: ", pad_left, text);

        text.push(')');
    }

    //Memory: 1.9M (peak: 6.2M swap: 224.0K swap peak: 444.0K)

    strwriterln!(text, "");
}

fn write_mem_param(
    mem_op: Option<&OwnedValue>,
    label: &str,
    pad_left: bool,
    text: &mut String,
) -> bool {
    let Some(mem) = mem_op else {
        return false;
    };

    let mem_num = value_u64(mem);
    if mem_num == U64MAX || mem_num == 0{
        return false;
    }

    if pad_left {
        text.push_str(" ");
    }

    text.push_str(label);
    let mem_human = &human_bytes(mem_num);
    text.push_str(mem_human);

    true
}

fn fill_main_pid(text: &mut String, map: &HashMap<String, OwnedValue>, unit: &UnitInfo) {
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
        fill_row(text, "Main PID:", v)
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

fn fill_cpu(text: &mut String, map: &HashMap<String, OwnedValue>) {
    let value = get_value!(map, "CPUUsageNSec");

    let value_u64 = value_u64(value);
    if value_u64 == U64MAX {
        return;
    }

    let value_str = &human_time(value_u64);
    fill_row(text, "CPU:", value_str)
}

fn fill_tasks(text: &mut String, map: &HashMap<String, OwnedValue>) {
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

    fill_row(text, "Tasks:", &tasks_info)
}

fn fill_trigger_timers_calendar(text: &mut String, map: &HashMap<String, OwnedValue>) {
    let value = get_value!(map, "TimersCalendar");

    let Value::Array(array) = value as &Value else {
        return;
    };

    if array.is_empty() {
        return;
    }

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

    let Some(Value::U64(_val_2)) = zstruc.fields().get(2) else {
        return;
    };

    let timers = format!("{} {}", val_0, val_1);

    fill_row(text, "Trigger:", &timers)
}

fn fill_trigger_timers_monotonic(text: &mut String, map: &HashMap<String, OwnedValue>) {
    let value = get_value!(map, "TimersMonotonic");

    let Value::Array(array) = value as &Value else {
        return;
    };

    if array.is_empty() {
        return;
    }

    let timers = value.to_string();

    if timers.is_empty() {
        return;
    }

    fill_row(text, "Trigger:", &timers)
}

fn fill_triggers(text: &mut String, map: &HashMap<String, OwnedValue>) {
    let value = get_value!(map, "Triggers");

    let triggers = get_array_str(value);

    if triggers.is_empty() {
        return;
    }

    //TODO add the active state of the triggers

    fill_row(text, "Triggers:", &triggers.join("\n"))
}

#[derive(Deserialize, Type, PartialEq, Debug)]
struct Struct {
    field1: String,
    field2: String,
}

fn fill_listen(text: &mut String, map: &HashMap<String, OwnedValue>) {
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

    fill_row(text, "Listen:", &listen)
}

fn fill_control_group(text: &mut String, map: &HashMap<String, OwnedValue>) {
    let value = get_value!(map, "ControlGroup");

    let c_group = value_str(value);

    if c_group.is_empty() {
        return;
    }

    const KEY_LABEL: &str = "CGroup:";

    if let Some(exec_full) = get_exec_full(map) {
        let main_pid = get_main_pid(map);

        write_key(text, KEY_LABEL);

        text.push_str(c_group);
        text.push('\n');

        strwriterln!(
            text,
            "{:KEY_WIDTH$} └─{} {}",
            " ",
            &main_pid.to_string(),
            exec_full
        );
    } else {
        fill_row(text, KEY_LABEL, c_group)
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
}
