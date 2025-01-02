//! Dbus abstraction
//! Documentation can be found at https://www.freedesktop.org/wiki/Software/systemd/dbus/

use std::collections::BTreeMap;
use std::collections::HashMap;

use log::debug;

/* use dbus::arg::messageitem::MessageItem;
use dbus::Message; */
use log::info;
use log::trace;
use log::warn;
use serde::Deserialize;
use zbus::blocking::fdo;
use zbus::blocking::Connection;
use zbus::blocking::MessageIterator;
use zbus::message::Flags;
use zbus::names::InterfaceName;
use zbus::Message;

use zvariant::DynamicType;
use zvariant::ObjectPath;
use zvariant::OwnedValue;
use zvariant::Str;
use zvariant::Type;

use crate::systemd::data::UnitInfo;
use crate::systemd::enums::ActiveState;
use crate::systemd::enums::UnitType;
use crate::widget::preferences::data::DbusLevel;

use super::enums::EnablementStatus;

use super::enums::KillWho;
use super::enums::StartStopMode;
use super::SystemdErrors;
use super::SystemdUnit;

const DESTINATION_SYSTEMD: &str = "org.freedesktop.systemd1";
pub(super) const INTERFACE_SYSTEMD_UNIT: &str = "org.freedesktop.systemd1.Unit";
pub(super) const INTERFACE_SYSTEMD_MANAGER: &str = "org.freedesktop.systemd1.Manager";
const PATH_SYSTEMD: &str = "/org/freedesktop/systemd1";

const METHOD_LIST_UNIT: &str = "ListUnits";

const METHOD_LIST_UNIT_FILES: &str = "ListUnitFiles";

const METHOD_START_UNIT: &str = "StartUnit";
const METHOD_STOP_UNIT: &str = "StopUnit";
const METHOD_RESTART_UNIT: &str = "RestartUnit";
const METHOD_GET_UNIT_FILE_STATE: &str = "GetUnitFileState";
const METHOD_KILL_UNIT: &str = "KillUnit";
const METHOD_GET_UNIT: &str = "GetUnit";
const METHOD_ENABLE_UNIT_FILES: &str = "EnableUnitFiles";
const METHOD_DISABLE_UNIT_FILES: &str = "DisableUnitFiles";
const METHOD_RELOAD: &str = "Reload";

/// Communicates with dbus to obtain a list of unit files and returns them as a `Vec<SystemdUnit>`.
pub fn list_unit_files(connection: &Connection) -> Result<Vec<SystemdUnit>, SystemdErrors> {
    let message = connection.call_method(
        Some(DESTINATION_SYSTEMD),
        PATH_SYSTEMD,
        Some(INTERFACE_SYSTEMD_MANAGER),
        METHOD_LIST_UNIT_FILES,
        &(),
    )?;

    let body = message.body();

    let array: Vec<LUnitFiles> = body.deserialize()?;

    let mut systemd_units: Vec<SystemdUnit> = Vec::with_capacity(array.len());

    for unit_file in array.iter() {
        let Some((_prefix, name_type)) = unit_file.primary_unit_name.rsplit_once('/') else {
            return Err(SystemdErrors::Malformed);
        };

        let Some((name, system_type)) = name_type.rsplit_once('.') else {
            return Err(SystemdErrors::Malformed);
        };

        let status_code = EnablementStatus::new(unit_file.enablement_status);
        let utype = UnitType::new(system_type);

        systemd_units.push(SystemdUnit {
            name: name.to_owned(),
            status_code,
            utype,
            path: unit_file.primary_unit_name.to_owned(),
        });
    }

    Ok(systemd_units)
}

#[derive(Deserialize, Type, PartialEq, Debug)]
struct LUnitFiles<'a> {
    primary_unit_name: &'a str,
    enablement_status: &'a str,
}

#[derive(Deserialize, Type, PartialEq, Debug)]
struct LUnit<'a> {
    primary_unit_name: &'a str,
    description: &'a str,
    load_state: &'a str,
    active_state: &'a str,
    sub_state: &'a str,
    followed_unit: &'a str,
    #[serde(borrow)]
    unit_object_path: ObjectPath<'a>,
    ///If there is a job queued for the job unit the numeric job id, 0 otherwise
    numeric_job_id: u32,
    job_type: &'a str,
    job_object_path: ObjectPath<'a>,
}

