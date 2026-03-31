#![allow(unused_must_use)]
pub mod analyze;
pub mod data;
pub mod enums;
pub mod errors;
mod file;
mod journal;
pub mod journal_data;
#[cfg(not(any(feature = "flatpak", feature = "appimage")))]
pub mod proxy_switcher;
pub mod socket_unit;
pub(crate) mod sysdbus;
pub mod time_handling;

use crate::{
    data::{ListedLoadedUnit, UnitInfo, UnitProcess, UnitPropertySetter},
    enums::{
        ActiveState, CleanOption, DependencyType, DisEnableFlags, KillWho, LoadState,
        StartStopMode, UnitFileStatus, UnitType,
    },
    file::save_text_to_file,
    journal_data::Boot,
    proxy_switcher::PROXY_SWITCHER,
    sysdbus::{
        ListedUnitFile,
        dbus_proxies::{Systemd1ManagerProxy, systemd_manager, systemd_manager_async},
        to_proxy::SysDManagerComLinkProxy,
        watcher::SystemdSignal,
    },
    time_handling::TimestampStyle,
};
use base::{
    enums::UnitDBusLevel,
    file::{
        commander_blocking, create_drop_in_path_file, flatpak_host_file_path, test_flatpak_spawn,
    },
    proxy::{DisEnAbleUnitFiles, DisEnAbleUnitFilesResponse},
};
use enumflags2::{BitFlag, BitFlags};
use errors::SystemdErrors;
use flagset::{FlagSet, flags};
use glib::Quark;
use journal_data::{EventRange, JournalEventChunk};
use std::{
    any::Any,
    collections::{BTreeMap, BTreeSet, HashMap},
    fs::File,
    io::Read,
    sync::OnceLock,
    time::Duration,
};
pub use sysdbus::{
    get_unit_file_state, list_units_description_and_state_async, shut_down_sysd_proxy,
    sysd_proxy_service_name,
    watcher::{SystemdSignalRow, init_signal_watcher},
};
use tokio::{
    runtime::Runtime,
    sync::broadcast::{self, error::RecvError},
    time::timeout,
};
use tracing::{debug, error, info, warn};
use zvariant::OwnedValue;

#[cfg(not(any(feature = "flatpak", feature = "appimage")))]
use crate::sysdbus::to_proxy;

#[cfg(not(any(feature = "flatpak", feature = "appimage")))]
use base::consts::PROXY_SERVICE;

#[derive(Default, Clone, PartialEq, Debug)]
pub enum BootFilter {
    #[default]
    Current,
    All,
    Id(String),
}

#[derive(Clone, Debug)]
// #[allow(unused)]
pub struct SystemdUnitFile {
    pub full_name: String,
    pub status_code: UnitFileStatus,
    pub level: UnitDBusLevel,
    pub file_path: String,
}

#[derive(Debug, Default)]
pub struct UpdatedUnitInfo {
    pub primary: String,
    // pub object_path: String,
    pub description: Option<String>,
    pub load_state: Option<LoadState>,
    pub sub_state: Option<String>,
    pub active_state: Option<ActiveState>,
    pub unit_file_preset: Option<String>,
    pub valid_unit_name: bool,
    pub fragment_path: Option<String>,
    pub enablement_status: Option<UnitFileStatus>,
    pub level: UnitDBusLevel,
}

impl UpdatedUnitInfo {
    fn new(primary: String, level: UnitDBusLevel) -> Self {
        Self {
            primary,
            level,
            ..Default::default()
        }
    }
}

flags! {
    pub enum UnitPropertiesFlags : u8 {
        EnablementStatus,
        ActiveStatus,
        Description,
        LoadState,
        SubState,
        UnitFilePreset,
        FragmentPath,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct UnitProperties(pub FlagSet<UnitPropertiesFlags>);

impl UnitProperties {
    // fn new(flags: impl Into<FlagSet<UnitPropertiesFlags>>) -> UnitProperties {
    //     UnitProperties(flags.into())
    // }
    //
}

pub struct CompleteUnitPropertiesCallParams {
    pub level: UnitDBusLevel,
    pub unit_name: String,
    pub object_path: String,
    pub status: UnitFileStatus,
}

impl CompleteUnitPropertiesCallParams {
    pub fn new(unit: &UnitInfo) -> Self {
        Self::new_params(
            unit.dbus_level(),
            unit.primary(),
            unit.object_path(),
            unit.enable_status(),
        )
    }

    pub fn new_params(
        level: UnitDBusLevel,
        unit_name: String,
        object_path: String,
        status: UnitFileStatus,
    ) -> Self {
        Self {
            level,
            unit_name,
            object_path,
            status,
        }
    }
}

pub fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| Runtime::new().expect("Setting up tokio runtime needs to succeed."))
}

///Try to Start Proxy
#[cfg(not(any(feature = "flatpak", feature = "appimage")))]
pub async fn init_proxy_async(run_mode: base::RunMode) {
    if let Err(e) = sysdbus::init_proxy_async(run_mode).await {
        error!("Fail starting Proxy. Error {e:?}");
    }
}

#[cfg(not(any(feature = "flatpak", feature = "appimage")))]
pub fn shut_down() {
    sysdbus::shut_down_sysd_proxy();
}

