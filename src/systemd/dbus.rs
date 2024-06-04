pub extern crate dbus as msgbus;

use std::collections::BTreeMap;

use log::trace;
use log::debug;
use log::info;

use self::msgbus::arg::messageitem::MessageItem;
use self::msgbus::Message;

use super::EnablementStatus;

use super::LoadedUnit;
use super::SystemdErrors;
use super::SystemdUnit;

/// Takes a systemd dbus function as input and returns the result as a `dbus::Message`.
fn dbus_message(function: &str) -> Result<Message, SystemdErrors> {
    let dest = "org.freedesktop.systemd1";
    let path = "/org/freedesktop/systemd1";
    let interface = "org.freedesktop.systemd1.Manager";
    match msgbus::Message::new_method_call(dest, path, interface, function) {
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
    let p = connection.with_proxy("org.freedesktop.systemd1", "/org/freedesktop/systemd1", duration);


    // The Metadata property is a Dict<String, Variant>.

    // Option 1: we can get the dict straight into a hashmap, like this:
    use systemd::dbus::msgbus::blocking::stdintf::org_freedesktop_dbus::Properties;

    let metadata = p.get("org.mpris.MediaPlayer2.Player", "Metadata")?;

    debug!("Option 1:");

    Ok(())

} */

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnitType {
    Automount,
    Busname,
    Mount,
    Path,
    Scope,
    Service,
    Slice,
    Socket,
    Target,
    Timer,
    Swap,
    Unknown(String),
}
impl UnitType {
    /// Takes the pathname of the unit as input to determine what type of unit it is.
    pub fn new(system_type: &str) -> UnitType {
        match system_type {
            "automount" => UnitType::Automount,
            "busname" => UnitType::Busname,
            "mount" => UnitType::Mount,
            "path" => UnitType::Path,
            "scope" => UnitType::Scope,
            "service" => UnitType::Service,
            "slice" => UnitType::Slice,
            "socket" => UnitType::Socket,
            "target" => UnitType::Target,
            "timer" => UnitType::Timer,
            "swap" => UnitType::Swap,
            _ => {
                info!("Unknown Type: {}", system_type);
                UnitType::Unknown(system_type.to_string())
            }
        }
    }
}

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

            let state = EnablementStatus::new(&status);
            let utype = UnitType::new(system_type);

            let path = systemd_unit.to_owned();

            systemd_units.push(SystemdUnit {
                name: name.to_owned(),
                state,
                utype,
                path: path,
                enable_status: status.clone(),
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

fn list_units_description() -> Result<BTreeMap<String, LoadedUnit>, SystemdErrors> {
    let message = dbus_message("ListUnits")?;
    debug!("MESSAGE {:?}", message);
    let m = dbus_connect(message)?;

    // debug!("{:#?}",m.get_items())
    let mi = m.get_items();
    debug!("{:#?}", mi.len());
    let message_item = &mi[0];

    let sig: dbus::Signature<'_> = message_item.signature();
    //"a(ssssssouso)\0",
    debug!("{:#?}", sig);

    let MessageItem::Array(array) = message_item else {
        return Err(SystemdErrors::MalformedWrongArgType(
            message_item.arg_type(),
        ));
    };
    debug!("Array_size {:#?}", array.len());

    let mut map: BTreeMap<String, LoadedUnit> = BTreeMap::new();

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
        let MessageItem::Str(ref active_state) = struct_value[3] else {
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

        let unit_info = LoadedUnit::new(
            primary,
            description,
            load_state,
            active_state,
            sub_state,
            followed_unit,
            object_path.to_string(),
        );

        map.insert(primary.to_ascii_lowercase(), unit_info);
    }
    Ok(map)
}

/// Returns the current enablement status of the unit
pub fn get_unit_file_state_path(unit_file: &str) -> Result<EnablementStatus, SystemdErrors> {
    let mut message = dbus_message("GetUnitFileState")?;
    let message_items = &[MessageItem::Str(unit_file.to_owned())];
    message.append_items(message_items);

    let m = dbus_connect(message)?;

    if let Some(enablement_status) = m.get1::<String>() {
        Ok(EnablementStatus::new(&enablement_status))
    } else {
        Err(SystemdErrors::Malformed)
    }
}

pub fn list_units_description_and_state() -> Result<BTreeMap<String, LoadedUnit>, SystemdErrors> {
    let mut units_map = list_units_description()?;

    let mut units = list_unit_files()?;

    for unit_file in units.drain(..) {
        match units_map.get_mut(&unit_file.full_name().to_ascii_lowercase()) {
            Some(lu) => {
                lu.file_path = Some(unit_file.path);
                lu.enable_status = Some(unit_file.enable_status)
            }
            None => debug!("unit \"{}\" not found!", unit_file.full_name()),
        }
    }

    Ok(units_map)
}

/// Takes the unit pathname of a service and enables it via dbus.
/// If dbus replies with `[Bool(true), Array([], "(sss)")]`, the service is already enabled.
/* fn enable_unit_files_path(unit: &str) -> Option<String> {

    let mut message = dbus_message("EnableUnitFiles");
    message.append_items(&[[unit][..].into(), false.into(), true.into()]);
    match dbus_connect(message) {
        Ok(reply) => {
            if format!("{:?}", reply.get_items()) == "[Bool(true), Array([], \"(sss)\")]" {
                debug!("{} already enabled", unit);
            } else {
                debug!("{} has been enabled", unit);
            }
            None
        }
        Err(reply) => {
            let error = format!("Error enabling {}:\n{:?}", unit, reply);
            debug!("{}", error);
            Some(error)
        }
    }
} */

/// Takes the unit pathname as input and disables it via dbus.
/// If dbus replies with `[Array([], "(sss)")]`, the service is already disabled.
/* fn disable_unit_files_path(unit: &str) -> Option<String> {

    let mut message = dbus_message("DisableUnitFiles");

    debug!("Try to disable: {}", unit);
    message.append_items(&[[unit][..].into(), false.into()]);

    debug!("Message: {:?}", message);
    match dbus_connect(message) {
        Ok(reply) => {
            if format!("{:?}", reply.get_items()) == "[Array([], \"(sss)\")]" {
                debug!("{} is already disabled", unit);
            } else {
                debug!("{} has been disabled", unit);
            }
            None
        }
        Err(reply) => {
            let error = format!("Error disabling {}:\n{:?}", unit, reply);
            debug!("{}", error);
            Some(error)
        }
    }
} */

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

#[cfg(test)]
mod tests {

    use std::collections::HashSet;

    /* use crate::systemd::collect_togglable_services; */

    use super::msgbus::arg::messageitem::Props;

    use super::*;

    pub const TEST_SERVICE: &str = "jackett.service";

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
                Some(lu) => {
                    lu.file_path = Some(unit_file.path);
                    lu.enable_status = Some(unit_file.enable_status)
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
        let c = msgbus::ffidisp::Connection::new_system().unwrap();
        let p = Props::new(
            &c,
            "org.freedesktop.PolicyKit1",
            "/org/freedesktop/PolicyKit1/Authority",
            "org.freedesktop.PolicyKit1.Authority",
            10000,
        );
        debug!("BackendVersion: {:?}", p.get("BackendVersion").unwrap())
    }

    #[test]
    fn test_prop2() {
        let c = msgbus::ffidisp::Connection::new_system().unwrap();

        let dest = "org.freedesktop.systemd1";
        let path = "/org/freedesktop/systemd1";
        let interface = "org.freedesktop.systemd1.Manager";
        let prop = Props::new(&c, dest, path, interface, 10000);
        debug!("Version: {:?}", prop.get("Version").unwrap());
        debug!("Architecture: {:?}", prop.get("Architecture").unwrap());

        //debug!("ActiveState: {:?}", p.get("ActiveState").unwrap());
    }

    #[test]
    fn test_prop3() -> Result<(), Box<dyn std::error::Error>> {
        let dest = "org.freedesktop.systemd1";
        let path = "/org/freedesktop/systemd1";
        let interface = "org.freedesktop.systemd1.Manager";
        let connection = msgbus::blocking::Connection::new_session()?;
        let proxy = connection.with_proxy(dest, path, std::time::Duration::from_millis(5000));

        use super::msgbus::blocking::stdintf::org_freedesktop_dbus::Properties;

        let metadata: super::msgbus::arg::Variant<String> = proxy.get(interface, "Version")?;

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
        let dest = "org.freedesktop.systemd1";
        let path = "/org/freedesktop/systemd1/unit/jackett_2eservice";
        let interface = "org.freedesktop.systemd1.Unit";
        let c = msgbus::ffidisp::Connection::new_system().unwrap();
        let p = Props::new(&c, dest, path, interface, 10000);

        debug!("ALL PARAM: {:#?}", p.get_all());
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
