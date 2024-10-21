use std::collections::HashMap;

use crate::systemd::{self, data::UnitInfo};
use log::{debug, error, warn};
use serde::Deserialize;
use std::fmt::Write;
use time_handling::get_since_and_passed_time;
use zvariant::{DynamicType, OwnedValue, Type, Value};

mod time_handling;

use gtk::{glib, subclass::prelude::ObjectSubclassIsExt};

// ANCHOR: mod
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
            /*             let text = match systemd::get_unit_journal(unit, in_color) {
                           Ok(journal_output) => journal_output,
                           Err(error) => {
                               let text = match error.gui_description() {
                                   Some(s) => s.clone(),
                                   None => String::from(""),
                               };
                               text
                           }
                       };
            */
            let text = match fill_all_info(unit) {
                Ok(s) => s,
                Err(e) => {
                    warn!("Error {:?}", e);
                    return;
                }
            };

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

/* pub fn fill_data(unit: &UnitInfo) -> gtk::Box {
    let info_box_main = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(5)
        .build();

    let info_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(5)
        .build();

    fill_all_info(&info_box, unit);

    info_box_main.append(&info_box);
    fill_buttons(&info_box_main, &info_box, unit);

    info_box_main
}
 */
fn fill_all_info(unit: &UnitInfo) -> Result<String, Box<dyn std::error::Error>> {
    let mut text = String::new();
    fill_name_description(&mut text, unit)?;

    let map = match systemd::fetch_system_unit_info_native(&unit) {
        Ok(m) => m,
        Err(e) => {
            error!("Fail to retreive Unit info: {:?}", e);
            HashMap::new()
        }
    };

    fill_description(&mut text, &map)?;
    fill_dropin(&mut text, &map)?;
    fill_active_state(&mut text, &map)?;
    fill_load_state(&mut text, &map)?;
    fill_docs(&mut text, &map)?;
    fill_main_pid(&mut text, &map, unit)?;
    fill_tasks(&mut text, &map)?;
    fill_memory(&mut text, &map)?;
    fill_cpu(&mut text, &map)?;
    fill_trigger_timers_calendar(&mut text, &map)?;
    fill_trigger_timers_monotonic(&mut text, &map)?;
    fill_triggers(&mut text, &map)?;
    fill_listen(&mut text, &map)?;
    fill_control_group(&mut text, &map)?;

    Ok(text)
}

fn fill_name_description(
    text: &mut String,
    unit: &UnitInfo,
) -> Result<(), Box<dyn std::error::Error>> {
    fill_row(text, "Name:", &unit.primary())
}

const KEY_WIDTH: usize = 15;

fn fill_key(text: &mut String, key_label: &str) -> Result<(), Box<dyn std::error::Error>> {
    write!(text, "{:>KEY_WIDTH$} ", key_label)?;
    Ok(())
}

fn fill_row(
    text: &mut String,
    key_label: &str,
    value: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    writeln!(text, "{:>KEY_WIDTH$} {}", key_label, value)?;
    Ok(())
}

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

fn fill_dropin(
    text: &mut String,
    map: &HashMap<String, OwnedValue>,
) -> Result<(), Box<dyn std::error::Error>> {
    let value = get_value!(map, "DropInPaths", Ok(()));

    let drop_in_paths = get_array_str(value);

    if drop_in_paths.is_empty() {
        return Ok(());
    }

    fill_key(text, "Drop in:")?;

    for s in drop_in_paths {
        let (first, last) = s.rsplit_once('/').unwrap();
        text.push_str(first);
        text.push('\n');

        writeln!(text, "{:KEY_WIDTH$} └─{}", " ", last)?;
    }
    Ok(())
}

fn fill_active_state(
    text: &mut String,
    map: &HashMap<String, OwnedValue>,
) -> Result<(), Box<dyn std::error::Error>> {
    let value = get_value!(map, "ActiveState", Ok(()));
    let state = value_str(value);

    let mut state_line = String::from(state);

    if let Some(substate) = get_substate(map) {
        state_line.push_str(" (");
        state_line.push_str(substate);
        state_line.push(')');
    }

    if let Some(since) = add_since(map, state) {
        state_line.push_str(" since ");
        state_line.push_str(&since.0);
        state_line.push_str("; ");
        state_line.push_str(&since.1);
        state_line.push_str(" ago");
    }

    fill_row(text, "Active State:", &state_line)
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
    text: &mut String,
    map: &HashMap<String, OwnedValue>,
) -> Result<(), Box<dyn std::error::Error>> {
    let value = get_value!(map, "Description", Ok(()));
    fill_row(text, "Description:", value_str(value))
}