#[derive(Debug)]
pub enum ListUnitResponse {
    Loaded(UnitDBusLevel, Vec<ListedLoadedUnit>),
    File(UnitDBusLevel, Vec<ListedUnitFile>),
}

impl ListUnitResponse {
    pub fn r_len(&self) -> (usize, usize) {
        match self {
            ListUnitResponse::Loaded(_, items) => (items.len(), 0),
            ListUnitResponse::File(_, items) => (0, items.len()),
        }
    }

    pub fn t_len(&self) -> usize {
        match self {
            ListUnitResponse::Loaded(_, lunits) => lunits.len(),
            ListUnitResponse::File(_, items) => items.len(),
        }
    }
    pub fn update_flags(&self) -> FlagSet<UnitPropertiesFlags> {
        match self {
            ListUnitResponse::Loaded(_, _) => {
                UnitPropertiesFlags::EnablementStatus
                    | UnitPropertiesFlags::Description
                    | UnitPropertiesFlags::LoadState
                    | UnitPropertiesFlags::SubState
                    | UnitPropertiesFlags::UnitFilePreset
            }

            ListUnitResponse::File(_, _) => {
                UnitPropertiesFlags::ActiveStatus
                    | UnitPropertiesFlags::Description
                    | UnitPropertiesFlags::LoadState
                    | UnitPropertiesFlags::SubState
                    | UnitPropertiesFlags::UnitFilePreset
            }
        }
    }
}

pub async fn list_loaded_units(level: UnitDBusLevel) -> Result<ListUnitResponse, SystemdErrors> {
    let v = systemd_manager_async(level).await?.list_units().await?;
    Ok(ListUnitResponse::Loaded(level, v))
}

pub async fn list_loaded_units_by_patterns(
    level: UnitDBusLevel,
    patterns: &[&str],
) -> Result<ListUnitResponse, SystemdErrors> {
    let v = systemd_manager_async(level)
        .await?
        .list_units_by_patterns(&[], patterns)
        .await?;
    Ok(ListUnitResponse::Loaded(level, v))
}

pub async fn list_loaded_units_timers(
    level: UnitDBusLevel,
) -> Result<ListUnitResponse, SystemdErrors> {
    list_loaded_units_by_patterns(level, &["*.timer"]).await
}

pub async fn list_loaded_units_sockets(
    level: UnitDBusLevel,
) -> Result<ListUnitResponse, SystemdErrors> {
    list_loaded_units_by_patterns(level, &["*.socket"]).await
}

pub async fn list_loaded_units_paths(
    level: UnitDBusLevel,
) -> Result<ListUnitResponse, SystemdErrors> {
    list_loaded_units_by_patterns(level, &["*.path"]).await
}

pub async fn list_loaded_units_automounts(
    level: UnitDBusLevel,
) -> Result<ListUnitResponse, SystemdErrors> {
    list_loaded_units_by_patterns(level, &["*.automount"]).await
}

pub async fn list_unit_files(level: UnitDBusLevel) -> Result<ListUnitResponse, SystemdErrors> {
    let v = systemd_manager_async(level)
        .await?
        .list_unit_files()
        .await?;
    Ok(ListUnitResponse::File(level, v))
}

pub async fn list_unit_files_by_patterns(
    level: UnitDBusLevel,
    patterns: &[&str],
) -> Result<ListUnitResponse, SystemdErrors> {
    let v = systemd_manager_async(level)
        .await?
        .list_unit_files_by_patterns(&[], patterns)
        .await?;
    Ok(ListUnitResponse::File(level, v))
}

pub async fn list_unit_files_timers(
    level: UnitDBusLevel,
) -> Result<ListUnitResponse, SystemdErrors> {
    list_unit_files_by_patterns(level, &["*.timer"]).await
}

pub async fn list_unit_files_sockets(
    level: UnitDBusLevel,
) -> Result<ListUnitResponse, SystemdErrors> {
    list_unit_files_by_patterns(level, &["*.socket"]).await
}

pub async fn list_unit_files_paths(
    level: UnitDBusLevel,
) -> Result<ListUnitResponse, SystemdErrors> {
    list_unit_files_by_patterns(level, &["*.path"]).await
}

pub async fn list_unit_files_automounts(
    level: UnitDBusLevel,
) -> Result<ListUnitResponse, SystemdErrors> {
    list_unit_files_by_patterns(level, &["*.automount"]).await
}

pub async fn complete_unit_information(
    units: &[CompleteUnitPropertiesCallParams],
) -> Result<Vec<UpdatedUnitInfo>, SystemdErrors> {
    sysdbus::complete_unit_information(units).await
}

pub async fn complete_single_unit_information(
    primary_name: String,
    level: UnitDBusLevel,
    object_path: String,
    status: UnitFileStatus,
) -> Result<Vec<UpdatedUnitInfo>, SystemdErrors> {
    let units = [CompleteUnitPropertiesCallParams::new_params(
        level,
        primary_name,
        object_path,
        status,
    )];
    sysdbus::complete_unit_information(&units).await
}

/// Takes a unit name as input and attempts to start it
/// # returns
/// job_path
pub fn start_unit(
    level: UnitDBusLevel,
    unit_name: &str,
    mode: StartStopMode,
) -> Result<String, SystemdErrors> {
    runtime()
        .block_on(async move { restartstop_unit(level, unit_name, mode, ReStartStop::Start).await })
}

