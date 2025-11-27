//! Dbus abstraction
//! Documentation can be found at https://www.freedesktop.org/wiki/Software/systemd/dbus/
pub(super) mod watcher;

#[cfg(test)]
mod tests;

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    str::FromStr,
    sync::RwLock,
};

use enumflags2::BitFlags;
use log::{debug, error, info, trace, warn};

use serde::Deserialize;

use zbus::{
    Message,
    blocking::{Connection, MessageIterator, Proxy, fdo},
    message::Flags,
    names::InterfaceName,
    proxy,
};
use zvariant::{Array, DynamicType, ObjectPath, OwnedValue, Str, Type};

use crate::{
    Dependency, SystemdUnitFile, UnitPropertyFetch, UpdatedUnitInfo,
    data::{DisEnAbleUnitFiles, EnableUnitFilesReturn, LUnit, UnitInfo},
    enums::{
        ActiveState, DependencyType, DisEnableFlags, EnablementStatus, KillWho, LoadState,
        StartStopMode, UnitDBusLevel, UnitType,
    },
    errors::SystemdErrors,
};

pub(crate) const DESTINATION_SYSTEMD: &str = "org.freedesktop.systemd1";
pub(super) const INTERFACE_SYSTEMD_UNIT: &str = "org.freedesktop.systemd1.Unit";
pub(super) const INTERFACE_SYSTEMD_MANAGER: &str = "org.freedesktop.systemd1.Manager";
pub(super) const INTERFACE_PROPERTIES: &str = "org.freedesktop.DBus.Properties";
pub(crate) const PATH_SYSTEMD: &str = "/org/freedesktop/systemd1";

const METHOD_LIST_UNIT: &str = "ListUnits";
const METHOD_LIST_UNIT_FILES: &str = "ListUnitFiles";

const METHOD_GET: &str = "Get";
const METHOD_START_UNIT: &str = "StartUnit";
const METHOD_STOP_UNIT: &str = "StopUnit";
const METHOD_RESTART_UNIT: &str = "RestartUnit";
const METHOD_GET_UNIT_FILE_STATE: &str = "GetUnitFileState";
const METHOD_KILL_UNIT: &str = "KillUnit";
const METHOD_QUEUE_SIGNAL_UNIT: &str = "QueueSignalUnit";
const METHOD_CLEAN_UNIT: &str = "CleanUnit";
const METHOD_MASK_UNIT_FILES: &str = "MaskUnitFiles";
const METHOD_UNMASK_UNIT_FILES: &str = "UnmaskUnitFiles";
const METHOD_GET_UNIT: &str = "GetUnit";
const METHOD_ENABLE_UNIT_FILES: &str = "EnableUnitFilesWithFlags";
const METHOD_DISABLE_UNIT_FILES: &str = "DisableUnitFilesWithFlags";
pub const METHOD_RELOAD: &str = "Reload";
pub const METHOD_GET_UNIT_PROCESSES: &str = "GetUnitProcesses";
pub const METHOD_FREEZE_UNIT: &str = "FreezeUnit";
pub const METHOD_THAW_UNIT: &str = "ThawUnit";
pub const METHOD_RELOAD_UNIT: &str = "ReloadUnit";

const METHOD_PRESET_UNIT_FILES: &str = "PresetUnitFiles";
const METHOD_LINK_UNIT_FILES: &str = "LinkUnitFiles";
const METHOD_REENABLE_UNIT_FILES: &str = "ReenableUnitFiles";

#[derive(Deserialize, Type, PartialEq, Debug)]
struct LUnitFiles<'a> {
    primary_unit_name: &'a str,
    enablement_status: &'a str,
}

pub static BLK_CON_SYST: RwLock<Option<Connection>> = RwLock::new(None);
pub static BLK_CON_USER: RwLock<Option<Connection>> = RwLock::new(None);

fn get_blocking_connection(level: UnitDBusLevel) -> Result<Connection, SystemdErrors> {
    let lock = match level {
        UnitDBusLevel::UserSession => &BLK_CON_USER,
        _ => &BLK_CON_SYST,
    };

    if let Some(ref conn) = *lock.read().unwrap() {
        return Ok(conn.clone());
    }

    let connection = build_blocking_connection(level)?;

    *lock.write().unwrap() = Some(connection.clone());

    Ok(connection)
}

fn build_blocking_connection(level: UnitDBusLevel) -> Result<Connection, SystemdErrors> {
    debug!("Getting connection Level {:?}, id {}", level, level as u32);
    let connection_builder = match level {
        UnitDBusLevel::UserSession => zbus::blocking::connection::Builder::session()?,
        _ => zbus::blocking::connection::Builder::system()?,
    };

    let connection = connection_builder
        .auth_mechanism(zbus::AuthMechanism::External)
        .build()?;

    debug!("Connection Sync: {connection:?}");

    Ok(connection)
}

pub static CON_ASYNC_SYST: RwLock<Option<zbus::Connection>> = RwLock::new(None);
pub static CON_ASYNC_USER: RwLock<Option<zbus::Connection>> = RwLock::new(None);

async fn get_connection(level: UnitDBusLevel) -> Result<zbus::Connection, SystemdErrors> {
    let lock = match level {
        UnitDBusLevel::UserSession => &CON_ASYNC_USER,
        _ => &CON_ASYNC_SYST,
    };

    if let Some(ref conn) = *lock.read().unwrap() {
        return Ok(conn.clone());
    }

    let connection = build_connection(level).await?;

    *lock.write().unwrap() = Some(connection.clone());

    Ok(connection)
}

