pub mod analyze;
pub mod data;
pub mod errors;
pub mod journal;
pub mod journal_data;
mod sysdbus;

use std::{
    any::Any,
    borrow::Cow,
    collections::{BTreeMap, BTreeSet, HashMap},
    fs::{self, File},
    io::{ErrorKind, Read, Write},
    process::{Command, Stdio},
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

use data::{DisEnAbleUnitFiles, UnitInfo, UnitProcess};
use enums::{
    ActiveState, CleanOption, DependencyType, DisEnableFlags, EnablementStatus, KillWho,
    StartStopMode, UnitDBusLevel, UnitType,
};
use errors::SystemdErrors;
use gtk::glib::GString;
use journal::Boot;
use journal_data::{EventRange, JournalEventChunk};
use log::{error, info, warn};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use zvariant::{OwnedObjectPath, OwnedValue};

pub mod enums;

const FLATPAK_SPAWN: &str = "flatpak-spawn";

pub fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| Runtime::new().expect("Setting up tokio runtime needs to succeed."))
}

#[cfg(feature = "flatpak")]
const IS_FLATPAK_MODE: bool = true;

#[cfg(not(feature = "flatpak"))]
const IS_FLATPAK_MODE: bool = false;

#[derive(Clone, Debug)]
#[allow(unused)]
pub struct SystemdUnitFile {
    pub full_name: String,
    pub status_code: EnablementStatus,
    pub level: UnitDBusLevel,
    pub path: String,
}

impl SystemdUnitFile {
    /*     pub fn full_name(&self) -> Result<&str, SystemdErrors> {
        match self.path.rsplit_once("/") {
            Some((_, end)) => Ok(end),
            None => Err(SystemdErrors::Malformed(
                "rsplit_once(\"/\")".to_string(),
                self.path.clone(),
            )),
        }
    } */
}

#[derive(Default, Clone, PartialEq, Debug)]
pub enum BootFilter {
    #[default]
    Current,
    All,
    Id(String),
}

#[derive(Debug, Default)]
pub struct UpdatedUnitInfo {
    pub primary: String,
    pub object_path: String,
    pub description: Option<String>,
    pub load_state: Option<String>,
    pub sub_state: Option<String>,
    pub active_state: Option<ActiveState>,
    pub unit_file_preset: Option<String>,
    pub valid_unit_name: bool,
    pub fragment_path: Option<String>,
    pub enablement_status: Option<EnablementStatus>,
}

impl UpdatedUnitInfo {
    fn new(primary: String, object_path: String) -> Self {
        Self {
            primary,
            object_path,
            ..Default::default()
        }
    }
}

pub fn get_unit_file_state(sytemd_unit: &UnitInfo) -> Result<EnablementStatus, SystemdErrors> {
    let level = sytemd_unit.dbus_level();
    sysdbus::get_unit_file_state(level, &sytemd_unit.primary())
}

/* pub fn list_units_description_and_state() -> Result<BTreeMap<String, UnitInfo>, SystemdErrors> {
    let level = match PREFERENCES.dbus_level() {
        DbusLevel::Session => UnitDBusLevel::UserSession,
        DbusLevel::System => UnitDBusLevel::System,
        DbusLevel::SystemAndSession => UnitDBusLevel::System,
    };

    match sysdbus::list_units_description_and_state(level) {
        Ok(map) => Ok(map),
        Err(e) => {
            warn!("{:?}", e);
            Err(e)
        }
    }
}
 */
pub async fn list_units_description_and_state_async()
-> Result<(HashMap<String, UnitInfo>, Vec<SystemdUnitFile>), SystemdErrors> {
    sysdbus::list_all_units().await
}

pub async fn complete_unit_information(
    units: &Vec<(String, UnitDBusLevel, Option<String>)>,
) -> Result<Vec<UpdatedUnitInfo>, SystemdErrors> {
    sysdbus::complete_unit_information(units).await
}

pub async fn complete_single_unit_information(
    unit: &UnitInfo,
) -> Result<Vec<UpdatedUnitInfo>, SystemdErrors> {
    let units = vec![(unit.primary(), unit.dbus_level(), unit.object_path())];
    sysdbus::complete_unit_information(&units).await
}

/// Takes a unit name as input and attempts to start it
/// # returns
/// job_path
pub fn start_unit(unit: &UnitInfo, mode: StartStopMode) -> Result<String, SystemdErrors> {
    start_unit_name(unit.dbus_level(), &unit.primary(), mode)
}