/// Takes a unit name as input and attempts to stop it.
pub fn stop_unit(
    level: UnitDBusLevel,
    unit_name: &str,
    mode: StartStopMode,
) -> Result<String, SystemdErrors> {
    runtime()
        .block_on(async move { restartstop_unit(level, unit_name, mode, ReStartStop::Stop).await })
}

#[derive(Debug)]
pub enum ReStartStop {
    Start,
    Stop,
    Restart,
    ReloadUnit,
}

impl ReStartStop {
    fn use_proxy(&self) -> bool {
        match self {
            ReStartStop::Start => PROXY_SWITCHER.start(),
            ReStartStop::Stop => PROXY_SWITCHER.stop(),
            ReStartStop::Restart => PROXY_SWITCHER.restart(),
            ReStartStop::ReloadUnit => PROXY_SWITCHER.reload_unit(),
        }
    }

    async fn action<'a>(
        &self,
        proxy: &SysDManagerComLinkProxy<'a>,
        unit_name: &str,
        mode: StartStopMode,
    ) -> Result<String, SystemdErrors> {
        let path = match self {
            ReStartStop::Start => proxy.start_unit(unit_name, mode.as_str()).await?,
            ReStartStop::Stop => proxy.stop_unit(unit_name, mode.as_str()).await?,
            ReStartStop::Restart => proxy.restart_unit(unit_name, mode.as_str()).await?,
            ReStartStop::ReloadUnit => proxy.reload_unit(unit_name, mode.as_str()).await?,
        };

        Ok(path.to_string())
    }

    async fn systemd_action<'a>(
        &self,
        manager: &Systemd1ManagerProxy<'a>,
        unit_name: &str,
        mode: StartStopMode,
    ) -> Result<String, SystemdErrors> {
        let path = match self {
            ReStartStop::Start => manager.start_unit(unit_name, mode.as_str()).await?,
            ReStartStop::Stop => manager.stop_unit(unit_name, mode.as_str()).await?,
            ReStartStop::Restart => manager.restart_unit(unit_name, mode.as_str()).await?,
            ReStartStop::ReloadUnit => manager.reload_unit(unit_name, mode.as_str()).await?,
        };

        Ok(path.to_string())
    }
}

pub async fn restartstop_unit(
    level: UnitDBusLevel,
    unit_name: &str,
    mode: StartStopMode,
    action: ReStartStop,
) -> Result<String, SystemdErrors> {
    let watcher = init_signal_watcher();
    let job = restartstop_unit_call(level, unit_name, mode, &action).await?;
    let job_id = job_number(&job).ok_or("Invalid Job Id for job: {job}")?;

    let duration = Duration::from_secs(10);
    timeout(duration, wait_job_removed(job_id, watcher))
        .await
        .map_err(|_err| SystemdErrors::Timeout(duration))
        .and_then(|res| res.map(|_| job))
        .inspect(|_job| {
            #[cfg(not(any(feature = "flatpak", feature = "appimage")))]
            if matches!(action, ReStartStop::Start | ReStartStop::Restart)
                && unit_name.starts_with(PROXY_SERVICE)
            {
                to_proxy::start_heart_beat()
            }
        })
}

const DONE: &str = "done";
const SKIPPED: &str = "skipped";
const CANCELED: &str = "canceled";
const TIMEOUT: &str = "timeout";
const FAILED: &str = "failed";
const DEPENDENCY: &str = "dependency";
const INVALID: &str = "invalid";

async fn wait_job_removed(
    job_id: u32,
    mut watcher: broadcast::Receiver<SystemdSignalRow>,
) -> Result<(), SystemdErrors> {
    loop {
        match watcher.recv().await {
            Ok(x) => {
                if let SystemdSignal::JobRemoved(id, _, _unit, result) = x.signal
                    && id == job_id
                {
                    match result.as_str() {
                        DONE => {
                            break;
                        }
                        CANCELED => return Err(SystemdErrors::JobRemoved(CANCELED.to_owned())),
                        TIMEOUT => return Err(SystemdErrors::JobRemoved(TIMEOUT.to_owned())),
                        FAILED => return Err(SystemdErrors::JobRemoved(FAILED.to_owned())),
                        DEPENDENCY => return Err(SystemdErrors::JobRemoved(DEPENDENCY.to_owned())),
                        SKIPPED => return Err(SystemdErrors::JobRemoved(SKIPPED.to_owned())),
                        INVALID => return Err(SystemdErrors::JobRemoved(INVALID.to_owned())),
                        unkown_result => {
                            warn!("Unknown JobRemoved result {unkown_result}");
                        }
                    }
                }
            }
            Err(RecvError::Lagged(lag)) => info!("Lagged {lag:?}"),
            Err(err) => {
                warn!("Recev Err {err:?}");
                return Err(SystemdErrors::JobRemoved(format!("{err:?}")));
            }
        }
    }
    Ok(())
}

fn job_number(job: &str) -> Option<u32> {
    job.rsplit_once('/').and_then(|(_, job_id)| {
        job_id
            .parse::<u32>()
            .inspect_err(|err| warn!("Job {err:?}"))
            .ok()
    })
}