async fn build_connection(level: UnitDBusLevel) -> Result<zbus::Connection, SystemdErrors> {
    debug!("Level {:?}, id {}", level, level as u32);
    let connection_builder = match level {
        UnitDBusLevel::UserSession => zbus::connection::Builder::session()?,
        _ => zbus::connection::Builder::system()?,
    };

    let connection = connection_builder
        .auth_mechanism(zbus::AuthMechanism::External)
        .build()
        .await?;

    trace!("Connection Async: {connection:#?}");

    Ok(connection)
}

async fn list_units_list_async(connection: zbus::Connection) -> Result<Vec<LUnit>, SystemdErrors> {
    let message = call_method_async(
        &connection,
        DESTINATION_SYSTEMD,
        PATH_SYSTEMD,
        INTERFACE_SYSTEMD_MANAGER,
        METHOD_LIST_UNIT,
        &(),
    )
    .await?;

    let body = message.body();

    let array: Vec<LUnit> = body.deserialize()?;

    Ok(array)
}

/// Returns the current enablement status of the unit
pub fn get_unit_file_state(
    level: UnitDBusLevel,
    unit_file: &str,
) -> Result<EnablementStatus, SystemdErrors> {
    let message = call_systemd_manager_method(level, METHOD_GET_UNIT_FILE_STATE, &(unit_file))?;

    let body = message.body();
    let enablement_status: &str = body.deserialize()?;

    EnablementStatus::from_str(enablement_status)
}

fn call_systemd_manager_method<B>(
    level: UnitDBusLevel,
    method: &str,
    body: &B,
) -> Result<Message, SystemdErrors>
where
    B: serde::ser::Serialize + DynamicType,
{
    let connection = get_blocking_connection(level)?;

    call_method(
        &connection,
        DESTINATION_SYSTEMD,
        PATH_SYSTEMD,
        INTERFACE_SYSTEMD_MANAGER,
        method,
        body,
    )
}

fn call_method<B>(
    connection: &Connection,
    destination: &str,
    path: &str,
    iface: &str,
    method: &str,
    body: &B,
) -> Result<Message, SystemdErrors>
where
    B: serde::ser::Serialize + DynamicType,
{
    connection
        .call_method(Some(destination), path, Some(iface), method, body)
        .map_err(|e| SystemdErrors::from((e, method)))
}

pub async fn call_method_async<B>(
    connection: &zbus::Connection,
    destination: &str,
    path: &str,
    iface: &str,
    method: &str,
    body: &B,
) -> Result<Message, SystemdErrors>
where
    B: serde::ser::Serialize + DynamicType,
{
    connection
        .call_method(Some(destination), path, Some(iface), method, body)
        .await
        .map_err(|e| SystemdErrors::from((e, method)))
}

pub async fn get_unit_file_state_async(
    connection: &zbus::Connection,
    unit_file: &str,
) -> Result<EnablementStatus, SystemdErrors> {
    let message = call_method_async(
        connection,
        DESTINATION_SYSTEMD,
        PATH_SYSTEMD,
        INTERFACE_SYSTEMD_MANAGER,
        METHOD_GET_UNIT_FILE_STATE,
        &(unit_file),
    )
    .await?;

    let body = message.body();
    let enablement_status: &str = body.deserialize()?;

    EnablementStatus::from_str(enablement_status)
}

pub async fn list_units_description_and_state_async(
    level: UnitDBusLevel,
) -> Result<(Vec<LUnit>, Vec<SystemdUnitFile>), SystemdErrors> {
    let connection = get_connection(level).await?;

    let t1 = tokio::spawn(list_units_list_async(connection.clone()));
    let t2 = tokio::spawn(list_unit_files_async(connection, level));

    let joined = tokio::join!(t1, t2);

    let units_map = joined.0??;
    let unit_files = joined.1??;

    Ok((units_map, unit_files))
}

pub async fn complete_unit_information(
    units: Vec<(UnitDBusLevel, String, String)>,
) -> Result<Vec<UpdatedUnitInfo>, SystemdErrors> {
    let mut connection_system = None;
    let mut connection_session = None;

    let mut ouput = Vec::with_capacity(units.len());
    for (dbus_level, unit_primary, object_path) in units {
        let connection = match dbus_level {
            UnitDBusLevel::UserSession => {
                if let Some(conn) = &connection_session {
                    conn
                } else {
                    let conn = get_connection(dbus_level).await?;
                    connection_session.get_or_insert(conn) as &zbus::Connection
                }
            }
            _ => {
                if let Some(conn) = &connection_system {
                    conn
                } else {
                    let conn = get_connection(dbus_level).await?;
                    connection_system.get_or_insert(conn) as &zbus::Connection
                }
            }
        };

        let f2 = get_unit_file_state_async(connection, &unit_primary);
        let f1 = complete_unit_info(connection, &unit_primary, object_path);

        let (r1, r2) = tokio::join!(f1, f2);

        match r1 {
            Ok(mut updated_unit_info) => {
                updated_unit_info.enablement_status = r2.ok();
                ouput.push(updated_unit_info)
            }
            Err(error) => warn!("Complete unit \"{unit_primary}\" error {error:?}"),
        }
    }
    Ok(ouput)
}