fn get_connection(level: DbusLevel) -> Result<Connection, SystemdErrors> {
    debug!("Level {:?}, id {}", level, level as u32);
    let connection_builder = match level {
        DbusLevel::Session => zbus::blocking::connection::Builder::session()?,
        DbusLevel::System => zbus::blocking::connection::Builder::system()?,
    };

    let connection = connection_builder
        .auth_mechanism(zbus::AuthMechanism::External)
        .build()?;

    //println!("connection {:#?}", connection);

    Ok(connection)
}

fn list_units_description(
    connection: &Connection,
) -> Result<BTreeMap<String, UnitInfo>, SystemdErrors> {
    let message = connection.call_method(
        Some(DESTINATION_SYSTEMD),
        PATH_SYSTEMD,
        Some(INTERFACE_SYSTEMD_MANAGER),
        METHOD_LIST_UNIT,
        &(),
    )?;

    let body = message.body();

    let array: Vec<LUnit> = body.deserialize()?;

    let mut map: BTreeMap<String, UnitInfo> = BTreeMap::new();

    for service_struct in array.iter() {
        let active_state: ActiveState = service_struct.active_state.into();

        let unit = UnitInfo::new(
            service_struct.primary_unit_name,
            service_struct.description,
            service_struct.load_state,
            active_state,
            service_struct.sub_state,
            service_struct.followed_unit,
            service_struct.unit_object_path.as_str(),
        );

        map.insert(service_struct.primary_unit_name.to_ascii_lowercase(), unit);
    }

    Ok(map)
}

/// Returns the current enablement status of the unit
pub fn get_unit_file_state_path(
    level: DbusLevel,
    unit_file: &str,
) -> Result<EnablementStatus, SystemdErrors> {
    let connection = get_connection(level)?;

    let message = connection.call_method(
        Some(DESTINATION_SYSTEMD),
        PATH_SYSTEMD,
        Some(INTERFACE_SYSTEMD_MANAGER),
        METHOD_GET_UNIT_FILE_STATE,
        &(unit_file),
    )?;

    let body = message.body();
    let enablement_status: &str = body.deserialize()?;

    Ok(EnablementStatus::new(enablement_status))
}

pub fn list_units_description_and_state(
    level: DbusLevel,
) -> Result<BTreeMap<String, UnitInfo>, SystemdErrors> {
    let connection = get_connection(level)?;

    let mut units_map = list_units_description(&connection)?;

    let mut unit_files = list_unit_files(&connection)?;

    for unit_file in unit_files.drain(..) {
        match units_map.get_mut(&unit_file.full_name().to_ascii_lowercase()) {
            Some(unit_info) => {
                fill_unit_file(unit_info, &unit_file);
            }
            None => {
                log::debug!(
                    "Unit \"{}\" status \"{}\" not loaded!",
                    unit_file.full_name(),
                    unit_file.status_code.to_string()
                );
                let mut unit = UnitInfo::new(
                    unit_file.full_name(),
                    "",
                    "",
                    ActiveState::Unknown,
                    "",
                    "",
                    "",
                );
                fill_unit_file(&mut unit, &unit_file);
                units_map.insert(unit_file.full_name().to_ascii_lowercase(), unit);
            }
        }
    }

    Ok(units_map)
}

fn fill_unit_file(unit_info: &mut UnitInfo, unit_file: &SystemdUnit) {
    unit_info.set_file_path(Some(unit_file.path.clone()));
    let status_code: u32 = unit_file.status_code.into();
    unit_info.set_enable_status(status_code);
}

/// Takes a unit name as input and attempts to start it
pub(super) fn start_unit(
    level: DbusLevel,
    unit_name: &str,
    mode: StartStopMode,
) -> Result<String, SystemdErrors> {
    send_disenable_message(
        level,
        METHOD_START_UNIT,
        &(unit_name, mode.as_str()),
        handle_start_stop_answer,
    )
}

fn handle_start_stop_answer(
    method: &str,
    return_message: &Message,
) -> Result<String, SystemdErrors> {
    let body = return_message.body();

    let job_path: zvariant::ObjectPath = body.deserialize()?;

    let created_job_object = job_path.to_string();
    info!("{method} SUCCESS, response job id {created_job_object}");

    return Ok(created_job_object);
}

