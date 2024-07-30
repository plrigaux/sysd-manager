pub extern crate dbus;

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::OnceLock;

use log::debug;
use log::trace;

use dbus::arg::messageitem::MessageItem;
use dbus::arg::messageitem::Props;
use dbus::Message;
use zvariant::OwnedObjectPath;

use crate::systemd::data::UnitInfo;
use crate::systemd::enums::ActiveState;
use crate::systemd::enums::UnitType;

use super::enums::EnablementStatus;

use super::SystemdErrors;
use super::SystemdUnit;

const DESTINATION_SYSTEMD: &str = "org.freedesktop.systemd1";
const INTERFACE_SYSTEMD_UNIT: &str = "org.freedesktop.systemd1.Unit";
const INTERFACE_SYSTEMD_MANAGER: &str = "org.freedesktop.systemd1.Manager";
const PATH_SYSTEMD: &str = "/org/freedesktop/systemd1";

use zbus::{connection, interface};
use zvariant::ObjectPath;

/// Takes a systemd dbus function as input and returns the result as a `dbus::Message`.
fn dbus_message(function: &str) -> Result<Message, SystemdErrors> {
    let dest = DESTINATION_SYSTEMD;
    let path = PATH_SYSTEMD;
    let interface = INTERFACE_SYSTEMD_MANAGER;
    match dbus::Message::new_method_call(dest, path, interface, function) {
        Ok(message) => Ok(message),
        Err(error) => Err(SystemdErrors::DBusErrorStr(error)),
    }
}

/// Takes a `dbus::Message` as input and makes a connection to dbus, returning the reply.
fn dbus_connect(message: Message) -> Result<Message, SystemdErrors> {
    let connection = dbus::ffidisp::Connection::get_private(dbus::ffidisp::BusType::System)?;

    let message = connection.send_with_reply_and_block(message, 30000)?;

    Ok(message)
}

/* /// Takes a `dbus::Message` as input and makes a connection to dbus, returning the reply.
fn dbus_property() -> Result<(), Error> {
    let connection = dbus::blocking::Connection::new_system()?;

    let duration = Duration::from_secs(5);
    let p = connection.with_proxy(SYSTEMD_DESTINATION, "/org/freedesktop/systemd1", duration);


    // The Metadata property is a Dict<String, Variant>.

    // Option 1: we can get the dict straight into a hashmap, like this:
    use systemd::dbus::msgbus::blocking::stdintf::org_freedesktop_dbus::Properties;

    let metadata = p.get("org.mpris.MediaPlayer2.Player", "Metadata")?;

    debug!("Option 1:");

    Ok(())

} */

/// Takes the dbus message as input and maps the information to a `Vec<SystemdUnit>`.
fn parse_message(message_item: &MessageItem) -> Result<Vec<SystemdUnit>, SystemdErrors> {
    debug!("parse_message");

    let MessageItem::Array(array) = message_item else {
        return Err(SystemdErrors::MalformedWrongArgType(
            message_item.arg_type(),
        ));
    };

    let mut systemd_units: Vec<SystemdUnit> = Vec::with_capacity(array.len());

    for service_struct in array.into_iter() {
        let MessageItem::Struct(struct_value) = service_struct else {
            return Err(SystemdErrors::MalformedWrongArgType(
                service_struct.arg_type(),
            ));
        };

        if struct_value.len() >= 2 {
            let Some(MessageItem::Str(systemd_unit)) = struct_value.get(0) else {
                return Err(SystemdErrors::Malformed);
            };

            let Some(MessageItem::Str(status)) = struct_value.get(1) else {
                return Err(SystemdErrors::Malformed);
            };

            let Some((_prefix, name_type)) = systemd_unit.rsplit_once('/') else {
                return Err(SystemdErrors::Malformed);
            };

            let Some((name, system_type)) = name_type.rsplit_once('.') else {
                return Err(SystemdErrors::Malformed);
            };

            let status_code = EnablementStatus::new(&status);
            let utype = UnitType::new(system_type);

            let path = systemd_unit.to_owned();

            systemd_units.push(SystemdUnit {
                name: name.to_owned(),
                status_code,
                utype,
                path: path,
            });
        }
    }

    Ok(systemd_units)
}