#[proxy(
    interface = "org.freedesktop.systemd1.Unit",
    default_service = "org.freedesktop.systemd1"
)]
trait ZUnitInfo {
    #[zbus(property)]
    fn id(&self) -> Result<String, zbus::Error>;

    #[zbus(property)]
    fn description(&self) -> Result<String, zbus::Error>;

    #[zbus(property)]
    fn load_state(&self) -> Result<String, zbus::Error>;

    #[zbus(property)]
    fn active_state(&self) -> Result<String, zbus::Error>;

    #[zbus(property)]
    fn sub_state(&self) -> Result<String, zbus::Error>;

    #[zbus(property)]
    fn following(&self) -> Result<String, zbus::Error>;

    #[zbus(property)]
    fn fragment_path(&self) -> Result<String, zbus::Error>;

    #[zbus(property)]
    fn unit_file_state(&self) -> Result<String, zbus::Error>;

    #[zbus(property)]
    fn unit_file_preset(&self) -> Result<String, zbus::Error>;

    #[zbus(property)]
    fn drop_in_paths(&self) -> Result<Vec<String>, zbus::Error>;
}

macro_rules! get_completing_info {
    ($unit_info_proxy:expr, $f:ident) => {
        $unit_info_proxy
            .$f()
            .await
            .map_err(|e| {
                warn!("bus {e:?}");
                SystemdErrors::from(e)
            })
            .ok()
    };
}

macro_rules! fill_completing_info {
    ($update:expr, $unit_info_proxy:expr, $f:ident) => {
        $update.$f = get_completing_info!($unit_info_proxy, $f);
    };
}

async fn complete_unit_info(
    connection: &zbus::Connection,
    unit_primary: &String,
    object_path: String,
) -> Result<UpdatedUnitInfo, SystemdErrors> {
    let unit_info_proxy = ZUnitInfoProxy::builder(connection)
        .path(object_path.clone())?
        .build()
        .await?;

    let mut update = UpdatedUnitInfo::new(unit_primary.to_owned(), object_path);

    if let Err(error) = fill_update(unit_info_proxy, &mut update).await {
        debug!("Complete info Error: {error:?}");
    }

    Ok(update)
}

async fn fill_update(
    unit_info_proxy: ZUnitInfoProxy<'_>,
    update: &mut UpdatedUnitInfo,
) -> Result<(), SystemdErrors> {
    let active_state = unit_info_proxy.active_state().await?;
    let active_state: ActiveState = active_state.as_str().into();
    update.active_state = Some(active_state);
    fill_completing_info!(update, unit_info_proxy, description);

    let load_state = get_completing_info!(unit_info_proxy, load_state);
    let load_state: LoadState = load_state.into();
    update.load_state = Some(load_state);

    fill_completing_info!(update, unit_info_proxy, sub_state);
    fill_completing_info!(update, unit_info_proxy, unit_file_preset);

    update.fragment_path = unit_info_proxy
        .fragment_path()
        .await
        .map(|path| if path.is_empty() { None } else { Some(path) })?;

    update.valid_unit_name = true;

    Ok(())
}

fn fill_list_unit_files(
    array: Vec<LUnitFiles>,
    level: UnitDBusLevel,
) -> Result<Vec<SystemdUnitFile>, SystemdErrors> {
    let mut systemd_units: Vec<SystemdUnitFile> = Vec::with_capacity(array.len());

    for unit_file in array.iter() {
        let Some((_prefix, full_name)) = unit_file.primary_unit_name.rsplit_once('/') else {
            error!(
                "MALFORMED rsplit_once(\"/\") {:?}",
                unit_file.primary_unit_name,
            );
            continue;
        };

        let status_code =
            EnablementStatus::from_str(unit_file.enablement_status).expect("Always status");

        systemd_units.push(SystemdUnitFile {
            full_name: full_name.to_owned(),
            status_code,
            level,
            path: unit_file.primary_unit_name.to_owned(),
        });
    }

    Ok(systemd_units)
}

/// Communicates with dbus to obtain a list of unit files and returns them as a `Vec<SystemdUnit>`.
pub async fn list_unit_files_async(
    connection: zbus::Connection,
    level: UnitDBusLevel,
) -> Result<Vec<SystemdUnitFile>, SystemdErrors> {
    let message = call_method_async(
        &connection,
        DESTINATION_SYSTEMD,
        PATH_SYSTEMD,
        INTERFACE_SYSTEMD_MANAGER,
        METHOD_LIST_UNIT_FILES,
        &(),
    )
    .await?;

    let body = message.body();

    let array: Vec<LUnitFiles> = body.deserialize()?;

    fill_list_unit_files(array, level)
}

/// Takes a unit name as input and attempts to start it
pub(super) fn start_unit(
    level: UnitDBusLevel,
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

    Ok(created_job_object)
}