async fn restartstop_unit_call(
    level: UnitDBusLevel,
    unit_name: &str,
    mode: StartStopMode,
    action: &ReStartStop,
) -> Result<String, SystemdErrors> {
    #[cfg(not(any(feature = "flatpak", feature = "appimage")))]
    match level {
        UnitDBusLevel::System | UnitDBusLevel::Both => {
            use crate::sysdbus::to_proxy::get_proxy_async;

            let proxy = get_proxy_async().await?;
            if action.use_proxy() && !unit_name.starts_with(PROXY_SERVICE) {
                // proxy_call_blocking!(restart_unit, unit_name, mode.as_str())

                match action.action(&proxy, unit_name, mode).await {
                    Ok(ok) => Ok(ok),
                    Err(SystemdErrors::ZFdoServiceUnknowm(msg)) => {
                        warn!("Async ServiceUnkown: {:?} Function: {:?}", msg, action);
                        to_proxy::lazy_start_proxy_async().await;
                        action.action(&proxy, unit_name, mode).await
                    }
                    Err(err) => Err(err),
                }
            } else {
                let manager = sysdbus::dbus_proxies::system_manager_system_async().await?;
                action.systemd_action(manager, unit_name, mode).await
            }
        }

        UnitDBusLevel::UserSession => {
            let manager = sysdbus::dbus_proxies::system_manager_user_session_async().await?;
            action.systemd_action(manager, unit_name, mode).await
        }
    }

    #[cfg(any(feature = "flatpak", feature = "appimage"))]
    {
        let manager = sysdbus::dbus_proxies::system_manager_async(level).await?;
        action.systemd_action(manager, unit_name, mode).await
    }
}

pub fn disenable_unit_file(
    primary_name: &str,
    level: UnitDBusLevel,
    enable_status: UnitFileStatus,
    expected_status: UnitFileStatus,
) -> Result<DisEnAbleUnitFilesResponse, SystemdErrors> {
    match expected_status {
        UnitFileStatus::Enabled | UnitFileStatus::EnabledRuntime => enable_unit_file(
            level,
            primary_name,
            DisEnableFlags::SdSystemdUnitForce.into(),
        ),
        _ => {
            let flags: BitFlags<DisEnableFlags> = if enable_status.is_runtime() {
                DisEnableFlags::SdSystemdUnitRuntime.into()
            } else {
                DisEnableFlags::empty()
            };

            disable_unit_file(level, primary_name, flags)
        }
    }
}

pub fn enable_unit_file(
    level: UnitDBusLevel,
    unit_file: &str,
    flags: BitFlags<DisEnableFlags>,
) -> Result<DisEnAbleUnitFilesResponse, SystemdErrors> {
    #[cfg(not(any(feature = "flatpak", feature = "appimage")))]
    match level {
        UnitDBusLevel::System | UnitDBusLevel::Both => {
            if proxy_switcher::PROXY_SWITCHER.enable_unit_file() {
                proxy_call_blocking!(
                    enable_unit_files_with_flags,
                    &[unit_file],
                    flags.bits_c() as u64
                )
            } else {
                systemd_manager()
                    .enable_unit_files_with_flags(&[unit_file], flags.bits_c() as u64)
                    .map_err(|err| err.into())
            }
        }
        UnitDBusLevel::UserSession => sysdbus::dbus_proxies::systemd_manager_session()
            .enable_unit_files_with_flags(&[unit_file], flags.bits_c() as u64)
            .map_err(|err| err.into()),
    }

    #[cfg(any(feature = "flatpak", feature = "appimage"))]
    {
        use crate::sysdbus::dbus_proxies::systemd_manager_blocking;
        systemd_manager_blocking(level)
            .enable_unit_files_with_flags(&[unit_file], flags.bits_c() as u64)
            .map_err(|err| err.into())
    }
}

pub fn disable_unit_file(
    level: UnitDBusLevel,
    unit_file: &str,
    flags: BitFlags<DisEnableFlags>,
) -> Result<DisEnAbleUnitFilesResponse, SystemdErrors> {
    info!("{:?} {} {:?}", level, unit_file, flags.bits_c());
    #[cfg(not(any(feature = "flatpak", feature = "appimage")))]
    match level {
        UnitDBusLevel::System | UnitDBusLevel::Both => {
            if proxy_switcher::PROXY_SWITCHER.disable_unit_file() {
                proxy_call_blocking!(
                    disable_unit_files_with_flags,
                    &[unit_file],
                    flags.bits_c() as u64
                )
            } else {
                systemd_manager()
                    .disable_unit_files_with_flags_and_install_info(
                        &[unit_file],
                        flags.bits_c() as u64,
                    )
                    .map_err(|err| err.into())
            }
        }
        UnitDBusLevel::UserSession => sysdbus::dbus_proxies::systemd_manager_session()
            .disable_unit_files_with_flags_and_install_info(&[unit_file], flags.bits_c() as u64)
            .map_err(|err| err.into()),
    }

    #[cfg(any(feature = "flatpak", feature = "appimage"))]
    {
        use crate::sysdbus::dbus_proxies::systemd_manager_blocking;
        systemd_manager_blocking(level)
            .disable_unit_files_with_flags_and_install_info(&[unit_file], flags.bits_c() as u64)
            .map_err(|err| err.into())
    }
}