/// Takes a unit name as input and attempts to start it
/// # returns
/// job_path
pub fn start_unit_name(
    level: UnitDBusLevel,
    unit_name: &str,
    mode: StartStopMode,
) -> Result<String, SystemdErrors> {
    sysdbus::start_unit(level, unit_name, mode)
}

/// Takes a unit name as input and attempts to stop it.
pub fn stop_unit(unit: &UnitInfo, mode: StartStopMode) -> Result<String, SystemdErrors> {
    sysdbus::stop_unit(unit.dbus_level(), &unit.primary(), mode)
}

pub fn restart_unit(unit: &UnitInfo, mode: StartStopMode) -> Result<String, SystemdErrors> {
    sysdbus::restart_unit(unit.dbus_level(), &unit.primary(), mode)
}

pub fn disenable_unit_file(
    unit: &UnitInfo,
    expected_status: EnablementStatus,
) -> Result<Vec<DisEnAbleUnitFiles>, SystemdErrors> {
    let msg_return = match expected_status {
        EnablementStatus::Enabled | EnablementStatus::EnabledRuntime => sysdbus::enable_unit_files(
            unit.dbus_level(),
            &[&unit.primary()],
            DisEnableFlags::SD_SYSTEMD_UNIT_FORCE,
        )?,
        _ => sysdbus::disable_unit_files(
            unit.dbus_level(),
            &[&unit.primary()],
            DisEnableFlags::empty(),
        )?,
    };

    Ok(msg_return)
}

pub fn enable_unit_file(
    unit_file: &str,
    level: UnitDBusLevel,
    flags: DisEnableFlags,
) -> Result<Vec<DisEnAbleUnitFiles>, SystemdErrors> {
    sysdbus::enable_unit_files(level, &[unit_file], flags)
}

/// Read the unit file and return it's contents so that we can display it
pub fn get_unit_file_info(unit: &UnitInfo) -> Result<String, SystemdErrors> {
    let Some(file_path) = &unit.file_path() else {
        warn!("No file path for {:?}", unit.primary());
        return Ok(String::new());
    };

    #[cfg(feature = "flatpak")]
    match file_open_get_content(file_path) {
        Ok(content) => Ok(content),
        Err(_err) => {
            info!("Flatpack {}", unit.primary());
            match commander_output(&["cat", file_path], None) {
                Ok(cat_output) => match String::from_utf8(cat_output.stdout) {
                    Ok(content) => Ok(content),
                    Err(e) => {
                        warn!("Can't retreive contnent:  {:?}", e);
                        Err(SystemdErrors::Custom("Utf8Error".to_owned()))
                    }
                },
                Err(e) => {
                    warn!("Can't open file \"{file_path}\" in cat, reason: {:?}", e);
                    Err(e)
                }
            }
        }
    }

    #[cfg(not(feature = "flatpak"))]
    file_open_get_content(file_path)
}

fn file_open_get_content(file_path: &str) -> Result<String, SystemdErrors> {
    let file_path = flatpak_host_file_path(file_path);

    let mut file = match File::open(file_path.as_ref()) {
        Ok(f) => f,
        Err(e) => {
            warn!("Can't open file \"{file_path}\", reason: {}", e);
            return Err(SystemdErrors::IoError(e));
        }
    };
    let mut output = String::new();
    let _ = file.read_to_string(&mut output);

    Ok(output)
}

/// Obtains the journal log for the given unit.
pub fn get_unit_journal(
    unit: &UnitInfo,
    boot_filter: BootFilter,
    range: EventRange,
) -> Result<JournalEventChunk, SystemdErrors> {
    journal::get_unit_journal_events(unit, boot_filter, range)
}