/// Communicates with dbus to obtain a list of unit files and returns them as a `Vec<SystemdUnit>`.
pub fn list_unit_files() -> Result<Vec<SystemdUnit>, SystemdErrors> {
    let message_vec = list_unit_files_message()?;

    trace!("MESSAGE {:?}", message_vec);

    let message_item = if message_vec.len() >= 1 {
        message_vec.get(0).expect("Missing argument")
    } else {
        panic!("Always suppose have one item")
    };

    let units = parse_message(message_item)?;

    Ok(units)
}

fn list_unit_files_message() -> Result<Vec<MessageItem>, SystemdErrors> {
    let message = dbus_message("ListUnitFiles")?;
    let m = dbus_connect(message)?;
    trace!("MESSAGE {:?}", m);
    Ok(m.get_items())
}
use serde::{Deserialize, Serialize};
use zvariant::Type;
#[derive(Deserialize, Serialize, Type, PartialEq, Debug)]
struct LUnit {
    s1: String,
    s2: String,
    s3: String,
    s4: String,
    s5: String,
    s6: String,
    o1: OwnedObjectPath,
    u1: u32,
    s7: String,
    o2: OwnedObjectPath,
}

const METHOD_LIST_UNIT: &str = "ListUnits";
fn try_zbus() -> Result<BTreeMap<String, UnitInfo>, SystemdErrors> {
    let connection = zbus::blocking::Connection::session()?;

    let message = connection.call_method(
        Some(DESTINATION_SYSTEMD),
        PATH_SYSTEMD,
        Some(INTERFACE_SYSTEMD_MANAGER),
        METHOD_LIST_UNIT,
        &(),
    )?;

    let body = message.body();

    //"a(ssssssouso)"

    //println!("header: {:#?}", message.header());

    let out: Vec<LUnit> = body.deserialize()?;

    println!("out: {:#?}", out);

    let mut map: BTreeMap<String, UnitInfo> = BTreeMap::new();

    Ok(map)
}