/// Takes a unit name as input and attempts to stop it.
pub(super) fn stop_unit(
    level: DbusLevel,
    unit_name: &str,
    mode: StartStopMode,
) -> Result<String, SystemdErrors> {
    send_disenable_message(
        level,
        METHOD_STOP_UNIT,
        &(unit_name, mode.as_str()),
        handle_start_stop_answer,
    )
}

/// Enqeues a start job, and possibly depending jobs.
pub(super) fn restart_unit(
    level: DbusLevel,
    unit: &str,
    mode: StartStopMode,
) -> Result<String, SystemdErrors> {
    send_disenable_message(
        level,
        METHOD_RESTART_UNIT,
        &(unit, mode.as_str()),
        handle_start_stop_answer,
    )
}

#[derive(Debug, Type, Deserialize)]
#[allow(unused)]
pub struct DisEnAbleUnitFiles {
    pub change_type: String,
    pub file_name: String,
    pub destination: String,
}

#[derive(Debug, Type, Deserialize)]
#[allow(unused)]
pub struct EnableUnitFilesReturn {
    pub carries_install_info: bool,
    pub vec: Vec<DisEnAbleUnitFiles>,
}

pub(super) fn enable_unit_files(
    level: DbusLevel,
    unit_name: &str,
) -> Result<EnableUnitFilesReturn, SystemdErrors> {
    let v = vec![unit_name];

    fn handle_answer(
        _method: &str,
        return_message: &Message,
    ) -> Result<EnableUnitFilesReturn, SystemdErrors> {
        let body = return_message.body();

        info!("body signature {}", body.signature());

        let return_msg: EnableUnitFilesReturn = body.deserialize()?;

        debug!("ret {:?}", return_msg);

        return Ok(return_msg);
    }

    send_disenable_message(
        level,
        METHOD_ENABLE_UNIT_FILES,
        &(v, false, true),
        handle_answer,
    )
}

pub(super) fn disable_unit_files(
    level: DbusLevel,
    unit_names: &[&str],
) -> Result<Vec<DisEnAbleUnitFiles>, SystemdErrors> {
    fn handle_answer(
        _method: &str,
        return_message: &Message,
    ) -> Result<Vec<DisEnAbleUnitFiles>, SystemdErrors> {
        let body = return_message.body();

        let return_msg: Vec<DisEnAbleUnitFiles> = body.deserialize()?;

        debug!("ret {:?}", return_msg);

        return Ok(return_msg);
    }

    send_disenable_message(
        level,
        METHOD_DISABLE_UNIT_FILES,
        &(unit_names, false),
        handle_answer,
    )
}

fn send_disenable_message<T, U>(
    level: DbusLevel,
    method: &str,
    body: &T,
    handler: impl Fn(&str, &Message) -> Result<U, SystemdErrors>,
) -> Result<U, SystemdErrors>
where
    T: serde::ser::Serialize + DynamicType,
    U: std::fmt::Debug,
{
    let message = Message::method_call(PATH_SYSTEMD, method)?
        .with_flags(Flags::AllowInteractiveAuth)?
        .destination(DESTINATION_SYSTEMD)?
        .interface(INTERFACE_SYSTEMD_MANAGER)?
        .build(body)?;

    let connection = get_connection(level)?;

    connection.send(&message)?;

    let mut stream = MessageIterator::from(connection);

    while let Some(message_res) = stream.next() {
        debug!("Message response {:?}", message_res);
        match message_res {
            Ok(return_message) => match return_message.message_type() {
                zbus::message::Type::MethodReturn => {
                    let result = handler(method, &return_message);
                    info!("{method} Response {:?}", result);
                    return result;
                }
                zbus::message::Type::MethodCall => {
                    warn!("Not supposed to happen");
                    break;
                }
                zbus::message::Type::Error => {
                    let error = zbus::Error::from(return_message);
                    return Err(SystemdErrors::from(error));
                }
                zbus::message::Type::Signal => continue,
            },
            Err(e) => return Err(SystemdErrors::from(e)),
        };
        //unreaceble
        //break;
    }

    warn!("{:?} ????, response supposed to be Unreachable", method);
    Err(SystemdErrors::Malformed)
}