pub async fn fetch_drop_in_paths(
    level: UnitDBusLevel,
    unit_name: &str,
) -> Result<Vec<String>, SystemdErrors> {
    sysdbus::fetch_drop_in_paths(level, unit_name).await
}
/// Read the unit file and return it's contents so that we can display it
pub fn fetch_unit_file_content(
    file_path: Option<&str>,
    unit_primary_name: &str,
) -> Result<String, SystemdErrors> {
    let Some(file_path) = file_path else {
        warn!("No file path for {:?}", unit_primary_name);
        return Ok(String::new());
    };

    file_open_get_content(file_path, unit_primary_name)
}

#[allow(unused)]
fn flatpak_file_open_get_content(
    file_path: &str,
    unit_primary_name: &str,
) -> Result<String, SystemdErrors> {
    file_open_get_content(file_path, unit_primary_name).or_else(|e| {
        info!("Trying to fetch file content through 'cat' command, because {e:?}");
        file_open_get_content_cat(file_path, unit_primary_name)
    })
}

fn file_open_get_content_cat(
    file_path: &str,
    unit_primary_name: &str,
) -> Result<String, SystemdErrors> {
    info!(
        "Flatpak Fetching file content Unit: {} File \"{file_path}\"",
        unit_primary_name
    );
    //Use the REAL path because try to acceess through the 'cat' command
    commander_output(&["cat", file_path], None)
        .map(|cat_output| String::from_utf8_lossy(&cat_output.stdout).to_string())
        .inspect_err(|e| warn!("Can't open file {file_path:?} with 'cat' command, reason: {e:?}"))
}

fn file_open_get_content(
    file_path: &str,
    unit_primary_name: &str,
) -> Result<String, SystemdErrors> {
    //To get the relative path from a Flatpak
    let file_path = flatpak_host_file_path(file_path);

    info!(
        "Fetching file content Unit: {} File: {}",
        unit_primary_name,
        file_path.display()
    );

    let mut file = File::open(&file_path).map_err(|e| {
        warn!(
            "Can't open file \"{}\", reason: {e} {:?}",
            file_path.display(),
            e.kind()
        );
        SystemdErrors::IoError(e)
    })?;

    let mut output = String::new();
    let _ = file.read_to_string(&mut output);

    Ok(output)
}