fn list_units_description() -> Result<BTreeMap<String, UnitInfo>, SystemdErrors> {
    match try_zbus() {
        Ok(_) => println!("Ok"),
        Err(e) => println!("Error: {:#?}", e),
    };
    let message = dbus_message(METHOD_LIST_UNIT)?;
    debug!("MESSAGE {:?}", message);
    let msg2 = dbus_connect(message)?;

    // debug!("{:#?}",m.get_items())
    let mi = msg2.get_items();
    debug!("{:#?}", mi.len());
    let message_item = &mi[0];

    debug!("{:#?}", message_item.signature());

    let MessageItem::Array(array) = message_item else {
        return Err(SystemdErrors::MalformedWrongArgType(
            message_item.arg_type(),
        ));
    };
    debug!("Array_size {:#?}", array.len());

    let mut map: BTreeMap<String, UnitInfo> = BTreeMap::new();

    for service_struct in array.iter() {
        let MessageItem::Struct(struct_value) = service_struct else {
            return Err(SystemdErrors::MalformedWrongArgType(
                service_struct.arg_type(),
            ));
        };

        //The primary unit name as string
        let MessageItem::Str(ref primary) = struct_value[0] else {
            return Err(SystemdErrors::MalformedWrongArgType(
                service_struct.arg_type(),
            ));
        };

        //The human readable description string
        let MessageItem::Str(ref description) = struct_value[1] else {
            return Err(SystemdErrors::MalformedWrongArgType(
                service_struct.arg_type(),
            ));
        };

        //The load state (i.e. whether the unit file has been loaded successfully)
        let MessageItem::Str(ref load_state) = struct_value[2] else {
            return Err(SystemdErrors::MalformedWrongArgType(
                service_struct.arg_type(),
            ));
        };

        //The active state (i.e. whether the unit is currently started or not)
        let MessageItem::Str(ref active_state_str) = struct_value[3] else {
            return Err(SystemdErrors::MalformedWrongArgType(
                service_struct.arg_type(),
            ));
        };
        //The sub state (a more fine-grained version of the active state that is specific to the unit type, which the active state is not)
        let MessageItem::Str(ref sub_state) = struct_value[4] else {
            return Err(SystemdErrors::MalformedWrongArgType(
                service_struct.arg_type(),
            ));
        };
        //A unit that is being followed in its state by this unit, if there is any, otherwise the empty string.
        let MessageItem::Str(ref followed_unit) = struct_value[5] else {
            return Err(SystemdErrors::MalformedWrongArgType(
                service_struct.arg_type(),
            ));
        };

        //The unit object path
        let MessageItem::ObjectPath(ref object_path) = struct_value[6] else {
            return Err(SystemdErrors::MalformedWrongArgType(
                service_struct.arg_type(),
            ));
        };
        /*                 //If there is a job queued for the job unit the numeric job id, 0 otherwise
        let MessageItem::UInt32(job_id) = struct_value[7] else {
            debug!("7 {:?}", struct_value[7]);
            continue;
        };
        //The job type as string
        let MessageItem::Str(ref job_type) = struct_value[8] else {
            continue;
        };
        //The job object path
        let MessageItem::ObjectPath(ref job_object_path) = struct_value[9] else {
            continue;
        }; */

        let active_state = ActiveState::from_str(active_state_str);

        let unit = UnitInfo::new(
            primary,
            description,
            load_state,
            active_state,
            sub_state,
            followed_unit,
            object_path.to_string(),
        );

        map.insert(primary.to_ascii_lowercase(), unit);
    }
    Ok(map)
}

/// Returns the current enablement status of the unit
pub fn get_unit_file_state_path(unit_file: &str) -> Result<EnablementStatus, SystemdErrors> {
    let mut message = dbus_message("GetUnitFileState")?;
    let message_items = &[MessageItem::Str(unit_file.to_owned())];
    message.append_items(message_items);

    let message_wraper = dbus_connect(message)?;

    if let Some(enablement_status) = message_wraper.get1::<String>() {
        Ok(EnablementStatus::new(&enablement_status))
    } else {
        Err(SystemdErrors::Malformed)
    }
}

pub fn list_units_description_and_state() -> Result<BTreeMap<String, UnitInfo>, SystemdErrors> {
    let mut units_map = list_units_description()?;

    let mut unit_files = list_unit_files()?;

    for unit_file in unit_files.drain(..) {
        match units_map.get_mut(&unit_file.full_name().to_ascii_lowercase()) {
            Some(unit_info) => {
                unit_info.set_file_path(unit_file.path);
                unit_info.set_enable_status(unit_file.status_code.to_string());
            }
            None => debug!(
                "Unit \"{}\" status \"{}\" not loaded!",
                unit_file.full_name(),
                unit_file.status_code.to_string()
            ),
        }
    }

    Ok(units_map)
}

/// Takes a unit name as input and attempts to start it
///
pub fn start_unit(unit: &str) -> Result<(), SystemdErrors> {
    let mut message = dbus_message("StartUnit")?;
    message.append_items(&[unit.into(), "fail".into()]);

    let message = dbus_connect(message)?;

    debug!("StartUnit answer: {:?}", message); //TODO return the msg

    Ok(())
}

/// Takes a unit name as input and attempts to stop it.
pub fn stop_unit(unit: &str) -> Result<(), SystemdErrors> {
    let mut message = dbus_message("StopUnit")?;
    message.append_items(&[unit.into(), "fail".into()]);
    let message = dbus_connect(message)?;

    debug!("StartUnit answer: {:?}", message); //TODO return the msg

    Ok(())
}