/// Takes a unit name as input and attempts to stop it.
pub(super) fn stop_unit(
    level: UnitDBusLevel,
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
    level: UnitDBusLevel,
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

pub(super) fn enable_unit_files(
    level: UnitDBusLevel,
    unit_names_or_files: &[&str],
    flags: BitFlags<DisEnableFlags>,
) -> Result<EnableUnitFilesReturn, SystemdErrors> {
    fn handle_answer(
        _method: &str,
        return_message: &Message,
    ) -> Result<EnableUnitFilesReturn, SystemdErrors> {
        let body = return_message.body();

        let return_msg: EnableUnitFilesReturn = body.deserialize()?;

        info!("Enable unit files {return_msg:?}");

        Ok(return_msg)
    }

    send_disenable_message(
        level,
        METHOD_ENABLE_UNIT_FILES,
        &(unit_names_or_files, flags.bits_c() as u64),
        handle_answer,
    )
}

pub(super) fn disable_unit_files(
    level: UnitDBusLevel,
    unit_names_or_files: &[&str],
    flags: BitFlags<DisEnableFlags>,
) -> Result<Vec<DisEnAbleUnitFiles>, SystemdErrors> {
    fn handle_answer(
        _method: &str,
        return_message: &Message,
    ) -> Result<Vec<DisEnAbleUnitFiles>, SystemdErrors> {
        let body = return_message.body();

        let return_msg: Vec<DisEnAbleUnitFiles> = body.deserialize()?;

        info!("Disable unit files {return_msg:?}");

        Ok(return_msg)
    }

    send_disenable_message(
        level,
        METHOD_DISABLE_UNIT_FILES,
        &(unit_names_or_files, flags.bits_c() as u64),
        handle_answer,
    )
}

fn send_disenable_message<T, U>(
    level: UnitDBusLevel,
    method: &str,
    body: &T,
    handler: impl Fn(&str, &Message) -> Result<U, SystemdErrors>,
) -> Result<U, SystemdErrors>
where
    T: serde::ser::Serialize + DynamicType + std::fmt::Debug,
    U: std::fmt::Debug,
{
    info!("Try to {method}, message body: {:?}", body);
    let message = Message::method_call(PATH_SYSTEMD, method)?
        .with_flags(Flags::AllowInteractiveAuth)?
        .destination(DESTINATION_SYSTEMD)?
        .interface(INTERFACE_SYSTEMD_MANAGER)?
        .build(body)?;

    let connection = get_blocking_connection(level)?;

    connection.send(&message)?;

    let message_it = MessageIterator::from(connection);

    for message_res in message_it {
        debug!("Message response {message_res:?}");
        let return_message = message_res?;

        match return_message.message_type() {
            zbus::message::Type::MethodReturn => {
                info!("{method} Response");
                let result = handler(method, &return_message);
                return result;
            }
            zbus::message::Type::MethodCall => {
                warn!("Not supposed to happen: {return_message:?}");
                break;
            }
            zbus::message::Type::Error => {
                let zb_error = zbus::Error::from(return_message);

                {
                    match zb_error {
                        zbus::Error::MethodError(
                            ref owned_error_name,
                            ref details,
                            ref message,
                        ) => {
                            warn!(
                                "Method error: {}\nDetails: {}\n{:?}",
                                owned_error_name.as_str(),
                                details.as_ref().map(|s| s.as_str()).unwrap_or_default(),
                                message
                            )
                        }
                        _ => warn!("Bus error: {zb_error:?}"),
                    }
                }
                let error = SystemdErrors::from((zb_error, method));
                return Err(error);
            }
            zbus::message::Type::Signal => {
                info!("Signal: {return_message:?}");
                continue;
            }
        }
    }

    let msg = format!("{method:?} ????, response supposed to be Unreachable");
    warn!("{msg}");
    Err(SystemdErrors::Malformed(
        msg,
        "sequences of messages".to_owned(),
    ))
}

#[allow(dead_code)]
fn get_unit_object_path_connection(
    unit_name: &str,
    connection: &Connection,
) -> Result<ObjectPath<'static>, SystemdErrors> {
    let message = call_method(
        connection,
        DESTINATION_SYSTEMD,
        PATH_SYSTEMD,
        INTERFACE_SYSTEMD_MANAGER,
        METHOD_GET_UNIT,
        &(unit_name),
    )?;

    let body = message.body();

    let object_path: zvariant::ObjectPath = body.deserialize()?;

    Ok(object_path.to_owned())
}

pub fn reload_all_units(level: UnitDBusLevel) -> Result<(), SystemdErrors> {
    //let handler_cloned: = handler;

    send_disenable_message(level, METHOD_RELOAD, &(), move |method, _message| {
        info!("{method} SUCCESS");
        Ok(())
    })
}

pub(super) fn kill_unit(
    level: UnitDBusLevel,
    unit_name: &str,
    mode: KillWho,
    signal: i32,
) -> Result<(), SystemdErrors> {
    let handler = |_method: &str, _return_message: &Message| -> Result<(), SystemdErrors> {
        info!("Kill Unit {unit_name} mode {mode} signal {signal} SUCCESS");
        Ok(())
    };

    send_disenable_message(
        level,
        METHOD_KILL_UNIT,
        &(unit_name, mode.as_str(), signal),
        handler,
    )
}

pub(super) fn freeze_unit(level: UnitDBusLevel, unit_name: &str) -> Result<(), SystemdErrors> {
    let handler = |_method: &str, _return_message: &Message| -> Result<(), SystemdErrors> {
        info!("Freeze Unit {unit_name} SUCCESS");
        Ok(())
    };

    send_disenable_message(level, METHOD_FREEZE_UNIT, &(unit_name), handler)
}