pub fn get_unit_journal_continuous(
    unit_name: String,
    level: UnitDBusLevel,
    range: EventRange,
    journal_continuous_receiver: std::sync::mpsc::Receiver<()>,
    sender: std::sync::mpsc::Sender<JournalEventChunk>,
) {
    if let Err(err) = journal::get_unit_journal_events_continuous(
        unit_name,
        level,
        range,
        journal_continuous_receiver,
        sender,
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
    let new_output_result = match commander(prog_n_args, environment_variables).output() {
        Ok(output) => {
            if IS_FLATPAK_MODE {
                info!("Journal status: {}", output.status);

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
        Err(err) => match test_flatpak_spawn() {
            Ok(()) => Err(SystemdErrors::IoError(err)),
            Err(e1) => return Err(e1),
        },
    };

    new_output_result
}

#[cfg(feature = "flatpak")]
pub fn commander(prog_n_args: &[&str], environment_variables: Option<&[(&str, &str)]>) -> Command {
    let mut cmd = Command::new(FLATPAK_SPAWN);
    cmd.arg("--host");
    for v in prog_n_args {
        cmd.arg(v);
    }

    if let Some(envs) = environment_variables {
        for env in envs {
            cmd.arg(format!("--env={}={}", env.0, env.1));
        }
    }

    cmd
}

#[cfg(not(feature = "flatpak"))]
pub fn commander(prog_n_args: &[&str], environment_variables: Option<&[(&str, &str)]>) -> Command {
    let mut cmd = Command::new(prog_n_args[0]);

    for arg in prog_n_args.iter().skip(1) {
        cmd.arg(arg);
    }

    if let Some(envs) = environment_variables {
        for env in envs {
            cmd.env(env.0, env.1);
        }
    }

    cmd
}

pub fn save_text_to_file(
    unit: &UnitInfo,
    text: &GString,
) -> Result<(String, usize), SystemdErrors> {
    let Some(file_path) = &unit.file_path() else {
        error!("No file path for {}", unit.primary());
        return Err(SystemdErrors::NoFilePathforUnit(unit.primary().to_string()));
    };

    let host_file_path = flatpak_host_file_path(file_path);
    match write_on_disk(text, &host_file_path) {
        Ok(bytes_written) => Ok((file_path.clone(), bytes_written)),
        Err(error) => {
            if let SystemdErrors::IoError(ref err) = error {
                match err.kind() {
                    ErrorKind::PermissionDenied => {
                        info!(
                            "Some error : {}, try executing command as another user",
                            err
                        );
                        write_with_priviledge(file_path, host_file_path, text)
                            .map(|bytes_written| (file_path.clone(), bytes_written))
                    }
                    _ => {
                        warn!("Unable to open file: {:?}", err);
                        Err(error)
                    }
                }
            } else {
                Err(error)
            }
        }
    }
}

fn write_on_disk(text: &GString, file_path: &str) -> Result<usize, SystemdErrors> {
    let mut file = match fs::OpenOptions::new().write(true).open(file_path) {
        Ok(file) => file,
        Err(err) => {
            return Err(SystemdErrors::IoError(err));
        }
    };

    match file.write(text.as_bytes()) {
        Ok(bytes_written) => {
            info!("{bytes_written} bytes writen to {}", file_path);
            Ok(bytes_written)
        }
        Err(err) => Err(SystemdErrors::IoError(err)),
    }
}

fn write_with_priviledge(
    file_path: &String,
    host_file_path: Cow<'_, str>,
    text: &GString,
) -> Result<usize, SystemdErrors> {
    let prog_n_args = &["pkexec", "tee", "tee", host_file_path.as_ref()];
    let mut cmd = commander(prog_n_args, None);
    let child_result = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();

    let mut child = match child_result {
        Ok(child) => child,
        Err(error) => {
            error!("failed to execute pkexec tee. Error {:?}", error);
            return Err(SystemdErrors::IoError(error));
        }
    };

    let child_stdin = match child.stdin.as_mut() {
        Some(cs) => cs,
        None => {
            return Err(SystemdErrors::Custom(
                "Unable to write to file: No stdin".to_owned(),
            ));
        }
    };

    let bytes = text.as_bytes();
    let bytes_written = bytes.len();

    match child_stdin.write_all(bytes) {
        Ok(()) => {
            info!("Write content as root on {}", file_path);
        }
        Err(error) => return Err(SystemdErrors::IoError(error)),
    };

    match child.wait() {
        Ok(exit_status) => {
            info!("Subprocess exit status: {:?}", exit_status);
            if !exit_status.success() {
                let code = exit_status.code();
                warn!("Subprocess exit code: {:?}", code);

                let Some(code) = code else {
                    return Err(SystemdErrors::Custom(
                        "Subprocess exit code: None".to_owned(),
                    ));
                };

                let subprocess_error = match code {
                    1 => {
                        if IS_FLATPAK_MODE {
                            let vec = prog_n_args
                                .iter()
                                .map(|s| s.to_string())
                                .collect::<Vec<String>>()
                                .join(" ");
                            SystemdErrors::CmdNoFreedesktopFlatpakPermission(
                                Some(vec),
                                Some(host_file_path.to_string()),
                            )
                        } else {
                            SystemdErrors::Custom(format!("Subprocess exit code: {code}"))
                        }
                    }
                    126 | 127 => return Err(SystemdErrors::NotAuthorized),
                    _ => SystemdErrors::Custom(format!("Subprocess exit code: {code}")),
                };
                return Err(subprocess_error);
            }
        }
        Err(error) => {
            //warn!("Failed to wait suprocess: {:?}", error);
            return Err(SystemdErrors::IoError(error));
        }
    };

    Ok(bytes_written)
}

/// To be able to acces the Flatpack mounted files.
/// Limit to /usr for the least access principle
#[cfg(feature = "flatpak")]
pub fn flatpak_host_file_path(file_path: &str) -> Cow<'_, str> {
    let host_file_path = if file_path.starts_with("/usr") || file_path.starts_with("/etc") {
        Cow::from(format!("/run/host{file_path}"))
    } else {
        Cow::from(file_path)
    };
    host_file_path
}

/// To be able to acces the Flatpack mounted files.
/// Limit to /usr for the least access principle
#[cfg(not(feature = "flatpak"))]
pub fn flatpak_host_file_path(file_path: &str) -> Cow<'_, str> {
    Cow::from(file_path)
}

pub fn generate_file_uri(file_path: &str) -> String {
    let flatpak_host_file_path = flatpak_host_file_path(file_path);
    format!("file://{}", flatpak_host_file_path)
}

pub fn fetch_system_info() -> Result<BTreeMap<String, String>, SystemdErrors> {
    //TODO chec with Session
    sysdbus::fetch_system_info(UnitDBusLevel::System)
}

pub fn fetch_system_unit_info_native(
    unit: &UnitInfo,
) -> Result<HashMap<String, OwnedValue>, SystemdErrors> {
    let level = unit.dbus_level();
    let unit_type: UnitType = UnitType::new(&unit.unit_type());

    let object_path = get_unit_path(unit);

    sysdbus::fetch_system_unit_info_native(level, &object_path, unit_type)
}

fn get_unit_path(unit: &UnitInfo) -> String {
    match unit.object_path() {
        Some(s) => s,
        None => {
            let object_path = sysdbus::unit_dbus_path_from_name(&unit.primary());
            unit.set_object_path(object_path.clone());
            object_path
        }
    }
}

pub fn fetch_unit(
    level: UnitDBusLevel,
    unit_primary_name: &str,
) -> Result<UnitInfo, SystemdErrors> {
    sysdbus::fetch_unit(level, unit_primary_name)
}

pub fn kill_unit(unit: &UnitInfo, who: KillWho, signal: i32) -> Result<(), SystemdErrors> {
    sysdbus::kill_unit(unit.dbus_level(), &unit.primary(), who, signal)
}

pub fn freeze_unit(unit: Option<&UnitInfo>) -> Result<(), SystemdErrors> {
    sysdbus::freeze_unit(
        unit.expect("unit not None").dbus_level(),
        &unit.expect("unit not None").primary(),
    )
}

pub fn thaw_unit(unit: Option<&UnitInfo>) -> Result<(), SystemdErrors> {
    sysdbus::thaw_unit(
        unit.expect("unit not None").dbus_level(),
        &unit.expect("unit not None").primary(),
    )
}

pub fn reload_unit(unit: &UnitInfo, mode: StartStopMode) -> Result<(), SystemdErrors> {
    sysdbus::reload_unit(unit.dbus_level(), &unit.primary(), mode.as_str())
}

pub fn queue_signal_unit(
    unit: &UnitInfo,
    who: KillWho,
    signal: i32,
    value: i32,
) -> Result<(), SystemdErrors> {
    sysdbus::queue_signal_unit(unit.dbus_level(), &unit.primary(), who, signal, value)
}

pub fn clean_unit(unit: &UnitInfo, what: &[String]) -> Result<(), SystemdErrors> {
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

    sysdbus::clean_unit(unit.dbus_level(), &unit.primary(), &clean_what)
}

pub fn mask_unit_files(
    unit: &UnitInfo,
    runtime: bool,
    force: bool,
) -> Result<Vec<DisEnAbleUnitFiles>, SystemdErrors> {
    sysdbus::mask_unit_files(unit.dbus_level(), &[&unit.primary()], runtime, force)
}

pub fn unmask_unit_files(
    unit: &UnitInfo,
    runtime: bool,
) -> Result<Vec<DisEnAbleUnitFiles>, SystemdErrors> {
    sysdbus::unmask_unit_files(unit.dbus_level(), &[&unit.primary()], runtime)
}

pub fn test_flatpak_spawn() -> Result<(), SystemdErrors> {
    if !IS_FLATPAK_MODE {
        return Ok(());
    }

    info!("test_flatpak_spawn");
    match Command::new(FLATPAK_SPAWN).arg("--help").output() {
        Ok(_output) => {}
        Err(_err) => {
            /*
             let message = "Program flatpack-spawn needed!";
             warn!("{message}");
             let message_detail = "The program flatpack-spawn is needed if you use the application from Flatpack. Please install it to enable all features";
             warn!("{message_detail}");

            let alert = gtk::AlertDialog::builder()
                 .message(message)
                 .detail(message_detail)
                 .build();

             alert.show(None::<&gtk::Window>); */
            return Err(SystemdErrors::CmdNoFlatpakSpawn);
        }
    }
    Ok(())
}

pub fn reload_all_units() -> Result<(), SystemdErrors> {
    sysdbus::reload_all_units(UnitDBusLevel::System) //I assume system tbd
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
    unit: &UnitInfo,
    dependency_type: DependencyType,
    plain: bool,
) -> Result<Dependency, SystemdErrors> {
    let object_path = get_unit_path(unit);

    sysdbus::unit_get_dependencies(
        unit.dbus_level(),
        &unit.primary(),
        &object_path,
        dependency_type,
        plain,
    )
}

pub fn get_unit_active_state(
    unit_name: &str,
    level: UnitDBusLevel,
) -> Result<ActiveState, SystemdErrors> {
    let object_path = sysdbus::unit_dbus_path_from_name(unit_name);

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

#[derive(Debug)]
pub struct SystemdSignalRow {
    pub time_stamp: u64,
    pub signal: SystemdSignal,
}

impl SystemdSignalRow {
    pub fn new(signal: SystemdSignal) -> Self {
        let current_system_time = SystemTime::now();
        let since_the_epoch = current_system_time
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        let time_stamp =
            since_the_epoch.as_secs() * 1_000_000 + since_the_epoch.subsec_nanos() as u64 / 1_000;
        SystemdSignalRow { time_stamp, signal }
    }

    pub fn type_text(&self) -> &str {
        self.signal.type_text()
    }

    pub fn details(&self) -> String {
        self.signal.details()
    }
}

#[derive(Debug)]
pub enum SystemdSignal {
    UnitNew(String, OwnedObjectPath),
    UnitRemoved(String, OwnedObjectPath),
    JobNew(u32, OwnedObjectPath, String),
    JobRemoved(u32, OwnedObjectPath, String, String),
    StartupFinished(u64, u64, u64, u64, u64, u64),
    UnitFilesChanged,
    Reloading(bool),
}

impl SystemdSignal {
    pub fn type_text(&self) -> &str {
        match self {
            SystemdSignal::UnitNew(_, _) => "UnitNew",
            SystemdSignal::UnitRemoved(_, _) => "UnitRemoved",
            SystemdSignal::JobNew(_, _, _) => "JobNew",
            SystemdSignal::JobRemoved(_, _, _, _) => "JobRemoved",
            SystemdSignal::StartupFinished(_, _, _, _, _, _) => "StartupFinished",
            SystemdSignal::UnitFilesChanged => "UnitFilesChanged",
            SystemdSignal::Reloading(_) => "Reloading",
        }
    }

    pub fn details(&self) -> String {
        match self {
            SystemdSignal::UnitNew(id, unit) => format!("{id} {unit}"),
            SystemdSignal::UnitRemoved(id, unit) => format!("{id} {unit}"),
            SystemdSignal::JobNew(id, job, unit) => {
                format!("unit={} id={} path={}", unit, id, job)
            }
            SystemdSignal::JobRemoved(id, job, unit, result) => {
                format!("unit={} id={} path={} result={}", unit, id, job, result)
            }
            SystemdSignal::StartupFinished(firmware, loader, kernel, initrd, userspace, total) => {
                format!(
                    "firmware={} loader={} kernel={} initrd={} userspace={} total={}",
                    firmware, loader, kernel, initrd, userspace, total,
                )
            }
            SystemdSignal::UnitFilesChanged => String::new(),
            SystemdSignal::Reloading(active) => format!("firmware={active}"),
        }
    }
}

pub async fn watch_systemd_signals(
    systemd_signal_sender: mpsc::Sender<SystemdSignalRow>,
    cancellation_token: tokio_util::sync::CancellationToken,
) {
    let result: Result<(), SystemdErrors> =
        sysdbus::watcher::watch_systemd_signals(systemd_signal_sender, cancellation_token).await;

    if let Err(err) = result {
        log::error!("Error listening to jobs {:?}", err);
    }
}