/// Enqeues a start job, and possibly depending jobs.
pub fn restart_unit(unit: &str) -> Result<(), SystemdErrors> {
    let mut message = dbus_message("RestartUnit")?;
    message.append_items(&[unit.into(), "fail".into()]);

    let message = dbus_connect(message)?;

    debug!("RestartUnit answer: {:?}", message); //TODO return the msg

    Ok(())
}

fn display_message_item(m_item: &MessageItem) -> String {
    let str_value: String = match m_item {
        MessageItem::Array(a) => {
            let mut d_str = String::from("[ ");

            let mut it = a.iter().peekable();
            while let Some(mi) = it.next() {
                d_str.push_str(&display_message_item(mi));
                if it.peek().is_some() {
                    d_str.push_str(", ");
                }
            }

            d_str.push_str(" ]");
            d_str
        }
        MessageItem::Struct(stc) => {
            let mut d_str = String::from("{ ");

            let mut it = stc.iter().peekable();
            while let Some(mi) = it.next() {
                d_str.push_str(&display_message_item(mi));
                if it.peek().is_some() {
                    d_str.push_str(", ");
                }
            }

            d_str.push_str(" }");
            d_str
        }
        MessageItem::Variant(v) => display_message_item(v.peel()),
        MessageItem::Dict(d) => {
            let mut d_str = String::from("{ ");
            for (mik, miv) in d.into_iter() {
                d_str.push_str(&display_message_item(mik));
                d_str.push_str(" : ");
                d_str.push_str(&display_message_item(miv));
            }
            d_str.push_str(" }");
            d_str
        }
        MessageItem::ObjectPath(p) => p.to_string(),
        MessageItem::Signature(s) => format!("{:?}", s),
        MessageItem::Str(s) => s.to_owned(),
        MessageItem::Bool(b) => b.to_string(),
        MessageItem::Byte(b) => b.to_string(),
        MessageItem::Int16(i) => i.to_string(),
        MessageItem::Int32(i) => i.to_string(),
        MessageItem::Int64(i) => i.to_string(),
        MessageItem::UInt16(i) => i.to_string(),
        MessageItem::UInt32(i) => i.to_string(),
        MessageItem::UInt64(i) => i.to_string(),
        MessageItem::Double(i) => i.to_string(),
        MessageItem::UnixFd(i) => format!("{:?}", i),
    };
    str_value
}

pub fn fetch_system_info() -> Result<BTreeMap<String, String>, SystemdErrors> {
    let c = dbus::ffidisp::Connection::new_system().unwrap();

    let dest = DESTINATION_SYSTEMD;
    let path = PATH_SYSTEMD;
    let interface = INTERFACE_SYSTEMD_MANAGER;
    let prop = Props::new(&c, dest, path, interface, 10000);

    let all_items = prop.get_all()?;
    let mut map = BTreeMap::new();

    for (key, b) in all_items.iter() {
        let str_val = display_message_item(b);
        // info!("prop : {} \t value: {}", a, str_val);

        map.insert(key.to_owned(), str_val);
    }
    Ok(map)
}

pub fn fetch_system_unit_info(path: &str) -> Result<BTreeMap<String, String>, SystemdErrors> {
    let dest = DESTINATION_SYSTEMD;
    let interface = INTERFACE_SYSTEMD_UNIT;
    let c = dbus::ffidisp::Connection::new_system()?;
    let prop = Props::new(&c, dest, path, interface, 10000);

    let mut map = BTreeMap::new();

    for (key, b) in prop.get_all()?.iter() {
        let str_val = display_message_item(b);
        // info!("prop : {} \t value: {}", a, str_val);

        map.insert(key.to_owned(), str_val);
    }
    Ok(map)
}

#[cfg(test)]
mod tests {

    use std::collections::HashSet;

    /* use crate::systemd::collect_togglable_services; */

    use super::dbus::arg::messageitem::Props;

    use super::*;

    pub const TEST_SERVICE: &str = "jackett.service";
    use log::*;