pub(super) fn thaw_unit(level: UnitDBusLevel, unit_name: &str) -> Result<(), SystemdErrors> {
    let handler = |_method: &str, _return_message: &Message| -> Result<(), SystemdErrors> {
        info!("Thaw Unit {unit_name} SUCCESS");
        Ok(())
    };

    send_disenable_message(level, METHOD_THAW_UNIT, &(unit_name), handler)
}

pub(super) fn preset_unit_file(
    level: UnitDBusLevel,
    files: &[&str],
    runtime: bool,
    force: bool,
) -> Result<EnableUnitFilesReturn, SystemdErrors> {
    let handler =
        |_method: &str, return_message: &Message| -> Result<EnableUnitFilesReturn, SystemdErrors> {
            let body = return_message.body();

            let return_msg: EnableUnitFilesReturn = body.deserialize()?;

            info!("Preset Unit Files {files:?} SUCCESS");
            Ok(return_msg)
        };

    send_disenable_message(
        level,
        METHOD_PRESET_UNIT_FILES,
        &(files, runtime, force),
        handler,
    )
}

pub(super) fn link_unit_files(
    level: UnitDBusLevel,
    files: &[&str],
    runtime: bool,
    force: bool,
) -> Result<Vec<DisEnAbleUnitFiles>, SystemdErrors> {
    let handler = |_method: &str,
                   return_message: &Message|
     -> Result<Vec<DisEnAbleUnitFiles>, SystemdErrors> {
        let body = return_message.body();

        let return_msg: Vec<DisEnAbleUnitFiles> = body.deserialize()?;

        info!("Link Unit Files {files:?} SUCCESS");
        Ok(return_msg)
    };
    send_disenable_message(
        level,
        METHOD_LINK_UNIT_FILES,
        &(files, runtime, force),
        handler,
    )
}

pub(super) fn reenable_unit_file(
    level: UnitDBusLevel,
    files: &[&str],
    runtime: bool,
    force: bool,
) -> Result<EnableUnitFilesReturn, SystemdErrors> {
    let handler =
        |_method: &str, return_message: &Message| -> Result<EnableUnitFilesReturn, SystemdErrors> {
            let body = return_message.body();

            let return_msg: EnableUnitFilesReturn = body.deserialize()?;

            info!("Reenable Unit Files {files:?} SUCCESS");
            Ok(return_msg)
        };
    send_disenable_message(
        level,
        METHOD_REENABLE_UNIT_FILES,
        &(files, runtime, force),
        handler,
    )
}

pub(super) fn reload_unit(
    level: UnitDBusLevel,
    unit_name: &str,
    mode: &str,
) -> Result<String, SystemdErrors> {
    send_disenable_message(
        level,
        METHOD_RELOAD_UNIT,
        &(unit_name, mode),
        handle_start_stop_answer,
    )
}

pub(super) fn queue_signal_unit(
    level: UnitDBusLevel,
    unit_name: &str,
    mode: KillWho,
    signal: i32,
    value: i32,
) -> Result<(), SystemdErrors> {
    fn handle_answer(_method: &str, _return_message: &Message) -> Result<(), SystemdErrors> {
        info!("Queue Signal SUCCESS");

        Ok(())
    }

    send_disenable_message(
        level,
        METHOD_QUEUE_SIGNAL_UNIT,
        &(unit_name, mode.as_str(), signal, value),
        handle_answer,
    )
}

pub(super) fn clean_unit(
    level: UnitDBusLevel,
    unit_name: &str,
    what: &[&str],
) -> Result<(), SystemdErrors> {
    let handle_answer = |_method: &str, _return_message: &Message| {
        info!("Clean Unit {unit_name} {what:?} SUCCESS");

        Ok(())
    };

    send_disenable_message(level, METHOD_CLEAN_UNIT, &(unit_name, what), handle_answer)
}

pub(super) fn mask_unit_files(
    level: UnitDBusLevel,
    files: &[&str],
    runtime: bool,
    force: bool,
) -> Result<Vec<DisEnAbleUnitFiles>, SystemdErrors> {
    let handle_answer = |_method: &str,
                         return_message: &Message|
     -> Result<Vec<DisEnAbleUnitFiles>, SystemdErrors> {
        info!("Mask Unit File {files:?} runtime {runtime:?} force {force:?} SUCCESS");

        let body = return_message.body();

        let return_msg: Vec<DisEnAbleUnitFiles> = body.deserialize()?;

        info!("Mask Unit File {return_msg:?}");

        Ok(return_msg)
    };

    send_disenable_message(
        level,
        METHOD_MASK_UNIT_FILES,
        &(files, runtime, force),
        handle_answer,
    )
}

pub(super) fn unmask_unit_files(
    level: UnitDBusLevel,
    files: &[&str],
    runtime: bool,
) -> Result<Vec<DisEnAbleUnitFiles>, SystemdErrors> {
    let handle_answer = |_method: &str,
                         return_message: &Message|
     -> Result<Vec<DisEnAbleUnitFiles>, SystemdErrors> {
        info!("UnMask Unit File {files:?} runtime {runtime:?}  SUCCESS");

        let body = return_message.body();

        let return_msg: Vec<DisEnAbleUnitFiles> = body.deserialize()?;

        info!("UnMask Unit File {return_msg:?}");

        Ok(return_msg)
    };

    send_disenable_message(
        level,
        METHOD_UNMASK_UNIT_FILES,
        &(files, runtime),
        handle_answer,
    )
}

