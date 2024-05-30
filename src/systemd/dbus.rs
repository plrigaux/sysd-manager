extern crate dbus;

use std::process::Command;

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

    //let connection = dbus::blocking::Connection::new_session()?;

    let duration = std::time::Duration::new(30, 0);
    connection.send_with_reply_and_block(message, 30000)
}

#[derive(Clone, Debug)]
pub struct SystemdUnit {
    pub name: String,
    pub state: EnablementStatus,
    pub utype: UnitType,
    pub path: String,
}

impl SystemdUnit {
    pub fn full_name(&self) -> &str {
        match self.path.rsplit_once("/") {
            Some((_, end)) => end,
            None => &self.name,
        }
    }
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
                println!("Unknown Type: {}", system_type);
                UnitType::Unknown(system_type.to_string())
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EnablementStatus {
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
impl EnablementStatus {
    /// Takes the string containing the state information from the dbus message and converts it
    /// into a UnitType by matching the first character.
    pub fn new(enablement_status: &str) -> EnablementStatus {
        if enablement_status.is_empty() {
            eprintln!("Empty Status: {}", enablement_status);
            return EnablementStatus::Unknown;
        }

        let c = enablement_status.chars().next().unwrap();

        match c {
            'a' => EnablementStatus::Alias,
            's' => EnablementStatus::Static,
            'd' => EnablementStatus::Disabled,
            'e' => EnablementStatus::Enabled,
            'i' => EnablementStatus::Indirect,
            'l' => EnablementStatus::Linked,
            'm' => EnablementStatus::Masked,
            'b' => EnablementStatus::Bad,
            'g' => EnablementStatus::Generated,
            't' => EnablementStatus::Trancient,
            _ => {
                println!("Unknown State: {}", enablement_status);
                EnablementStatus::Unknown
            }
        }
    }
}

/// Takes the dbus message as input and maps the information to a `Vec<SystemdUnit>`.
fn parse_message(message_item: &MessageItem) -> Vec<SystemdUnit> {
    println!("parse_message");
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
            let Some(MessageItem::Str(systemd_unit)) = struct_value.get(0) else {
                continue;
            };

            let Some(MessageItem::Str(status)) = struct_value.get(1) else {
                continue;
            };

            let Some((_prefix, name_type)) = systemd_unit.rsplit_once('/') else {
                continue;
            };

            let Some((name, system_type)) = name_type.rsplit_once('.') else {
                continue;
            };

            let state = EnablementStatus::new(&status);
            let utype = UnitType::new(system_type);

            if name.eq("jackett") {
                println!("{systemd_unit} {status} {system_type}")
            }
            systemd_units.push(SystemdUnit {
                name: name.to_owned(),
                state,
                utype,
                path: systemd_unit.to_owned(),
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

pub fn get_unit_file_state(sytemd_unit: &SystemdUnit) -> EnablementStatus {
    return get_unit_file_state_path(sytemd_unit.full_name());
}

/// Returns the current enablement status of the unit
fn get_unit_file_state_path(unit_file: &str) -> EnablementStatus {
    let mut message = dbus_message("GetUnitFileState");
    let message_items = &[MessageItem::Str(unit_file.to_owned())];
    message.append_items(message_items);

    match dbus_connect(message) {
        Ok(m) => {
            if let Some(enablement_status) = m.get1::<String>() {
                EnablementStatus::new(&enablement_status)
            } else {
                eprintln!("Something wrong");
                EnablementStatus::Unknown
            }
        }
        Err(e) => {
            eprintln!("Error! {}", e);
            EnablementStatus::Unknown
        }
    }
}

/// Takes a `Vec<SystemdUnit>` as input and returns a new vector only containing services which can be enabled and
/// disabled.
pub fn collect_togglable_services(units: &Vec<SystemdUnit>) -> Vec<SystemdUnit> {
    units
        .iter()
        .filter(|x| {
            x.utype == UnitType::Service
                && (x.state == EnablementStatus::Enabled || x.state == EnablementStatus::Disabled)
            // && !x.path.contains("/etc/")
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
                && (x.state == EnablementStatus::Enabled || x.state == EnablementStatus::Disabled)
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
                && (x.state == EnablementStatus::Enabled || x.state == EnablementStatus::Disabled)
        })
        .cloned()
        .collect()
}

pub fn enable_unit_files(sytemd_unit: &SystemdUnit) -> Option<String> {
    enable_unit_files_path(sytemd_unit.full_name())
}
/// Takes the unit pathname of a service and enables it via dbus.
/// If dbus replies with `[Bool(true), Array([], "(sss)")]`, the service is already enabled.
fn enable_unit_files_path(unit: &str) -> Option<String> {
    let command_output = Command::new("systemctl").arg("enable").arg(unit).output();
    match command_output {
        Ok(output) => {
            println!("Status {}", output.status);
            println!("stdout: {}", String::from_utf8(output.stdout).unwrap());
            eprintln!("stderr: {}", String::from_utf8(output.stderr).unwrap());
            None
        },
        Err(error) => {
            eprintln!("{}", error);
            Some("ERROR".to_owned())
        }
    }
/*     println!("0Try to enable: {}", unit);
    match pkexec::pkexec() {
        Ok(_) => {}
        Err(e) => return Some("Need priv".to_owned()),
    }

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
    } */
}

pub fn disable_unit_files(sytemd_unit: &SystemdUnit) -> Option<String> {
    disable_unit_files_path(sytemd_unit.full_name())
}

/// Takes the unit pathname as input and disables it via dbus.
/// If dbus replies with `[Array([], "(sss)")]`, the service is already disabled.
fn disable_unit_files_path(unit: &str) -> Option<String> {
    let command_output = Command::new("systemctl").arg("disable").arg(unit).output();
    match command_output {
        Ok(output) => {
            println!("Status {}", output.status);
            println!("stdout: {}", String::from_utf8(output.stdout).unwrap());
            eprintln!("stderr: {}", String::from_utf8(output.stderr).unwrap());
            None
        },
        Err(error) => {
            eprintln!("{}", error);
            Some("ERROR".to_owned())
        }
    }

    /*     println!("0Try to disable: {}", unit);
    match pkexec::escalate_if_needed() {
        Ok(_user) => {println!("Run as {:?}", _user)},
        Err(_e) => return Some("Need private".to_owned())
    }

    let mut message = dbus_message("DisableUnitFiles");

    //let sig = Signature::new("asdf").unwrap();
    //let mia = MessageItemArray::new(&vec![MessageItem::Str(unit.to_owned())], sig);
    //Ok(MessageItem::Array(MessageItemArray::new(v, s)?))

    //let message_items = &[mia, MessageItem::Bool(true)];
    //message.append_items(message_items);
    println!("Try to disable: {}", unit);
    message.append_items(&[[unit][..].into(), false.into()]);

    println!("Message: {:?}", message);
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
    } */
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

    #[test]
    fn stop_service_test() {
        stop_unit("jacket.service");
    }

    #[test]
    fn dbus_test() {
        // let file: &str = "/etc/systemd/system/jackett.service";
        let file1: &str = "jackett.service";
        let mut message = dbus_message("GetUnitFileState");

        let message_items = &[MessageItem::Str(file1.to_owned())];
        message.append_items(message_items);

        match dbus_connect(message) {
            Ok(m) => println!("{:?}", m.get1::<String>()),
            Err(e) => {
                eprintln!("Error! {}", e);
            }
        }
    }

    #[test]
    fn test_get_unit_file_state() {
        // let file: &str = "/etc/systemd/system/jackett.service";
        let file1: &str = "jackett.service";

        let status = get_unit_file_state_path(file1);
        println!("Status: {:?}", status);
    }

    /// to make it work
    /// put in your projet  .cargo/config.toml file
    /// [target.x86_64-unknown-linux-gnu]
    /// runner = 'sudo -E'
    ///
    #[test]
    fn test_disable_unit_files_path() {
        //let file1: &str = "/etc/systemd/system/jackett.service";
        let file1: &str = "jackett.service";

        let status = disable_unit_files_path(file1);
        println!("Status: {:?}", status);

        let status = disable_unit_files_path(file1);
        println!("Status: {:?}", status);
    }
/* 
    #[test]
    fn test_disable_unit_files_path_w_priv() {
        //let file1: &str = "/etc/systemd/system/jackett.service";
        let file1: &str = "jackett.service";
        let res = pkexec::pkexec();
        println!("Result: {:?}", res);
        let status = disable_unit_files_path(file1);
        println!("Status: {:?}", status);
    } */
    /*
      #[test]
      fn test_privilege() {

          match karen::escalate_if_needed() {
              Ok(w) => {
                  println!("Hello, Root-World!");
                  println!("{:?}", w);
              }
              Err(e) => println!("Error {:?}", e),
          };
      }

      #[test]
      fn test_privilege2() {
          match karen::pkexec() {
              Ok(a) =>  println!("Run as: {:?}", a),
              Err(e) => println!("Error {:?}", e),
          }

          println!("OK OK");
          match karen::pkexec() {
              Ok(a) =>  println!("Run as: {:?}", a),
              Err(e) => println!("Error {:?}", e),
          }
      }

      #[test]
      fn test_privilege3() {
          let run_as = karen::check() ;
          println!("Run as: {:?}", run_as)
    /*        {
              Ok(a) =>  println!("Run as: {:?}", a),
              Err(e) => println!("Error {:?}", e),
          } */


      }

      #[test]
      fn test_privilege4() {
          match karen::pkexec() {
              Ok(a) =>  println!("Run as: {:?}", a),
              Err(e) => println!("Error {:?}", e),
          }

          println!("OK OK");
          match karen::pkexec() {
              Ok(a) =>  println!("Run as: {:?}", a),
              Err(e) => println!("Error {:?}", e),
          }
      } */
}