/// Used to get the unit object path for a unit name
pub fn get_unit_object_path(level: DbusLevel, unit_name: &str) -> Result<String, SystemdErrors> {
    let connection = get_connection(level)?;

    get_unit_object_path_connection(unit_name, &connection)
}

fn get_unit_object_path_connection(
    unit_name: &str,
    connection: &Connection,
) -> Result<String, SystemdErrors> {
    let message = connection.call_method(
        Some(DESTINATION_SYSTEMD),
        PATH_SYSTEMD,
        Some(INTERFACE_SYSTEMD_MANAGER),
        METHOD_GET_UNIT,
        &(unit_name),
    )?;

    let body = message.body();

    let object_path: zvariant::ObjectPath = body.deserialize()?;

    Ok(object_path.as_str().to_owned())
}

pub fn reload_all_units(level: DbusLevel) -> Result<(), SystemdErrors> {
    fn reload_answer(method: &str, _return_message: &Message) -> Result<(), SystemdErrors> {
        info!("{method} SUCCESS");
        return Ok(());
    }

    send_disenable_message(level, METHOD_RELOAD, &(), reload_answer)
}

pub(super) fn kill_unit(
    level: DbusLevel,
    unit_name: &str,
    mode: KillWho,
    signal: i32,
) -> Result<(), SystemdErrors> {
    fn handle_answer(_method: &str, _return_message: &Message) -> Result<(), SystemdErrors> {
        info!("Kill SUCCESS");

        return Ok(());
    }

    send_disenable_message(
        level,
        METHOD_KILL_UNIT,
        &(unit_name, mode.as_str(), signal),
        handle_answer,
    )
}

fn convert_to_string(value: &zvariant::Value) -> String {
    let str_value: String = match value {
        zvariant::Value::U8(i) => i.to_string(),
        zvariant::Value::Bool(b) => b.to_string(),
        zvariant::Value::I16(i) => i.to_string(),
        zvariant::Value::U16(i) => i.to_string(),
        zvariant::Value::I32(i) => i.to_string(),
        zvariant::Value::U32(i) => i.to_string(),
        zvariant::Value::I64(i) => i.to_string(),
        zvariant::Value::U64(i) => i.to_string(),
        zvariant::Value::F64(i) => i.to_string(),
        zvariant::Value::Str(s) => s.to_string(),
        zvariant::Value::Signature(s) => s.to_string(),
        zvariant::Value::ObjectPath(op) => op.to_string(),
        zvariant::Value::Value(v) => v.to_string(),
        zvariant::Value::Array(a) => {
            let mut d_str = String::from("[ ");

            let mut it = a.iter().peekable();
            while let Some(mi) = it.next() {
                d_str.push_str(&convert_to_string(mi));
                if it.peek().is_some() {
                    d_str.push_str(", ");
                }
            }

            d_str.push_str(" ]");
            d_str
        }
        zvariant::Value::Dict(d) => {
            let mut d_str = String::from("{ ");
            for (mik, miv) in d.iter() {
                d_str.push_str(&convert_to_string(&mik));
                d_str.push_str(" : ");
                d_str.push_str(&convert_to_string(&miv));
            }
            d_str.push_str(" }");
            d_str
        }
        zvariant::Value::Structure(stc) => {
            let mut d_str = String::from("{ ");

            let mut it = stc.fields().iter().peekable();
            while let Some(mi) = it.next() {
                d_str.push_str(&convert_to_string(mi));
                if it.peek().is_some() {
                    d_str.push_str(", ");
                }
            }

            d_str.push_str(" }");
            d_str
        }
        zvariant::Value::Fd(fd) => fd.to_string(),
        //zvariant::Value::Maybe(maybe) => maybe.to_string(),
    };
    str_value
}

pub fn fetch_system_info(level: DbusLevel) -> Result<BTreeMap<String, String>, SystemdErrors> {
    fetch_system_unit_info(level, PATH_SYSTEMD, UnitType::Manager)
}