fn convert_to_string(value: &zvariant::Value) -> String {
    match value {
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
                d_str.push_str(&convert_to_string(mik));
                d_str.push_str(" : ");
                d_str.push_str(&convert_to_string(miv));
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
    }
}

pub fn fetch_system_info(level: UnitDBusLevel) -> Result<BTreeMap<String, String>, SystemdErrors> {
    let res = change_p(level);
    warn!("res {res:?}");
    fetch_system_unit_info(level, PATH_SYSTEMD, UnitType::Manager)
}

pub fn fetch_system_unit_info(
    level: UnitDBusLevel,
    object_path: &str,
    unit_type: UnitType,
) -> Result<BTreeMap<String, String>, SystemdErrors> {
    let properties: HashMap<String, OwnedValue> =
        fetch_system_unit_info_native(level, object_path, unit_type)?;

    let mut map = BTreeMap::new();

    for (key, value) in properties.into_iter() {
        trace!("{key:?} {value:?}");

        let str_val = convert_to_string(&value);
        map.insert(key.to_owned(), str_val);
    }

    Ok(map)
}

fn change_p(level: UnitDBusLevel) -> Result<(), SystemdErrors> {
    let method = "Get";

    let body = ("org.freedesktop.systemd1.Manager", "LogLevel");

    let message = Message::method_call(PATH_SYSTEMD, method)?
        .destination(DESTINATION_SYSTEMD)?
        .interface("org.freedesktop.DBus.Properties")?
        .with_flags(Flags::AllowInteractiveAuth)?
        .build(&body)?;

    let connection = get_blocking_connection(level)?;

    connection.send(&message)?;

    let message_it = MessageIterator::from(connection);

    for message_res in message_it {
        debug!("Message response {message_res:?}");
        let return_message = message_res?;

        match return_message.message_type() {
            zbus::message::Type::MethodReturn => {
                info!("{method} Response");

                let body = return_message.body();

                let return_msg: OwnedValue = body.deserialize()?;

                info!("{method} Response {return_msg:?}");

                //  let result = handler(method, &return_message);
                return Ok(());
            }
            zbus::message::Type::MethodCall => {
                warn!("Not supposed to happen: {return_message:?}");
                break;
            }
            zbus::message::Type::Error => {
                let error = zbus::Error::from(return_message);
                return Err(SystemdErrors::from(error));
            }
            zbus::message::Type::Signal => {
                info!("Signal: {return_message:?}");
                continue;
            }
        }
    }

    let msg = format!("{method:?} ????, response supposed to be Unreachable");
    warn!("{msg}");
    Err(SystemdErrors::Malformed(
        msg,
        "sequences of messages".to_owned(),
    ))
}

pub fn fetch_system_unit_info_native(
    level: UnitDBusLevel,
    object_path: &str,
    unit_type: UnitType,
) -> Result<HashMap<String, OwnedValue>, SystemdErrors> {
    let connection = get_blocking_connection(level)?;

    debug!("Unit path: {object_path}");
    let properties_proxy: zbus::blocking::fdo::PropertiesProxy =
        fdo::PropertiesProxy::builder(&connection)
            .destination(DESTINATION_SYSTEMD)?
            .path(object_path)?
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

    trace!("properties {properties:?}");
    Ok(properties)
}

pub fn fetch_unit(
    level: UnitDBusLevel,
    unit_primary_name: &str,
) -> Result<UnitInfo, SystemdErrors> {
    let connection = get_blocking_connection(level)?;

    let object_path = unit_dbus_path_from_name(unit_primary_name);

    debug!("path {object_path}");

    let properties_proxy = ZUnitInfoProxyBlocking::builder(&connection)
        .destination(DESTINATION_SYSTEMD)?
        .path(object_path.clone())?
        .build()?;

    let primary = properties_proxy.id()?;
    let description = properties_proxy.description().unwrap_or_default();
    let load_state = properties_proxy.load_state().unwrap_or_default();
    let active_state_str = properties_proxy.active_state().unwrap_or_default();
    let sub_state = properties_proxy.sub_state().unwrap_or_default();
    let followed_unit = properties_proxy.following().unwrap_or_default();

    let listed_unit = LUnit {
        primary_unit_name: primary,
        description,
        load_state,
        active_state: active_state_str,
        sub_state,
        followed_unit,
        // unit_object_path: OwnedObjectPath::from(object_path),
        unit_object_path: ObjectPath::from_string_unchecked(object_path).into(),
        numeric_job_id: 0,
        job_type: String::new(),
        job_object_path: ObjectPath::from_static_str_unchecked("").into(),
    };

    let unit = UnitInfo::from_listed_unit(listed_unit, level);

    if let Ok(fragment_path) = properties_proxy.fragment_path() {
        unit.set_file_path(Some(fragment_path));
    }

    /*     match get_unit_file_state(level, unit_primary_name) {
        Ok(unit_file_status) => unit.set_enable_status(unit_file_status as u8),
        Err(err) => warn!("Fail to get unit file state : {:?}", err),
    } */

    Ok(unit)
}

