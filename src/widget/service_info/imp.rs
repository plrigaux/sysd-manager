use gtk::{glib, prelude::*, subclass::prelude::*};
use log::{error, warn};
use zvariant::{DynamicType, Value};

use crate::{
    systemd::{self, data::UnitInfo},
    systemd_gui,
    widget::info_window::InfoWindow,
};

#[derive(Debug, Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/service_status.ui")]
pub struct ServiceStatusImp {
    #[template_child]
    pub name_description: TemplateChild<gtk::Label>,

    #[template_child]
    pub info_loaded: TemplateChild<gtk::Label>,

    #[template_child]
    pub info_dropin: TemplateChild<gtk::Label>,

    #[template_child]
    pub info_active: TemplateChild<gtk::Label>,

    #[template_child]
    pub info_mainpid: TemplateChild<gtk::Label>,

    #[template_child]
    pub info_tasks: TemplateChild<gtk::Label>,

    #[template_child]
    pub info_memory: TemplateChild<gtk::Label>,

    #[template_child]
    pub info_cpu: TemplateChild<gtk::Label>,

    #[template_child]
    pub info_cgroup: TemplateChild<gtk::Label>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for ServiceStatusImp {
    const NAME: &'static str = "ServiceStatus";
    type Type = super::ServiceStatus;
    type ParentType = gtk::Grid;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
        klass.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

#[gtk::template_callbacks]
impl ServiceStatusImp {
    #[template_callback(name = "handle_refresh_click")]
    fn handle_refresh_click(&self, _button: &gtk::Button) {
        systemd_gui::selected_unit(|unit: &UnitInfo| self.fill_data(unit));
    }

    #[template_callback]
    fn handle_all_details_click(_button: &gtk::Button) {
        systemd_gui::selected_unit(|unit: &UnitInfo| {
            let info_window = InfoWindow::new();

            info_window.fill_data(&unit);

            info_window.present();
        });
    }

    pub(super) fn fill_data(&self, unit: &UnitInfo) {
        self.name_description
            .set_label(&format!("{} - {}", unit.primary(), unit.description()));

        let map = match systemd::fetch_system_unit_info_native(&unit) {
            Ok(m) => m,
            Err(e) => {
                error!("Fail to retreive Unit info: {:?}", e);
                return;
            }
        };

        if let Some(value) = map.get("DropInPaths") {
            let drop_in_paths = match value as &zvariant::Value {
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
                    vec![]
                }
            };

            self.info_loaded.set_label(&drop_in_paths.join("\n"));
        }

        if let Some(active_state) = map.get("ActiveState") {
            self.info_active.set_label(value_str(active_state));
        }

        if let Some(load_state) = map.get("LoadState") {
            self.info_dropin.set_label(value_str(load_state));
        }

        if let Some(value) = map.get("MainPID") {
            if let zvariant::Value::U32(main_pid) = value as &Value {
                if 0 == *main_pid {
                    self.info_mainpid.set_label(" - ");
                } else {
                    self.info_mainpid
                        .set_label(&format!("{} ({})", main_pid, unit.display_name()));
                }
            }
        }

        if let Some(value) = map.get("MemoryCurrent") {
            let memory_current = value_u64(value);

            let mem = if memory_current == U64MAX {
                ""
            } else {
                &human_bytes(memory_current)
            };

            self.info_memory.set_label(mem);
        }

        if let Some(value) = map.get("CPUUsageNSec") {
            let cpu_usage = value_u64(value);
            let cpu = if cpu_usage == U64MAX {
                ""
            } else {
                &human_time(cpu_usage)
            };
            self.info_cpu.set_label(cpu);
        }
    }

    //TODO Documentation
}

impl ObjectImpl for ServiceStatusImp {}
impl WidgetImpl for ServiceStatusImp {}
impl GridImpl for ServiceStatusImp {}

/// 2^16-1
const U64MAX: u64 = 18_446_744_073_709_551_615;
const SUFFIX: [&str; 9] = ["B", "K", "M", "G", "T", "P", "E", "Z", "Y"];
const UNIT: f64 = 1024.0;

fn value_str<'a>(value: &'a Value<'a>) -> &'a str {
    if let zvariant::Value::Str(converted) = value as &Value {
        return converted.as_str();
    }
    warn!("Wrong zvalue conversion: {:?}", value);
    ""
}

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
