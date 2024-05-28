extern crate dbus;

use systemd::dbus::dbus::arg::messageitem::MessageItem;
use systemd::dbus::dbus::Error;
use systemd::dbus::dbus::Message;

/// Takes a systemd dbus function as input and returns the result as a `dbus::Message`.
fn dbus_message(function: &str) -> Message {
    let dest = "org.freedesktop.systemd1";
    let node = "/org/freedesktop/systemd1";
    let interface = "org.freedesktop.systemd1.Manager";
    let message = dbus::Message::new_method_call(dest, node, interface, function)
        .unwrap_or_else(|e| panic!("{}", e));
    message
}

/// Takes a `dbus::Message` as input and makes a connection to dbus, returning the reply.
fn dbus_connect(message: Message) -> Result<Message, Error> {
    let connection = dbus::ffidisp::Connection::get_private(dbus::ffidisp::BusType::System)?;

    connection.send_with_reply_and_block(message, 4000)
}

#[derive(Clone, Debug)]
pub struct SystemdUnit {
    pub name: String,
    pub prefix: String,
    pub state: UnitState,
    pub utype: UnitType,
    pub path: String,
}


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
            _ => {
                println!("Unknown Type: {}", system_type);
                UnitType::Unknown(system_type.to_string())
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnitState {
    Bad,
    Disabled,
    Enabled,
    Indirect,
    Linked,
    Masked,
    Static,
    Alias,
    Generated,
    Trancient,
    Unknown,
}
impl UnitState {
    /// Takes the string containing the state information from the dbus message and converts it
    /// into a UnitType by matching the first character.
    pub fn new(system_type: &str) -> UnitState {
        let c = if system_type.is_empty() {
            'Z'
        } else {
            system_type.chars().next().unwrap()
        };

        match c {
            'a' => UnitState::Alias,
            's' => UnitState::Static,
            'd' => UnitState::Disabled,
            'e' => UnitState::Enabled,
            'i' => UnitState::Indirect,
            'l' => UnitState::Linked,
            'm' => UnitState::Masked,
            'b' => UnitState::Bad,
            'g' => UnitState::Generated,
            't' => UnitState::Trancient,
            _ => {
                println!("Unknown State: {}", system_type);
                UnitState::Unknown
            }
        }
    }
}

/// Takes the dbus message as input and maps the information to a `Vec<SystemdUnit>`.
fn parse_message(message_item: &MessageItem) -> Vec<SystemdUnit> {
    let MessageItem::Array(array) = message_item else {
        eprintln!("Malformed message");
        return vec![];
    };

    let mut systemd_units: Vec<SystemdUnit> = Vec::with_capacity(array.len());

    for service_struct in array.into_iter() {
        let MessageItem::Struct(struct_value) = service_struct else {
            continue;
        };

        if struct_value.len() >= 2 {
            let Some(MessageItem::Str(service)) = struct_value.get(0) else {
                continue;
            };

            let Some(MessageItem::Str(status)) = struct_value.get(1) else {
                continue;
            };

            let Some((prefix, name_type)) = service.rsplit_once('/') else {
                continue;
            };

            let Some((name, system_type)) = name_type.rsplit_once('.') else {
                continue;
            };

            let state = UnitState::new(&status);
            let utype = UnitType::new(system_type);
            systemd_units.push(SystemdUnit {
                name: name.to_owned(),
                prefix: prefix.to_owned(),
                state,
                utype,
                path : service.to_owned()
            });
        }
    }

    systemd_units
}

/// Communicates with dbus to obtain a list of unit files and returns them as a `Vec<SystemdUnit>`.
pub fn list_unit_files() -> Vec<SystemdUnit> {
    let message_vec = list_unit_files_message();
    //println!("MESSAGE {:?}", message);

    let message_item = if message_vec.len() >= 1 {
        message_vec.get(0).expect("Missing argument")
    } else {
        panic!("Always suppose have one item")
    };

    parse_message(message_item)
}

fn list_unit_files_message() -> Vec<MessageItem> {
    let message = dbus_message("ListUnitFiles");
    match dbus_connect(message) {
        Ok(m) => m.get_items(),
        Err(e) => {
            eprintln!("Error! {}", e);
            Vec::new()
        }
    }
}

/// Returns the current enablement status of the unit
pub fn get_unit_file_state(path: &str) -> bool {
    for unit in list_unit_files() {
        if unit.name.as_str() == path {
            return unit.state == UnitState::Enabled;
        }
    }
    false
}

/// Takes a `Vec<SystemdUnit>` as input and returns a new vector only containing services which can be enabled and
/// disabled.
pub fn collect_togglable_services(units: &[SystemdUnit]) -> Vec<SystemdUnit> {
    units
        .iter()
        .filter(|x| {
            x.utype == UnitType::Service
                && (x.state == UnitState::Enabled || x.state == UnitState::Disabled)
                && !x.name.contains("/etc/")
        })
        .cloned()
        .collect()
}

/// Takes a `Vec<SystemdUnit>` as input and returns a new vector only containing sockets which can be enabled and
/// disabled.
pub fn collect_togglable_sockets(units: &[SystemdUnit]) -> Vec<SystemdUnit> {
    units
        .iter()
        .filter(|x| {
            x.utype == UnitType::Socket
                && (x.state == UnitState::Enabled || x.state == UnitState::Disabled)
        })
        .cloned()
        .collect()
}

/// Takes a `Vec<SystemdUnit>` as input and returns a new vector only containing timers which can be enabled and
/// disabled.
pub fn collect_togglable_timers(units: &[SystemdUnit]) -> Vec<SystemdUnit> {
    units
        .iter()
        .filter(|x| {
            x.utype == UnitType::Timer
                && (x.state == UnitState::Enabled || x.state == UnitState::Disabled)
        })
        .cloned()
        .collect()
}

/// Takes the unit pathname of a service and enables it via dbus.
/// If dbus replies with `[Bool(true), Array([], "(sss)")]`, the service is already enabled.
pub fn enable_unit_files(unit: &str) -> Option<String> {
    let mut message = dbus_message("EnableUnitFiles");
    message.append_items(&[[unit][..].into(), false.into(), true.into()]);
    match dbus_connect(message) {
        Ok(reply) => {
            if format!("{:?}", reply.get_items()) == "[Bool(true), Array([], \"(sss)\")]" {
                println!("{} already enabled", unit);
            } else {
                println!("{} has been enabled", unit);
            }
            None
        }
        Err(reply) => {
            let error = format!("Error enabling {}:\n{:?}", unit, reply);
            println!("{}", error);
            Some(error)
        }
    }
}

/// Takes the unit pathname as input and disables it via dbus.
/// If dbus replies with `[Array([], "(sss)")]`, the service is already disabled.
pub fn disable_unit_files(unit: &str) -> Option<String> {
    let mut message = dbus_message("DisableUnitFiles");
    message.append_items(&[[unit][..].into(), false.into()]);
    match dbus_connect(message) {
        Ok(reply) => {
            if format!("{:?}", reply.get_items()) == "[Array([], \"(sss)\")]" {
                println!("{} is already disabled", unit);
            } else {
                println!("{} has been disabled", unit);
            }
            None
        }
        Err(reply) => {
            let error = format!("Error disabling {}:\n{:?}", unit, reply);
            println!("{}", error);
            Some(error)
        }
    }
}

/// Takes a unit name as input and attempts to start it
pub fn start_unit(unit: &str) -> Option<String> {
    let mut message = dbus_message("StartUnit");
    message.append_items(&[unit.into(), "fail".into()]);
    match dbus_connect(message) {
        Ok(_) => {
            println!("{} successfully started", unit);
            None
        }
        Err(error) => {
            let output = format!("{} failed to start:\n{:?}", unit, error);
            println!("{}", output);
            Some(output)
        }
    }
}

/// Takes a unit name as input and attempts to stop it.
pub fn stop_unit(unit: &str) -> Option<String> {
    let mut message = dbus_message("StopUnit");
    message.append_items(&[unit.into(), "fail".into()]);
    match dbus_connect(message) {
        Ok(_) => {
            println!("{} successfully stopped", unit);
            None
        }
        Err(error) => {
            let output = format!("{} failed to stop:\n{:?}", unit, error);
            println!("{}", output);
            Some(output)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_unit_files_message_test() {
        let message_vec = list_unit_files_message();
        //println!("{:?}", message);

        let message_item = if message_vec.len() >= 1 {
            message_vec.get(0).expect("Missing argument")
        } else {
            panic!("Aways suppose have one item")
        };

        handle_message_item(message_item)
    }

    fn handle_message_item(message_item: &MessageItem) {
        match message_item {
            MessageItem::Array(array) => {
                //let _ = array.into_iter().map(|item| println!("{:?}", item));
                for (i, n) in array.into_iter().enumerate() {
                    println!("{} - {:?}", i, n);
                    handle_message_item(n);
                }
            }
            MessageItem::Struct(struct_) => {
                for a in struct_.into_iter() {
                    //println!("{} - {:?}", i , n);
                    handle_message_item(a);
                }
            }
            MessageItem::Variant(_) => todo!(),
            MessageItem::Dict(_) => todo!(),
            MessageItem::ObjectPath(_) => todo!(),
            MessageItem::Signature(_) => todo!(),
            MessageItem::Str(_str_value) => {
                //println!("{}", str_value );
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
    fn list_unit_files_message_test2() {
        let message_vec = list_unit_files_message();
        //println!("{:?}", message);

        let message_item = if message_vec.len() >= 1 {
            message_vec.get(0).expect("Missing argument")
        } else {
            panic!("Aways suppose have one item")
        };

        let asdf = parse_message(message_item);

        println!("{:#?}", asdf)
    }

    
}