pub(super) fn unit_get_dependencies(
    dbus_level: UnitDBusLevel,
    unit_name: &str,
    unit_object_path: &str,
    dependency_type: DependencyType,
    plain: bool,
) -> Result<Dependency, SystemdErrors> {
    let connection = get_blocking_connection(dbus_level)?;
    let dependencies_properties = dependency_type.properties();
    let mut units = HashSet::new();

    let mut dependency = Dependency::new(unit_name);
    //writeln!(out, "{}", unit_name).unwrap();
    reteive_dependencies(
        &mut dependency,
        unit_object_path,
        dependencies_properties,
        &connection,
        &mut units,
    )?;

    if plain {
        let mut all_children = BTreeSet::new();
        flatit(&dependency, &mut all_children);
        dependency.children.clear();
        dependency.children.append(&mut all_children);
    }

    Ok(dependency)
}

fn flatit(parent: &Dependency, all_children: &mut BTreeSet<Dependency>) {
    for child in parent.children.iter() {
        flatit(child, all_children);
        all_children.insert(child.partial_clone());
    }
}

fn reteive_dependencies(
    dependency: &mut Dependency,
    unit_object_path: &str,
    dependencies_properties: &[&str],
    connection: &Connection,
    units: &mut HashSet<String>,
) -> Result<(), SystemdErrors> {
    let map = fetch_unit_all_properties(connection, unit_object_path)?;

    dependency.state = map.get("ActiveState").into();
    let mut set = BTreeSet::new();
    //let mut set = BTreeSet::new();
    for property_key in dependencies_properties {
        let value = map.get(*property_key);
        let Some(value) = value else {
            warn!("property key {property_key:?} does't exist");
            continue;
        };

        let array: &Array = value.try_into()?;

        for sv in array.iter() {
            let unit_name: &str = sv.try_into()?;

            if units.contains(unit_name) {
                continue;
            }

            set.insert(unit_name);
            units.insert(unit_name.to_string());
        }
    }

    for child_name in set {
        let objet_path = unit_dbus_path_from_name(child_name);

        let mut child_depency = Dependency::new(child_name);

        reteive_dependencies(
            &mut child_depency,
            &objet_path,
            dependencies_properties,
            connection,
            units,
        )?;

        dependency.children.insert(child_depency);
    }

    //units.remove(parent_unit_name);
    Ok(())
}

fn fetch_unit_all_properties(
    connection: &Connection,
    path: &str,
) -> Result<HashMap<String, OwnedValue>, SystemdErrors> {
    let proxy = Proxy::new(
        connection,
        DESTINATION_SYSTEMD,
        path,
        "org.freedesktop.DBus.Properties",
    )?;

    let all_properties: HashMap<String, OwnedValue> =
        match proxy.call("GetAll", &(INTERFACE_SYSTEMD_UNIT)) {
            Ok(m) => m,
            Err(e) => {
                warn!("{e:#?}");
                return Err(e.into());
            }
        };

    Ok(all_properties)
}

pub(super) fn unit_dbus_path_from_name(name: &str) -> String {
    let converted = bus_label_escape(name);
    const PREFIX: &str = "/org/freedesktop/systemd1/unit/";

    let mut out = String::with_capacity(PREFIX.len() + converted.len());
    out.push_str(PREFIX);
    out.push_str(&converted);
    out
}

fn bus_label_escape(name: &str) -> String {
    /* Escapes all chars that D-Bus' object path cannot deal
     * with. Can be reversed with bus_path_unescape(). We special
     * case the empty string. */

    if name.is_empty() {
        return String::from("_");
    }

    let mut r = String::with_capacity(name.len() * 3 + 1);

    /* Escape everything that is not a-zA-Z0-9. We also escape 0-9 if it's the first character */
    for (i, c) in name.bytes().enumerate() {
        if !c.is_ascii_alphabetic() || i != 0 && c.is_ascii_digit() {
            r.push('_');
            r.push(hexchar(c >> 4));
            r.push(hexchar(c));
        } else {
            r.push(c as char);
        }
    }

    r
}

fn hexchar(x: u8) -> char {
    const TABLE: [char; 16] = [
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
    ];

    TABLE[(x & 15) as usize]
}

pub fn get_unit_active_state(
    dbus_level: UnitDBusLevel,
    unit_path: &str,
) -> Result<ActiveState, SystemdErrors> {
    let connection = get_blocking_connection(dbus_level)?;

    let proxy = Proxy::new(
        &connection,
        DESTINATION_SYSTEMD,
        unit_path,
        INTERFACE_SYSTEMD_UNIT,
    )?;

    let active_state: Str = proxy.get_property("ActiveState")?;

    Ok(ActiveState::from(active_state.as_str()))
}

#[derive(Deserialize, Type, PartialEq, Debug)]
pub(crate) struct UnitProcessDeserialize {
    pub(crate) path: String,
    pub(crate) pid: u32,
    pub(crate) name: String,
}

pub(crate) fn retreive_unit_processes(
    dbus_level: UnitDBusLevel,
    unit_name: &str,
) -> Result<Vec<UnitProcessDeserialize>, SystemdErrors> {
    let message = call_systemd_manager_method(dbus_level, METHOD_GET_UNIT_PROCESSES, &(unit_name))?;

    let unit_processes: Vec<UnitProcessDeserialize> = message.body().deserialize()?;

    Ok(unit_processes)
}