pub fn fetch_system_unit_info(
    level: DbusLevel,
    path: &str,
    unit_type: UnitType,
) -> Result<BTreeMap<String, String>, SystemdErrors> {
    let mut properties: HashMap<String, OwnedValue> =
        fetch_system_unit_info_native(level, path, unit_type)?;

    let mut map = BTreeMap::new();

    for (key, value) in properties.drain() {
        trace!("{:?} {:?}", key, value);

        let str_val = convert_to_string(&value);
        map.insert(key.to_owned(), str_val);
    }

    Ok(map)
}

pub fn fetch_system_unit_info_native(
    level: DbusLevel,
    path: &str,
    unit_type: UnitType,
) -> Result<HashMap<String, OwnedValue>, SystemdErrors> {
    let connection = get_connection(level)?;

    debug!("path {path}");
    let properties_proxy: zbus::blocking::fdo::PropertiesProxy =
        fdo::PropertiesProxy::builder(&connection)
            .destination(DESTINATION_SYSTEMD)?
            .path(path)?
            .build()?;

    let unit_interface = unit_type.interface();

    let interface_name = InterfaceName::try_from(unit_interface).unwrap();

    let mut properties: HashMap<String, OwnedValue> = properties_proxy.get_all(interface_name)?;

    if unit_type.extends_unit() {
        let unit_interface_name = InterfaceName::try_from(INTERFACE_SYSTEMD_UNIT).unwrap();

        let unit_properties: HashMap<String, OwnedValue> =
            properties_proxy.get_all(unit_interface_name)?;

        properties.extend(unit_properties);
    }

    trace!("properties {:?}", properties);
    Ok(properties)
}

pub fn fetch_unit(level: DbusLevel, unit_primary_name: &str) -> Result<UnitInfo, SystemdErrors> {
    let connection = get_connection(level)?;

    let object_path = get_unit_object_path_connection(unit_primary_name, &connection)?;

    debug!("path {object_path}");
    let properties_proxy: zbus::blocking::fdo::PropertiesProxy =
        fdo::PropertiesProxy::builder(&connection)
            .destination(DESTINATION_SYSTEMD)?
            .path(object_path.clone())?
            .build()?;

    let interface_name = InterfaceName::try_from(INTERFACE_SYSTEMD_UNIT).unwrap();

    /*     The primary unit name as string
    The human readable description string
    The load state (i.e. whether the unit file has been loaded successfully)
    The active state (i.e. whether the unit is currently started or not)
    The sub state (a more fine-grained version of the active state that is specific to the unit type, which the active state is not)
    A unit that is being followed in its state by this unit, if there is any, otherwise the empty string.
    The unit object path
    If there is a job queued for the job unit the numeric job id, 0 otherwise
    The job type as string
    The job object path
     */

    let primary: Str<'_> = properties_proxy
        .get(interface_name.clone(), "Id")?
        .try_into()?;

    let description: Str<'_> = properties_proxy
        .get(interface_name.clone(), "Description")?
        .try_into()?;

    let load_state: Str<'_> = properties_proxy
        .get(interface_name.clone(), "LoadState")?
        .try_into()?;

    let active_state_str: Str<'_> = properties_proxy
        .get(interface_name.clone(), "ActiveState")?
        .try_into()?;

    let active_state: ActiveState = active_state_str.as_str().into();

    let sub_state: Str<'_> = properties_proxy
        .get(interface_name.clone(), "SubState")?
        .try_into()?;
    let followed_unit: Str<'_> = properties_proxy
        .get(interface_name, "Following")?
        .try_into()?;

    let unit = UnitInfo::new(
        &primary,
        &description,
        &load_state,
        active_state,
        &sub_state,
        &followed_unit,
        &object_path,
    );

    Ok(unit)
}

#[cfg(test)]
mod tests {

    use super::*;

    pub const TEST_SERVICE: &str = "tiny_daemon.service";

    fn init() {
        let _ = env_logger::builder()
            .target(env_logger::Target::Stdout)
            .filter_level(log::LevelFilter::Debug)
            .is_test(true)
            .try_init();
    }

    #[ignore = "need a connection to a service"]
    #[test]
    fn stop_service_test() -> Result<(), SystemdErrors> {
        stop_unit(DbusLevel::System, TEST_SERVICE, StartStopMode::Fail)?;
        Ok(())
    }