fn fill_load_state(
    text: &mut String,
    map: &HashMap<String, OwnedValue>,
) -> Result<(), Box<dyn std::error::Error>> {
    let value = get_value!(map, "LoadState", Ok(()));
    fill_row(text, "Load State:", value_str(value))
}

fn fill_docs(
    text: &mut String,
    map: &HashMap<String, OwnedValue>,
) -> Result<(), Box<dyn std::error::Error>> {
    let value = get_value!(map, "Documentation", Ok(()));

    let docs = get_array_str(value);

    if docs.is_empty() {
        return Ok(());
    }

    fill_row(text, "Doc:", &docs.join("\n"))
}

fn get_array_str<'a>(value: &'a zvariant::Value<'a>) -> Vec<&'a str> {
    let vec = match value as &zvariant::Value {
        zvariant::Value::Array(a) => {
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

fn fill_memory(
    text: &mut String,
    map: &HashMap<String, OwnedValue>,
) -> Result<(), Box<dyn std::error::Error>> {
    let value = get_value!(map, "MemoryCurrent", Ok(()));

    let memory_current = value_u64(value);
    if memory_current == U64MAX {
        return Ok(());
    }

    let value_str = &human_bytes(memory_current);
    fill_row(text, "Memory:", value_str)
}

fn fill_main_pid(
    text: &mut String,
    map: &HashMap<String, OwnedValue>,
    unit: &UnitInfo,
) -> Result<(), Box<dyn std::error::Error>> {
    let main_pid = get_main_pid(map);

    if 0 == main_pid {
        Ok(())
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

    if let zvariant::Value::U32(main_pid) = value as &Value {
        return *main_pid;
    }
    0
}

fn get_exec_full<'a>(map: &'a HashMap<String, OwnedValue>) -> Option<&'a str> {
    let value = get_value!(map, "ExecStart", None);

    if let zvariant::Value::Array(array) = value as &Value {
        if let Ok(Some(owned_value)) = array.get::<&Value>(0) {
            if let zvariant::Value::Structure(zstruc) = owned_value {
                if let Some(val_0) = zstruc.fields().get(0) {
                    if let zvariant::Value::Str(zstr) = val_0 {
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

fn fill_cpu(
    text: &mut String,
    map: &HashMap<String, OwnedValue>,
) -> Result<(), Box<dyn std::error::Error>> {
    let value = get_value!(map, "CPUUsageNSec", Ok(()));

    let value_u64 = value_u64(value);
    if value_u64 == U64MAX {
        return Ok(());
    }

    let value_str = &human_time(value_u64);
    fill_row(text, "CPU:", value_str)
}

fn fill_tasks(
    text: &mut String,
    map: &HashMap<String, OwnedValue>,
) -> Result<(), Box<dyn std::error::Error>> {
    let value = get_value!(map, "TasksCurrent", Ok(()));

    let value_nb = value_u64(value);

    if value_nb == U64MAX {
        return Ok(());
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

fn fill_trigger_timers_calendar(
    text: &mut String,
    map: &HashMap<String, OwnedValue>,
) -> Result<(), Box<dyn std::error::Error>> {
    let value = get_value!(map, "TimersCalendar", Ok(()));

    let zvariant::Value::Array(array) = value as &Value else {
        return Ok(());
    };

    if array.is_empty() {
        return Ok(());
    }

    let Ok(Some(val_listen_stc)) = array.get::<&Value>(0) else {
        return Ok(());
    };

    let zvariant::Value::Structure(zstruc) = val_listen_stc else {
        return Ok(());
    };

    let Some(zvariant::Value::Str(val_0)) = zstruc.fields().get(0) else {
        return Ok(());
    };

    let Some(zvariant::Value::Str(val_1)) = zstruc.fields().get(1) else {
        return Ok(());
    };

    let Some(zvariant::Value::U64(_val_2)) = zstruc.fields().get(2) else {
        return Ok(());
    };

    let timers = format!("{} {}", val_0, val_1);

    fill_row(text, "Trigger:", &timers)
}

fn fill_trigger_timers_monotonic(
    text: &mut String,
    map: &HashMap<String, OwnedValue>,
) -> Result<(), Box<dyn std::error::Error>> {
    let value = get_value!(map, "TimersMonotonic", Ok(()));

    let zvariant::Value::Array(array) = value as &Value else {
        return Ok(());
    };

    if array.is_empty() {
        return Ok(());
    }

    let timers = value.to_string();

    if timers.is_empty() {
        return Ok(());
    }

    fill_row(text, "Trigger:", &timers)
}

fn fill_triggers(
    text: &mut String,
    map: &HashMap<String, OwnedValue>,
) -> Result<(), Box<dyn std::error::Error>> {
    let value = get_value!(map, "Triggers", Ok(()));

    let triggers = get_array_str(value);

    if triggers.is_empty() {
        return Ok(());
    }

    //TODO add the active state of the triggers

    fill_row(text, "Triggers:", &triggers.join("\n"))
}

#[derive(Deserialize, Type, PartialEq, Debug)]
struct Struct {
    field1: String,
    field2: String,
}

fn fill_listen(
    text: &mut String,
    map: &HashMap<String, OwnedValue>,
) -> Result<(), Box<dyn std::error::Error>> {
    let value = get_value!(map, "Listen", Ok(()));

    let zvariant::Value::Array(array) = value as &Value else {
        return Ok(());
    };

    let Ok(Some(val_listen_stc)) = array.get::<&Value>(0) else {
        return Ok(());
    };

    let zvariant::Value::Structure(zstruc) = val_listen_stc else {
        return Ok(());
    };

    let Some(zvariant::Value::Str(val_0)) = zstruc.fields().get(0) else {
        return Ok(());
    };

    let Some(zvariant::Value::Str(val_1)) = zstruc.fields().get(1) else {
        return Ok(());
    };

    let listen = format!("{} ({})", val_1, val_0);

    fill_row(text, "Listen:", &listen)
}

fn fill_control_group(
    text: &mut String,
    map: &HashMap<String, OwnedValue>,
) -> Result<(), Box<dyn std::error::Error>> {
    let value = get_value!(map, "ControlGroup", Ok(()));

    let c_group = value_str(value);

    if c_group.is_empty() {
        return Ok(());
    }

    const KEY_LABEL: &str = "CGroup:";

    if let Some(exec_full) = get_exec_full(map) {
        let main_pid = get_main_pid(map);

        fill_key(text, KEY_LABEL)?;

        text.push_str(c_group);
        text.push('\n');

        writeln!(
            text,
            "{:KEY_WIDTH$} └─{} {}",
            " ",
            &main_pid.to_string(),
            exec_full
        )?;

        Ok(())
    } else {
        fill_row(text, KEY_LABEL, c_group)
    }
}

fn value_str<'a>(value: &'a Value<'a>) -> &'a str {
    if let zvariant::Value::Str(converted) = value as &Value {
        return converted.as_str();
    }
    warn!("Wrong zvalue conversion: {:?}", value);
    ""
}

/// 2^16-1
const U64MAX: u64 = 18_446_744_073_709_551_615;
const SUFFIX: [&str; 9] = ["B", "K", "M", "G", "T", "P", "E", "Z", "Y"];
const UNIT: f64 = 1024.0;

fn value_u64(value: &Value) -> u64 {
    if let zvariant::Value::U64(converted) = value {
        return *converted;
    }
    warn!("Wrong zvalue conversion: {:?}", value);
    U64MAX
}

/// Converts bytes to human-readable values
fn human_bytes(bytes: u64) -> String {
    // let size: f64 = *bytes as f64;

    if bytes <= 0 {
        return "0 B".to_string();
    }

    let base = (bytes as f64).log10() / UNIT.log10();

    let mut result: String = format!("{:.1}", UNIT.powf(base - base.floor()))
        .trim_end_matches(".0")
        .to_string();

    result.push_str(" ");
    result.push_str(SUFFIX[base.floor() as usize]);

    result
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