pub async fn test(test: &str, level: UnitDBusLevel) -> Result<(), SystemdErrors> {
    info!("Testing {test:?}");

    async fn connection_testing(level: UnitDBusLevel) -> Result<zbus::Connection, SystemdErrors> {
        let connection = get_connection(level).await?;
        debug!("Credentials: {:#?}", connection.peer_credentials().await?);
        debug!("Unique name: {:#?}", connection.unique_name());

        let con_info = format!("{connection:?}");

        let re: regex::Regex = regex::Regex::new("peer:\\s\"(.+)\"").unwrap();

        if let Some(c) = re.captures(&con_info)
            && let Some(m) = c.get(1)
        {
            info!("socket file: {}", m.as_str());
        } else {
            info!("No socket file found!");
        }

        Ok(connection)
    }

    match test {
        "unit_list" => {
            let connection = connection_testing(level).await?;
            let hmap = list_units_list_async(connection).await?;

            debug!("UNIT LIST, bus {level:?}\n{:#?}", hmap);
            info!("UNIT LIST, bus {level:?} TOTAL: {}", hmap.len());
        }
        "unit_file_list" => {
            let connection = connection_testing(level).await?;
            let list = list_unit_files_async(connection, level).await?;

            debug!("UNIT FILE LIST, bus {level:?}\n{list:#?}");
            info!("UNIT FILE LIST, bus {level:?} TOTAL: {}", list.len());
        }
        _ => {
            warn!("No test selected")
        }
    }

    Ok(())
}

pub(super) async fn fetch_unit_interface_properties()
-> Result<BTreeMap<String, Vec<UnitPropertyFetch>>, SystemdErrors> {
    let connection = get_connection(UnitDBusLevel::System).await?;

    let proxy = zbus::Proxy::new(
        &connection,
        DESTINATION_SYSTEMD,
        "/org/freedesktop/systemd1/unit",
        INTERFACE_SYSTEMD_UNIT,
    )
    .await?;

    info!("Proxy {proxy:?}");

    let xml = proxy.introspect().await?;

    let root_node = zbus_xml::Node::from_reader(xml.as_bytes())?;

    let mut map: BTreeMap<String, Vec<UnitPropertyFetch>> = BTreeMap::new();
    let mut set: HashSet<String> = HashSet::new();

    for node_name in root_node
        .nodes()
        .iter()
        .map(|node| node.name())
        .filter_map(|name| name.map(|s| s.to_owned()))
        .filter(|n_name| {
            if let Some(unit_type) = n_name.split("_2e").last()
                && !set.contains(unit_type)
            {
                set.insert(unit_type.to_owned());
                true
            } else {
                false
            }
        })
    {
        collect_properties(&node_name, &connection, &mut map).await?
    }

    info!("Interface len {}", map.len());
    Ok(map)
}

async fn collect_properties(
    unit: &str,
    connection: &zbus::Connection,
    map: &mut BTreeMap<String, Vec<UnitPropertyFetch>>,
) -> Result<(), SystemdErrors> {
    let mut path = String::from("/org/freedesktop/systemd1/unit/");
    path.push_str(unit);

    let proxy = zbus::Proxy::new(
        connection,
        DESTINATION_SYSTEMD,
        path,
        INTERFACE_SYSTEMD_UNIT,
    )
    .await?;

    let xml = proxy.introspect().await?;

    let root_node = zbus_xml::Node::from_reader(xml.as_bytes())?;

    for intf in root_node.interfaces() {
        let list: Vec<_> = intf
            .properties()
            .iter()
            .map(|p| UnitPropertyFetch::new(p))
            .collect();

        map.insert(intf.name().to_string(), list);
    }
    Ok(())
}

pub async fn fetch_unit_properties(
    level: UnitDBusLevel,
    path: &str,
    property_interface: &str,
    property: &str,
) -> Result<OwnedValue, SystemdErrors> {
    let connection = get_connection(level).await?;

    let message = call_method_async(
        &connection,
        DESTINATION_SYSTEMD,
        path,
        INTERFACE_PROPERTIES,
        METHOD_GET,
        &(property_interface, property),
    )
    .await?;

    let body = message.body();

    let property_value: OwnedValue = body.deserialize()?;

    debug!("fetched property value {:?}", property_value);

    Ok(property_value)
}

pub async fn fetch_drop_in_paths(
    level: UnitDBusLevel,
    object_path: &str,
) -> Result<Vec<String>, SystemdErrors> {
    let connection = get_connection(level).await?;

    let unit_info_proxy = ZUnitInfoProxy::builder(&connection)
        .path(object_path)?
        .build()
        .await?;

    let drop_in_paths = unit_info_proxy.drop_in_paths().await?;

    Ok(drop_in_paths)
}

#[allow(unused)]
pub async fn reboot_async(
    connection: zbus::Connection,
    interactive: bool,
) -> Result<(), SystemdErrors> {
    let message = call_method_async(
        &connection,
        "org.freedesktop.login1",
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
        "Reboot",
        &(interactive),
    )
    .await?;

    let body = message.body();

    println!("Reboot {:?}", body);
    Ok(())
}

#[allow(unused)]
pub async fn power_off_async(connection: zbus::Connection) -> Result<(), SystemdErrors> {
    let message = call_method_async(
        &connection,
        DESTINATION_SYSTEMD,
        PATH_SYSTEMD,
        INTERFACE_SYSTEMD_MANAGER,
        "PowerOff",
        &(),
    )
    .await?;

    let body = message.body();

    println!("PowerOff {:?}", body);
    Ok(())
}