    fn init() {
        let _ = env_logger::builder()
            .target(env_logger::Target::Stdout)
            .filter_level(log::LevelFilter::Trace)
            .is_test(true)
            .try_init();
    }

    #[test]
    fn list_unit_files_message_test() -> Result<(), SystemdErrors> {
        let message_vec = list_unit_files_message()?;
        //debug!("{:?}", message);

        let message_item = if message_vec.len() >= 1 {
            message_vec.get(0).expect("Missing argument")
        } else {
            panic!("Aways suppose have one item")
        };

        handle_message_item(message_item);
        Ok(())
    }

    fn handle_message_item(message_item: &MessageItem) {
        match message_item {
            MessageItem::Array(array) => {
                //let _ = array.into_iter().map(|item| debug!("{:?}", item));
                for (i, n) in array.into_iter().enumerate() {
                    debug!("{} - {:?}", i, n);
                    handle_message_item(n);
                }
            }
            MessageItem::Struct(struct_) => {
                for a in struct_.into_iter() {
                    //debug!("{} - {:?}", i , n);
                    handle_message_item(a);
                }
            }
            MessageItem::Variant(_) => todo!(),
            MessageItem::Dict(_) => todo!(),
            MessageItem::ObjectPath(_) => todo!(),
            MessageItem::Signature(_) => todo!(),
            MessageItem::Str(_str_value) => {
                //debug!", str_value );
            }
            MessageItem::Bool(_) => todo!(),
            MessageItem::Byte(_) => todo!(),
            MessageItem::Int16(_) => todo!(),
            MessageItem::Int32(_) => todo!(),
            MessageItem::Int64(_) => todo!(),
            MessageItem::UInt16(_) => todo!(),
            MessageItem::UInt32(_) => todo!(),
            MessageItem::UInt64(_) => todo!(),
            MessageItem::Double(_) => todo!(),
            MessageItem::UnixFd(_) => todo!(),
        }
    }

    #[test]
    fn list_unit_files_message_test2() -> Result<(), SystemdErrors> {
        let message_vec = list_unit_files_message()?;
        //debug!("{:?}", message);

        let message_item = if message_vec.len() >= 1 {
            message_vec.get(0).expect("Missing argument")
        } else {
            panic!("Aways suppose have one item")
        };

        let vector = parse_message(message_item)?;

        debug!("{:#?}", vector);
        Ok(())
    }

    #[test]
    fn stop_service_test() -> Result<(), SystemdErrors> {
        stop_unit(TEST_SERVICE)?;
        Ok(())
    }