/// Obtains the journal log for the given unit.
pub fn get_unit_journal(
    primary_name: String,
    level: UnitDBusLevel,
    boot_filter: BootFilter,
    range: EventRange,
    message_max_char: usize,
    timestamp_style: TimestampStyle,
) -> Result<JournalEventChunk, SystemdErrors> {
    journal::get_unit_journal_events(
        primary_name,
        level,
        boot_filter,
        range,
        message_max_char,
        timestamp_style,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn get_unit_journal_continuous(
    unit_name: String,
    level: UnitDBusLevel,
    range: EventRange,
    journal_continuous_receiver: std::sync::mpsc::Receiver<()>,
    sender: std::sync::mpsc::Sender<JournalEventChunk>,
    message_max_char: usize,
    timestamp_style: TimestampStyle,
    check_for_new_journal_entry: fn(),
) {
    if let Err(err) = journal::get_unit_journal_events_continuous(
        unit_name,
        level,
        range,
        journal_continuous_receiver,
        sender,
        message_max_char,
        timestamp_style,
        check_for_new_journal_entry,
    ) {
        warn!(
            "Journal TailError type: {:?}  Error: {:?}",
            err.type_id(),
            err
        );
    } else {
        warn!("Ok journal tail thread finished");
    }
}

pub fn list_boots() -> Result<Vec<Boot>, SystemdErrors> {
    journal::list_boots()
}

pub fn fetch_last_time() -> Result<u64, SystemdErrors> {
    journal::fetch_last_time()
}

pub fn commander_output(
    prog_n_args: &[&str],
    environment_variables: Option<&[(&str, &str)]>,
) -> Result<std::process::Output, SystemdErrors> {
    match commander_blocking(prog_n_args, environment_variables).output() {
        Ok(output) => {
            if cfg!(feature = "flatpak") {
                info!("Command Exit status: {}", output.status);

                if !output.status.success() {
                    warn!("Flatpak mode, command line did not succeed, please investigate.");
                    error!("Command exit status: {}", output.status);
                    info!(
                        "{}",
                        String::from_utf8(output.stdout).expect("from_utf8 failed")
                    );
                    error!(
                        "{}",
                        String::from_utf8(output.stderr).expect("from_utf8 failed")
                    );
                    let vec = prog_n_args.iter().map(|s| s.to_string()).collect();
                    return Err(SystemdErrors::CmdNoFreedesktopFlatpakPermission(
                        Some(vec),
                        None,
                    ));
                }
            }
            Ok(output)
        }
        Err(err) => {
            error!("commander_output {err}");

            match test_flatpak_spawn() {
                Ok(()) => Err(SystemdErrors::IoError(err)),
                Err(e1) => {
                    error!("commander_output e1 {e1}");
                    Err(SystemdErrors::CmdNoFlatpakSpawn)
                }
            }
        }
    }
}

pub fn generate_file_uri(file_path: &str) -> String {
    let flatpak_host_file_path = flatpak_host_file_path(file_path);
    format!("file://{}", flatpak_host_file_path.display())
}

pub fn fetch_system_info() -> Result<Vec<(UnitType, String, String)>, SystemdErrors> {
    //TODO check with Session (user)
    sysdbus::fetch_system_info(UnitDBusLevel::System)
}

pub fn fetch_system_unit_info_native(
    unit: &UnitInfo,
) -> Result<Vec<(UnitType, String, OwnedValue)>, SystemdErrors> {
    let level = unit.dbus_level();
    let unit_type: UnitType = unit.unit_type();
    let object_path = unit.object_path();

    sysdbus::fetch_system_unit_info_native(level, &object_path, unit_type)
}

pub fn fetch_system_unit_info_native_map(
    unit: &UnitInfo,
) -> Result<HashMap<String, OwnedValue>, SystemdErrors> {
    let level = unit.dbus_level();
    let unit_type: UnitType = unit.unit_type();
    let object_path = unit.object_path();

    sysdbus::fetch_system_unit_info_native_map(level, &object_path, unit_type)
}

/* fn get_unit_path(unit: &UnitInfo) -> String {
    match unit.object_path() {
        Some(s) => s,
        None => {
            let object_path = sysdbus::unit_dbus_path_from_name(&unit.primary());
            unit.set_object_path(object_path.clone());
            object_path
        }
    }
}
 */
pub fn fetch_unit(
    level: UnitDBusLevel,
    unit_primary_name: &str,
) -> Result<UnitInfo, SystemdErrors> {
    sysdbus::fetch_unit(level, unit_primary_name)
}

pub fn kill_unit(
    level: UnitDBusLevel,
    primary_name: &str,
    who: KillWho,
    signal: i32,
) -> Result<(), SystemdErrors> {
    sysdbus::kill_unit(level, primary_name, who, signal)
}

pub fn freeze_unit(params: Option<(UnitDBusLevel, String)>) -> Result<(), SystemdErrors> {
    if let Some((_level, primary_name)) = params {
        #[cfg(not(any(feature = "flatpak", feature = "appimage")))]
        match _level {
            UnitDBusLevel::System | UnitDBusLevel::Both => {
                if proxy_switcher::PROXY_SWITCHER.freeze() {
                    proxy_call_blocking!(freeze_unit, &primary_name)
                } else {
                    let proxy = systemd_manager();
                    proxy.freeze_unit(&primary_name)?;
                    Ok(())
                }
            }
            UnitDBusLevel::UserSession => sysdbus::dbus_proxies::systemd_manager_session()
                .freeze_unit(&primary_name)
                .map_err(|err| err.into()),
        }

        #[cfg(any(feature = "flatpak", feature = "appimage"))]
        {
            let proxy = systemd_manager();
            proxy.freeze_unit(&primary_name)?;
            Ok(())
        }
    } else {
        Err(SystemdErrors::NoUnit)
    }
}

pub fn thaw_unit(params: Option<(UnitDBusLevel, String)>) -> Result<(), SystemdErrors> {
    let Some((level, primary_name)) = params else {
        return Err(SystemdErrors::NoUnit);
    };

    #[cfg(not(any(feature = "flatpak", feature = "appimage")))]
    match level {
        UnitDBusLevel::System | UnitDBusLevel::Both => {
            if proxy_switcher::PROXY_SWITCHER.thaw() {
                proxy_call_blocking!(thaw_unit, &primary_name)
            } else {
                let proxy = systemd_manager();
                proxy.thaw_unit(&primary_name)?;
                Ok(())
            }
        }
        UnitDBusLevel::UserSession => sysdbus::dbus_proxies::systemd_manager_session()
            .thaw_unit(&primary_name)
            .map_err(|err| err.into()),
    }

    #[cfg(any(feature = "flatpak", feature = "appimage"))]
    {
        use crate::sysdbus::dbus_proxies::systemd_manager_blocking;
        let proxy = systemd_manager_blocking(level);
        proxy.thaw_unit(&primary_name)?;
        Ok(())
    }
}

pub fn reload_unit(
    level: UnitDBusLevel,
    primary_name: &str,
    mode: StartStopMode,
) -> Result<String, SystemdErrors> {
    sysdbus::reload_unit(level, primary_name, mode.as_str())
}

pub fn queue_signal_unit(
    level: UnitDBusLevel,
    primary_name: &str,
    who: KillWho,
    signal: i32,
    value: i32,
) -> Result<(), SystemdErrors> {
    sysdbus::queue_signal_unit(level, primary_name, who, signal, value)
}

pub fn clean_unit(
    level: UnitDBusLevel,
    unit_name: &str,
    what: &[String],
) -> Result<(), SystemdErrors> {
    //just send all if seleted
    let mut what_peekable = what
        .iter()
        .filter(|c_op| *c_op == CleanOption::All.code())
        .peekable();

    let clean_what: Vec<&str> = if what_peekable.peek().is_some() {
        vec![CleanOption::All.code()]
    } else {
        what.iter().map(|s| s.as_str()).collect()
    };

    #[cfg(not(any(feature = "flatpak", feature = "appimage")))]
    match level {
        UnitDBusLevel::System | UnitDBusLevel::Both => {
            if proxy_switcher::PROXY_SWITCHER.clean() {
                proxy_call_blocking!(clean_unit, unit_name, &clean_what)
            } else {
                let proxy = systemd_manager();
                proxy
                    .clean_unit(unit_name, &clean_what)
                    .map_err(|err| err.into())
            }
        }
        UnitDBusLevel::UserSession => sysdbus::dbus_proxies::systemd_manager_session()
            .clean_unit(unit_name, &clean_what)
            .map_err(|err| err.into()),
    }

    #[cfg(any(feature = "flatpak", feature = "appimage"))]
    {
        use crate::sysdbus::dbus_proxies::systemd_manager_blocking;

        systemd_manager_blocking(level)
            .clean_unit(unit_name, &clean_what)
            .map_err(|err| err.into())
    }
}

pub fn mask_unit_files(
    level: UnitDBusLevel,
    primary_name: &str,
    runtime: bool,
    force: bool,
) -> Result<Vec<DisEnAbleUnitFiles>, SystemdErrors> {
    sysdbus::mask_unit_files(level, &[primary_name], runtime, force)
}

pub fn preset_unit_files(
    level: UnitDBusLevel,
    primary_name: &str,
    runtime: bool,
    force: bool,
) -> Result<DisEnAbleUnitFilesResponse, SystemdErrors> {
    sysdbus::preset_unit_file(level, &[primary_name], runtime, force)
}

pub fn reenable_unit_file(
    level: UnitDBusLevel,
    primary_name: &str,
    runtime: bool,
    force: bool,
) -> Result<DisEnAbleUnitFilesResponse, SystemdErrors> {
    sysdbus::reenable_unit_file(level, &[primary_name], runtime, force)
}

pub fn unmask_unit_files(
    level: UnitDBusLevel,
    primary_name: &str,
    runtime: bool,
) -> Result<Vec<DisEnAbleUnitFiles>, SystemdErrors> {
    sysdbus::unmask_unit_files(level, &[primary_name], runtime)
}

pub fn link_unit_files(
    dbus_level: UnitDBusLevel,
    unit_file: &str,
    runtime: bool,
    force: bool,
) -> Result<Vec<DisEnAbleUnitFiles>, SystemdErrors> {
    sysdbus::link_unit_files(dbus_level, &[unit_file], runtime, force)
}

pub async fn daemon_reload(level: UnitDBusLevel) -> Result<(), SystemdErrors> {
    let mut watcher = init_signal_watcher();
    daemon_reload_core(level).await?;

    let mut wait_reload = async || {
        loop {
            match watcher.recv().await {
                Ok(x) => {
                    if let SystemdSignal::Reloading(active) = x.signal {
                        if active {
                            info!("Reloading!");
                        } else {
                            info!("Reload Finised");
                            break;
                        }
                    }
                }
                Err(RecvError::Lagged(lag)) => info!("Lagged {lag:?}"),
                Err(err) => {
                    warn!("Recev Err {err:?}");
                    break;
                }
            }
        }
    };

    let duration = Duration::from_secs(10);
    match timeout(duration, wait_reload()).await {
        Ok(_) => Ok(()),
        Err(_err) => Err(SystemdErrors::Timeout(duration)),
    }
}

async fn daemon_reload_core(level: UnitDBusLevel) -> Result<(), SystemdErrors> {
    #[cfg(not(any(feature = "flatpak", feature = "appimage")))]
    if level.user_session() || !proxy_switcher::PROXY_SWITCHER.reload() {
        info!("Reloading Daemon - Direct");
        systemd_manager_async(level)
            .await?
            .reload()
            .await
            .map_err(|err| err.into())
    } else {
        info!("Reloading Daemon - Proxy");
        proxy_call_async!(reload)
    }

    #[cfg(any(feature = "flatpak", feature = "appimage"))]
    {
        info!("Reloading Daemon - Direct");
        systemd_manager_async(level)
            .await?
            .reload()
            .await
            .map_err(|err| err.into())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Dependency {
    pub unit_name: String,
    pub state: ActiveState,
    pub children: BTreeSet<Dependency>,
}

impl Dependency {
    pub fn new(unit_name: &str) -> Self {
        Self {
            unit_name: unit_name.to_string(),
            state: ActiveState::Unknown,
            children: BTreeSet::new(),
        }
    }

    fn partial_clone(&self) -> Dependency {
        Self {
            unit_name: self.unit_name.clone(),
            state: self.state,
            children: BTreeSet::new(),
        }
    }
}

impl Ord for Dependency {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.unit_name.cmp(&other.unit_name)
    }
}

impl PartialOrd for Dependency {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub fn fetch_unit_dependencies(
    level: UnitDBusLevel,
    primary_name: &str,
    object_path: &str,
    dependency_type: DependencyType,
    plain: bool,
) -> Result<Dependency, SystemdErrors> {
    sysdbus::unit_get_dependencies(level, primary_name, object_path, dependency_type, plain)
}

pub fn get_unit_active_state(
    level: UnitDBusLevel,
    primary_name: &str,
) -> Result<ActiveState, SystemdErrors> {
    let object_path = sysdbus::unit_dbus_path_from_name(primary_name);

    sysdbus::get_unit_active_state(level, &object_path)
}

pub fn retreive_unit_processes(
    unit: &UnitInfo,
) -> Result<BTreeMap<String, BTreeSet<UnitProcess>>, SystemdErrors> {
    let level = unit.dbus_level();

    let unit_processes = sysdbus::retreive_unit_processes(level, &unit.primary())?;

    // let mut unit_processes_out = Vec::with_capacity(unit_processes.len());
    let mut unit_processes_map: BTreeMap<String, BTreeSet<UnitProcess>> = BTreeMap::new();
    for unit_process in unit_processes {
        let unit_process = {
            let Some(unit_name) = unit_process.path.rsplit_once('/').map(|a| a.1) else {
                warn!("No unit name for path {:?}", unit_process.path);
                continue;
            };

            let unit_name_idx = unit_process.path.len() - unit_name.len();

            UnitProcess {
                path: unit_process.path,
                pid: unit_process.pid,
                name: unit_process.name,
                unit_name: unit_name_idx,
            }
        };

        if let Some(set) = unit_processes_map.get_mut(unit_process.unit_name()) {
            set.insert(unit_process);
        } else {
            let mut set = BTreeSet::new();
            let key = unit_process.unit_name().to_string();
            set.insert(unit_process);
            unit_processes_map.insert(key, set);
        }
    }

    Ok(unit_processes_map)
}

pub async fn test(test_name: &str, level: UnitDBusLevel) {
    info!("Testing {test_name:?}");

    if let Err(error) = sysdbus::test(test_name, level).await {
        error!("{error:#?}");
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct UnitPropertyFetch {
    pub name: String,
    pub signature: String,
    pub access: String,
}

impl UnitPropertyFetch {
    fn new(p: &zbus_xml::Property) -> Self {
        let access = match p.access() {
            zbus_xml::PropertyAccess::Read => "read",
            zbus_xml::PropertyAccess::Write => "write",
            zbus_xml::PropertyAccess::ReadWrite => "readwrite",
        };

        UnitPropertyFetch {
            name: p.name().to_string(),
            signature: p.ty().to_string(),
            access: access.to_string(),
        }
    }
}

pub async fn fetch_unit_interface_properties()
-> Result<BTreeMap<String, Vec<UnitPropertyFetch>>, SystemdErrors> {
    sysdbus::fetch_unit_interface_properties().await
}

pub async fn fetch_unit_properties(
    level: UnitDBusLevel,
    unit_primary_name: &str,
    path: &str,
    unit_properties: UnitProperties,
    properties: Vec<(UnitType, &str, Quark)>,
) -> Result<Vec<UnitPropertySetter>, SystemdErrors> {
    sysdbus::fetch_unit_properties(level, unit_primary_name, path, unit_properties, properties)
        .await
}

pub fn fetch_unit_property_blocking(
    level: UnitDBusLevel,
    unit_primary_name: &str,
    unit_type: UnitType,
    unit_property: &str,
) -> Result<OwnedValue, SystemdErrors> {
    sysdbus::fetch_unit_property_blocking(level, unit_primary_name, unit_type, unit_property)
}

pub async fn create_drop_in(
    user_session: bool,
    runtime: bool,
    unit_name: &str,
    file_name: &str,
    content: &str,
) -> Result<String, SystemdErrors> {
    let file_path = create_drop_in_path_file(unit_name, runtime, user_session, file_name)?;

    #[cfg(not(any(feature = "flatpak", feature = "appimage")))]
    let result = if user_session || !proxy_switcher::PROXY_SWITCHER.create_dropin() {
        file::create_drop_in(user_session, &file_path, content).await
    } else {
        proxy_call_async!(create_drop_in, runtime, unit_name, &file_path, content)
    };

    #[cfg(any(feature = "flatpak", feature = "appimage"))]
    let result = file::create_drop_in(user_session, &file_path, content).await;

    result.map(|_| file_path)
}

pub async fn save_file(
    level: UnitDBusLevel,
    file_path: &str,
    content: &str,
) -> Result<u64, SystemdErrors> {
    info!("Saving file {file_path:?}");

    let user_session = level.user_session();
    //TODO check the case of /run

    #[cfg(not(any(feature = "flatpak", feature = "appimage")))]
    if user_session || !proxy_switcher::PROXY_SWITCHER.save_file() {
        save_text_to_file(file_path, content, user_session).await
    } else {
        proxy_call_async!(save_file, file_path, content)
    }

    #[cfg(any(feature = "flatpak", feature = "appimage"))]
    save_text_to_file(file_path, content, user_session).await
}

pub async fn revert_unit_file_full(
    level: UnitDBusLevel,
    unit_name: &str,
) -> Result<Vec<DisEnAbleUnitFiles>, SystemdErrors> {
    info!("Reverting unit file {unit_name:?}");

    #[cfg(not(any(feature = "flatpak", feature = "appimage")))]
    if level.user_session() || !proxy_switcher::PROXY_SWITCHER.revert_unit_file() {
        systemd_manager_async(level)
            .await?
            .revert_unit_files(&[unit_name])
            .await
            .map_err(|err| err.into())
    } else {
        proxy_call_async!(revert_unit_files, &[unit_name])
    }

    #[cfg(any(feature = "flatpak", feature = "appimage"))]
    {
        systemd_manager_async(level)
            .await?
            .revert_unit_files(&[unit_name])
            .await
            .map_err(|err| err.into())
    }
}
pub async fn fill_list_unit_files(
    level: UnitDBusLevel,
) -> Result<Vec<SystemdUnitFile>, SystemdErrors> {
    sysdbus::fill_list_unit_files(level).await
}