    #[ignore = "need a connection to a service"]
    #[test]
    fn test_get_unit_file_state() {
        let file1: &str = TEST_SERVICE;

        let status = get_unit_file_state_path(DbusLevel::System, file1);
        debug!("Status: {:?}", status);
    }

    #[ignore = "need a connection to a service"]
    #[test]
    fn test_list_unit_files() -> Result<(), SystemdErrors> {
        let units = list_unit_files(&get_connection(DbusLevel::System)?)?;

        let serv = units
            .iter()
            .filter(|ud| ud.full_name() == TEST_SERVICE)
            .nth(0);

        debug!("{:#?}", serv);
        Ok(())
    }

    #[ignore = "need a connection to a service"]
    #[test]
    fn test_list_units() -> Result<(), SystemdErrors> {
        let units = list_units_description(&get_connection(DbusLevel::System)?)?;

        let serv = units.get(TEST_SERVICE);
        debug!("{:#?}", serv);
        Ok(())
    }

    #[ignore = "need a connection to a service"]
    #[test]
    pub fn test_get_unit_path() -> Result<(), SystemdErrors> {
        let unit_file: &str = "tiny_daemon.service";

        let connection = get_connection(DbusLevel::System)?;

        let message = connection.call_method(
            Some(DESTINATION_SYSTEMD),
            PATH_SYSTEMD,
            Some(INTERFACE_SYSTEMD_MANAGER),
            "GetUnit",
            &(unit_file),
        )?;

        println!("message {:?}", message);

        let body = message.body();

        let z: zvariant::ObjectPath = body.deserialize()?;
        //let z :String = body.deserialize()?;

        println!("obj {:?}", z.as_str());

        /*         let body = message.body();

        let des = body.deserialize();

        println!("{:#?}", des); */
        Ok(())
    }

    #[ignore = "need a connection to a service"]
    #[test]
    pub fn test_fetch_system_unit_info() -> Result<(), SystemdErrors> {
        init();

        let btree_map = fetch_system_unit_info(
            DbusLevel::System,
            "/org/freedesktop/systemd1/unit/tiny_5fdaemon_2eservice",
            UnitType::Service,
        )?;

        debug!("ALL PARAM: {:#?}", btree_map);
        Ok(())
    }

    #[ignore = "need a connection to a service"]
    #[test]
    fn test_enable_unit_files() -> Result<(), SystemdErrors> {
        init();
        let _res = enable_unit_files(DbusLevel::System, TEST_SERVICE)?;

        Ok(())
    }

    #[ignore = "need a connection to a service"]
    #[test]
    fn test_disable_unit_files() -> Result<(), SystemdErrors> {
        init();
        let _res = disable_unit_files(DbusLevel::System, &[TEST_SERVICE])?;

        Ok(())
    }

    #[ignore = "need a connection to a service"]
    #[test]
    fn test_fetch_info() -> Result<(), SystemdErrors> {
        init();

        let path = get_unit_object_path(DbusLevel::System, TEST_SERVICE)?;

        println!("unit {} Path {}", TEST_SERVICE, path);
        let map = fetch_system_unit_info(DbusLevel::System, &path, UnitType::Service)?;

        println!("{:#?}", map);
        Ok(())
    }

    #[ignore = "need a connection to a service"]
    #[test]
    fn test_fetch_system_info() -> Result<(), SystemdErrors> {
        init();

        let map = fetch_system_info(DbusLevel::System)?;

        info!("{:#?}", map);
        Ok(())
    }

    #[ignore = "need a connection to a service"]
    #[test]
    fn test_fetch_unit() -> Result<(), SystemdErrors> {
        init();

        let unit = fetch_unit(DbusLevel::System, TEST_SERVICE)?;

        info!("{:#?}", unit);
        Ok(())
    }

    #[ignore = "need a connection to a service"]
    #[test]
    fn test_fetch_unit_fail_wrong_name() -> Result<(), SystemdErrors> {
        init();

        let fake = format!("{TEST_SERVICE}_fake");
        match fetch_unit(DbusLevel::System, &fake) {
            Ok(_) => todo!(),
            Err(e) => {
                warn!("{:?}", e);
                if let SystemdErrors::NoSuchUnit(_msg) = e {
                    return Ok(());
                } else {
                    return Err(SystemdErrors::Custom("Wrong expected Error".to_owned()));
                }
            }
        }
    }
}