    #[test]
    fn dbus_test() -> Result<(), SystemdErrors> {
        // let file: &str = "/etc/systemd/system/jackett.service";
        let file1: &str = TEST_SERVICE;
        let mut message = dbus_message("GetUnitFileState")?;

        let message_items = &[MessageItem::Str(file1.to_owned())];
        message.append_items(message_items);

        match dbus_connect(message) {
            Ok(m) => {
                debug!("{:?}", m.get1::<String>());
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    #[test]
    fn test_get_unit_file_state() {
        // let file: &str = "/etc/systemd/system/jackett.service";
        let file1: &str = TEST_SERVICE;

        let status = get_unit_file_state_path(file1);
        debug!("Status: {:?}", status);
    }

    #[test]
    fn test_list_unit_files() -> Result<(), SystemdErrors> {
        let units = list_unit_files()?;

        let serv = units
            .iter()
            .filter(|ud| ud.full_name() == TEST_SERVICE)
            .nth(0);

        debug!("{:#?}", serv);
        Ok(())
    }

    #[test]
    fn test_list_units() -> Result<(), SystemdErrors> {
        let units = list_units_description()?;

        let serv = units.get(TEST_SERVICE);
        debug!("{:#?}", serv);
        Ok(())
    }

    #[test]
    fn test_list_units_merge() -> Result<(), SystemdErrors> {
        let mut units_map = list_units_description()?;

        let mut units = list_unit_files()?;

        let mut set: HashSet<String> = HashSet::new();
        for unit_file in units.drain(..) {
            match units_map.get_mut(&unit_file.full_name().to_ascii_lowercase()) {
                Some(unit_info) => {
                    unit_info.set_file_path(unit_file.path);
                    unit_info.set_enable_status(unit_file.status_code.to_string());
                    unit_info.set_enable_status(unit_file.status_code.to_string());
                }
                None => debug!("unit \"{}\" not found!", unit_file.full_name()),
            }
        }

        debug!("{:#?}", units_map.get(TEST_SERVICE));

        for unit in units_map.values() {
            set.insert(unit.unit_type().to_owned());
        }

        debug!("Unit types {:#?}", set);

        Ok(())
    }

    /*  #[test]
        fn test_list_units_description_and_state() -> Result<(), SystemdErrors> {
           let units_map = list_units_description_and_state()?;

           let ts = units_map.get(TEST_SERVICE);
           debug!("Test Service {:#?}", ts);
           let units = units_map.into_values().collect::<Vec<LoadedUnit>>();

           let services = collect_togglable_services(&units);

           debug!("service.len {}", services.len());

           Ok(())
       }
    */

    #[test]
    fn test_prop() {
        init();
        let c = dbus::ffidisp::Connection::new_system().unwrap();
        let p = Props::new(
            &c,
            "org.freedesktop.PolicyKit1",
            "/org/freedesktop/PolicyKit1/Authority",
            "org.freedesktop.PolicyKit1.Authority",
            10000,
        );
        info!("BackendVersion: {:?}", p.get("BackendVersion").unwrap())
    }

    #[test]
    fn test_color() {
        init();

        let name = "org.freedesktop.portal.Desktop";
        let path = "/org/freedesktop/portal/desktop";
        let interface = "org.freedesktop.portal.Settings";
        let c = dbus::ffidisp::Connection::new_system().unwrap();

        let prop = Props::new(&c, name, path, interface, 10000);

        let all_items = prop.get_all().unwrap();
        info!("Systemd: {:#?}", all_items);

        for (a, b) in all_items.iter() {
            let str_val = display_message_item(b);
            info!("prop : {} \t value: {}", a, str_val);
        }

        /*     let p = Props::new(
            &c,
            "org.freedesktop.PolicyKit1",
            "/org/freedesktop/PolicyKit1/Authority",
            "org.freedesktop.PolicyKit1.Authority",
            10000,
        ); */
        /*         "Read",
        &("org.freedesktop.appearance", "color-scheme"),

        let c = dbus::ffidisp::Connection::new_system().unwrap();
        let p = Props::new(
            &c,
            "org.freedesktop.PolicyKit1",
            "/org/freedesktop/PolicyKit1/Authority",
            "org.freedesktop.PolicyKit1.Authority",
            10000,
        );
        info!("BackendVersion: {:?}", p.get("BackendVersion").unwrap()) */
    }

    #[test]
    fn test_prop_all_systemd_manager() -> Result<(), SystemdErrors> {
        init();
        let c = dbus::ffidisp::Connection::new_system().unwrap();

        let dest = DESTINATION_SYSTEMD;
        let path = PATH_SYSTEMD;
        let interface = INTERFACE_SYSTEMD_MANAGER;
        let prop = Props::new(&c, dest, path, interface, 10000);

        let all_items = prop.get_all()?;
        log::info!("Systemd: {:#?}", all_items);

        for (a, b) in all_items.iter() {
            let str_val = display_message_item(b);
            log::info!("prop : {} \t value: {}", a, str_val);
        }

        Ok(())
    }

    #[test]
    fn test_prop2() {
        init();
        let c = dbus::ffidisp::Connection::new_system().unwrap();

        let dest = DESTINATION_SYSTEMD;
        let path = "/org/freedesktop/systemd1";
        let interface = "org.freedesktop.systemd1.Manager";
        let prop = Props::new(&c, dest, path, interface, 10000);
        debug!("Version: {:?}", prop.get("Version").unwrap());
        debug!("Architecture: {:?}", prop.get("Architecture").unwrap());
    }

    #[test]
    fn test_prop33() {
        init();
        let c = dbus::ffidisp::Connection::new_system().unwrap();

        let dest = "org.freedesktop.portal.Desktop";
        let path = "/org/freedesktop/portal/desktop";
        let interface = "org.freedesktop.portal.Settings.Read";
        let prop = Props::new(&c, dest, path, interface, 10000);

        match prop.get_all() {
            Ok(a) => println!("Results {:#?}", a),
            Err(e) => println!("Error! {:?}", e),
        }
        /*   debug!("Version: {:?}", prop.get("Version").unwrap());
        debug!("Architecture: {:?}", prop.get("Architecture").unwrap()); */
    }

    #[test]
    fn test_prop34() -> Result<(), Box<dyn std::error::Error>> {
        let dest = "org.freedesktop.portal.Desktop";
        let path = "/org/freedesktop/portal/desktop";
        let interface = "org.freedesktop.portal.Settings.Read";
        let connection = dbus::blocking::Connection::new_session()?;
        let proxy = connection.with_proxy(dest, path, std::time::Duration::from_millis(5000));

        use super::dbus::blocking::stdintf::org_freedesktop_dbus::Properties;

        let metadata: super::dbus::arg::Variant<String> = proxy.get(interface, "Version")?;

        debug!("Meta: {:?}", metadata);
        Ok(())
    }

    #[test]
    fn test_prop3() -> Result<(), Box<dyn std::error::Error>> {
        let dest = DESTINATION_SYSTEMD;
        let path = "/org/freedesktop/systemd1";
        let interface = "org.freedesktop.systemd1.Manager";
        let connection = dbus::blocking::Connection::new_session()?;
        let proxy = connection.with_proxy(dest, path, std::time::Duration::from_millis(5000));

        use super::dbus::blocking::stdintf::org_freedesktop_dbus::Properties;

        let metadata: super::dbus::arg::Variant<String> = proxy.get(interface, "Version")?;

        debug!("Meta: {:?}", metadata);
        Ok(())
    }

    #[test]
    pub fn test_get_unit_path() -> Result<(), SystemdErrors> {
        let unit_file: &str = TEST_SERVICE;
        let mut message = dbus_message("GetUnit")?;
        let message_items = &[MessageItem::Str(unit_file.to_owned())];
        message.append_items(message_items);

        let load_unit_ret = dbus_connect(message)?;
        debug!("{:?}", load_unit_ret);
        Ok(())
    }

    #[test]
    pub fn test_get_unit_parameters() {
        init();
        let dest = DESTINATION_SYSTEMD;
        let path = "/org/freedesktop/systemd1/unit/jackett_2eservice";

        let interface = INTERFACE_SYSTEMD_UNIT;
        let c = dbus::ffidisp::Connection::new_system().unwrap();
        let p = Props::new(&c, dest, path, interface, 10000);

        debug!("ALL PARAM: {:#?}", p.get_all());
    }

    #[test]
    pub fn test_fetch_system_unit_info() -> Result<(), SystemdErrors> {
        init();

        let btree_map = fetch_system_unit_info("/org/freedesktop/systemd1/unit/jackett_2eservice")?;

        debug!("ALL PARAM: {:#?}", btree_map);
        Ok(())
    }

    #[test]
    pub fn test_load_unit_() -> Result<(), SystemdErrors> {
        let unit_file: &str = TEST_SERVICE;
        let mut message = dbus_message("LoadUnit")?;
        let message_items = &[MessageItem::Str(unit_file.to_owned())];
        message.append_items(message_items);

        let load_unit_ret = dbus_connect(message)?;
        debug!("{:?}", load_unit_ret);
        Ok(())
    }
}
