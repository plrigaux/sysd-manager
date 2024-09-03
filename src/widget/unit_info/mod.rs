use std::collections::HashMap;

use gtk::{prelude::*, Orientation};
use log::{error, warn};
use zvariant::{DynamicType, OwnedValue, Value};

use crate::systemd::{self, data::UnitInfo};

pub fn fill_data(unit: &UnitInfo) -> gtk::Box {
    let info_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(5)
        .build();

    fill_name_description(&info_box, unit);

    let map = match systemd::fetch_system_unit_info_native(&unit) {
        Ok(m) => m,
        Err(e) => {
            error!("Fail to retreive Unit info: {:?}", e);
            HashMap::new()
        }
    };

    fill_dropin(&info_box, &map);
    fill_active_state(&info_box, &map);
    fill_load_state(&info_box, &map);
    fill_main_pid(&info_box, &map, unit);
    fill_memory(&info_box, &map);
    fill_cpu(&info_box, &map);

    info_box
}

fn fill_name_description(info_box: &gtk::Box, unit: &UnitInfo) {
    fill_row(info_box, "Name", &unit.primary());
    fill_row(info_box, "Description", &unit.description());
}

fn fill_row(info_box: &gtk::Box, key: &str, value: &str) {
    let item = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(5)
        .width_request(30)
        .build();

    let key_label = gtk::Label::builder().label(key).width_request(130).build();

    item.append(&key_label);

    item.append(&gtk::Label::new(Some(value)));

    info_box.append(&item);
}

macro_rules! get_value {
    ($map:expr, $key:expr) => {{
        let Some(value) = $map.get($key) else {
            warn!("Key doesn't exists: {:?}", $key);
            return;
        };
        value
    }};
}

fn fill_dropin(info_box: &gtk::Box, map: &HashMap<String, OwnedValue>) {
    /*     let Some(value) = map.get("DropInPaths") else {
        warn!("Key doesn't exists: {:?}", "asdf");
        return;
    }; */

    let value = get_value!(map, "DropInPaths");

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

    if drop_in_paths.is_empty() {
        return;
    }

    fill_row(info_box, "Drop in:", &drop_in_paths.join("\n"));
}

fn fill_active_state(info_box: &gtk::Box, map: &HashMap<String, OwnedValue>) {
    let value = get_value!(map, "ActiveState");
    fill_row(info_box, "Active State:", value_str(value));
}

fn fill_load_state(info_box: &gtk::Box, map: &HashMap<String, OwnedValue>) {
    let value = get_value!(map, "LoadState");
    fill_row(info_box, "Load State:", value_str(value));
}

fn fill_memory(info_box: &gtk::Box, map: &HashMap<String, OwnedValue>) {
    let value = get_value!(map, "MemoryCurrent");

    let memory_current = value_u64(value);
    if memory_current == U64MAX {
        return;
    }

    let value_str = &human_bytes(memory_current);
    fill_row(info_box, "Memory:", value_str);
}

fn fill_main_pid(info_box: &gtk::Box, map: &HashMap<String, OwnedValue>, unit: &UnitInfo) {
    let value = get_value!(map, "MainPID");

    if let zvariant::Value::U32(main_pid) = value as &Value {
        if 0 == *main_pid {
        } else {
            let v = &format!("{} ({})", main_pid, unit.display_name());
            fill_row(info_box, "Main PID:", v);
        }
    }
}

fn fill_cpu(info_box: &gtk::Box, map: &HashMap<String, OwnedValue>) {
    let value = get_value!(map, "CPUUsageNSec");

    let value_u64 = value_u64(value);
    if value_u64 == U64MAX {
        return;
    }

    let value_str = &human_time(value_u64);
    fill_row(info_box, "CPU:", value_str);
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
