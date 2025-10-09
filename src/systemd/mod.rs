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

use tokio::{runtime::Runtime, sync::mpsc};
use zvariant::{OwnedObjectPath, OwnedValue};

use crate::systemd::{
    data::{EnableUnitFilesReturn, LUnit},
    enums::LoadState,
};

pub mod enums;

const FLATPAK_SPAWN: &str = "flatpak-spawn";

pub fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| Runtime::new().expect("Setting up tokio runtime needs to succeed."))
}

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
    pub load_state: Option<LoadState>,
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

pub fn get_unit_file_state(
    level: UnitDBusLevel,
    primary_name: &str,
) -> Result<EnablementStatus, SystemdErrors> {
    sysdbus::get_unit_file_state(level, primary_name)
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

pub async fn list_units_description_and_state_async(
    level: UnitDBusLevel,
) -> Result<(Vec<LUnit>, Vec<SystemdUnitFile>), SystemdErrors> {
    sysdbus::list_units_description_and_state_async(level).await
}

pub async fn complete_unit_information(
    units: Vec<(UnitDBusLevel, String, String)>,
) -> Result<Vec<UpdatedUnitInfo>, SystemdErrors> {
    sysdbus::complete_unit_information(units).await
}

pub async fn complete_single_unit_information(
    primary_name: String,
    level: UnitDBusLevel,
    object_path: String,
) -> Result<Vec<UpdatedUnitInfo>, SystemdErrors> {
    let units = vec![(level, primary_name, object_path)];
    sysdbus::complete_unit_information(units).await
}

/// Takes a unit name as input and attempts to start it
/// # returns
/// job_path
pub fn start_unit(
    level: UnitDBusLevel,
    unit_name: &str,
    mode: StartStopMode,
) -> Result<String, SystemdErrors> {
    start_unit_name(level, unit_name, mode)
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
pub fn stop_unit(
    level: UnitDBusLevel,
    primary_name: &str,
    mode: StartStopMode,
) -> Result<String, SystemdErrors> {
    sysdbus::stop_unit(level, primary_name, mode)
}

pub fn restart_unit(
    level: UnitDBusLevel,
    primary_name: &str,
    mode: StartStopMode,
) -> Result<String, SystemdErrors> {
    sysdbus::restart_unit(level, primary_name, mode)
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum DisEnableUnitFilesOutput {
    Enable(EnableUnitFilesReturn),
    Disable(Vec<DisEnAbleUnitFiles>),
}

pub fn disenable_unit_file(
    primary_name: String,
    level: UnitDBusLevel,
    enable_status: EnablementStatus,
    expected_status: EnablementStatus,
) -> Result<DisEnableUnitFilesOutput, SystemdErrors> {
    let msg_return = match expected_status {
        EnablementStatus::Enabled | EnablementStatus::EnabledRuntime => {
            let res = sysdbus::enable_unit_files(
                level,
                &[&primary_name],
                DisEnableFlags::SD_SYSTEMD_UNIT_FORCE,
            )?;
            DisEnableUnitFilesOutput::Enable(res)
        }
        _ => {
            let flags = if enable_status.is_runtime() {
                DisEnableFlags::SD_SYSTEMD_UNIT_RUNTIME
            } else {
                DisEnableFlags::empty()
            };

            let out = sysdbus::disable_unit_files(level, &[&primary_name], flags)?;
            DisEnableUnitFilesOutput::Disable(out)
        }
    };

    Ok(msg_return)
}

pub fn enable_unit_file(
    level: UnitDBusLevel,
    unit_file: &str,
    flags: DisEnableFlags,
) -> Result<EnableUnitFilesReturn, SystemdErrors> {
    sysdbus::enable_unit_files(level, &[unit_file], flags)
}

pub fn disable_unit_files(
    level: UnitDBusLevel,
    unit_file: &str,
    flags: DisEnableFlags,
) -> Result<Vec<DisEnAbleUnitFiles>, SystemdErrors> {
    sysdbus::disable_unit_files(level, &[unit_file], flags)
}

/// Read the unit file and return it's contents so that we can display it
pub fn get_unit_file_info(unit: &UnitInfo) -> Result<String, SystemdErrors> {
    let Some(file_path) = &unit.file_path() else {
        warn!("No file path for {:?}", unit.primary());
        return Ok(String::new());
    };

    if cfg!(feature = "flatpak") {
        flatpak_file_open_get_content(file_path, unit)
    } else {
        //#[cfg(not(feature = "flatpak"))]
        file_open_get_content(file_path, unit)
    }
}

#[allow(dead_code)]
fn flatpak_file_open_get_content(
    file_path: &str,
    unit: &UnitInfo,
) -> Result<String, SystemdErrors> {
    match file_open_get_content(file_path, unit) {
        Ok(content) => Ok(content),
        Err(_) => file_open_get_content_cat(file_path, unit),
    }
}

fn file_open_get_content_cat(file_path: &str, unit: &UnitInfo) -> Result<String, SystemdErrors> {
    info!(
        "Flatpack Fetching file content Unit: {} File \"{file_path}\"",
        unit.primary()
    );
    //Use the REAL path because try to acceess through the 'cat' command
    match commander_output(&["cat", file_path], None) {
        Ok(cat_output) => match String::from_utf8(cat_output.stdout) {
            Ok(content) => Ok(content),
            Err(e) => {
                warn!("Can't retreive contnent: {e:?}");
                Err(SystemdErrors::Custom("Utf8Error".to_owned()))
            }
        },
        Err(e) => {
            error!("Can't open file \"{file_path}\" with 'cat' command, reason: {e:?}");
            Err(e)
        }
    }
}

fn file_open_get_content(file_path: &str, unit: &UnitInfo) -> Result<String, SystemdErrors> {
    //To get the relative path from a Flatpack
    let file_path = flatpak_host_file_path(file_path);
    info!(
        "Fetching file content Unit: {} File: {file_path}",
        unit.primary()
    );
    let mut file = File::open(file_path.as_ref()).map_err(|e| {
        warn!("Can't open file \"{file_path}\", reason: {e}");
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
) -> Result<JournalEventChunk, SystemdErrors> {
    journal::get_unit_journal_events(primary_name, level, boot_filter, range)
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
    match commander(prog_n_args, environment_variables).output() {
        Ok(output) => {
            if cfg!(feature = "flatpak") {
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
            Err(e1) => Err(e1),
        },
    }
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
    info!("Try to save content on File: {host_file_path}");
    match write_on_disk(text, &host_file_path) {
        Ok(bytes_written) => Ok((file_path.clone(), bytes_written)),
        Err(error) => {
            if let SystemdErrors::IoError(ref err) = error {
                match err.kind() {
                    ErrorKind::PermissionDenied => {
                        info!("Some error : {err}, try executing command as another user");
                        write_with_priviledge(file_path, host_file_path, text)
                            .map(|bytes_written| (file_path.clone(), bytes_written))
                    }
                    _ => {
                        warn!("Unable to open file: {err:?}");
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
    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(file_path)?;

    let test_bytes = text.as_bytes();
    file.write_all(test_bytes)?;
    file.flush()?;

    let bytes_written = test_bytes.len();
    info!("{bytes_written} bytes writen on File: {file_path}");
    Ok(bytes_written)
}

fn write_with_priviledge(
    file_path: &String,
    _host_file_path: Cow<'_, str>,
    text: &GString,
) -> Result<usize, SystemdErrors> {
    let prog_n_args = &["pkexec", "tee", "tee", file_path.as_str()];
    let mut cmd = commander(prog_n_args, None);
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| SystemdErrors::create_command_error(&cmd, error))?;

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
            info!("Write content as root on {file_path}");
        }
        Err(error) => return Err(SystemdErrors::IoError(error)),
    };

    match child.wait() {
        Ok(exit_status) => {
            info!("Subprocess exit status: {exit_status:?}");
            if !exit_status.success() {
                let code = exit_status.code();
                warn!("Subprocess exit code: {code:?}");

                let Some(code) = code else {
                    return Err(SystemdErrors::Custom(
                        "Subprocess exit code: None".to_owned(),
                    ));
                };

                let subprocess_error = match code {
                    1 => {
                        if cfg!(feature = "flatpak") {
                            let vec = prog_n_args
                                .iter()
                                .map(|s| s.to_string())
                                .collect::<Vec<String>>()
                                .join(" ");
                            SystemdErrors::CmdNoFreedesktopFlatpakPermission(
                                Some(vec),
                                Some(file_path.to_string()),
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
pub fn flatpak_host_file_path(file_path: &str) -> Cow<'_, str> {
    if cfg!(feature = "flatpak") && (file_path.starts_with("/usr") || file_path.starts_with("/etc"))
    {
        Cow::from(format!("/run/host{file_path}"))
    } else {
        Cow::from(file_path)
    }
}

pub fn generate_file_uri(file_path: &str) -> String {
    let flatpak_host_file_path = flatpak_host_file_path(file_path);
    format!("file://{flatpak_host_file_path}")
}

pub fn fetch_system_info() -> Result<BTreeMap<String, String>, SystemdErrors> {
    //TODO check with Session (user)
    sysdbus::fetch_system_info(UnitDBusLevel::System)
}

pub fn fetch_system_unit_info_native(
    unit: &UnitInfo,
) -> Result<HashMap<String, OwnedValue>, SystemdErrors> {
    let level = unit.dbus_level();
    let unit_type: UnitType = unit.unit_type();

    let object_path = unit.object_path();

    sysdbus::fetch_system_unit_info_native(level, &object_path, unit_type)
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
    if let Some((level, primary_name)) = params {
        sysdbus::freeze_unit(level, &primary_name)
    } else {
        Err(SystemdErrors::NoUnit)
    }
}

pub fn thaw_unit(params: Option<(UnitDBusLevel, String)>) -> Result<(), SystemdErrors> {
    if let Some((level, primary_name)) = params {
        sysdbus::thaw_unit(level, &primary_name)
    } else {
        Err(SystemdErrors::NoUnit)
    }
}

pub fn reload_unit(
    level: UnitDBusLevel,
    primary_name: &str,
    mode: StartStopMode,
) -> Result<(), SystemdErrors> {
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
    primary_name: &str,
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

    sysdbus::clean_unit(level, primary_name, &clean_what)
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
) -> Result<EnableUnitFilesReturn, SystemdErrors> {
    sysdbus::preset_unit_file(level, &[primary_name], runtime, force)
}

pub fn reenable_unit_file(
    level: UnitDBusLevel,
    primary_name: &str,
    runtime: bool,
    force: bool,
) -> Result<EnableUnitFilesReturn, SystemdErrors> {
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

pub fn test_flatpak_spawn() -> Result<(), SystemdErrors> {
    if cfg!(feature = "flatpak") {
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
                format!("unit={unit} id={id} path={job}")
            }
            SystemdSignal::JobRemoved(id, job, unit, result) => {
                format!("unit={unit} id={id} path={job} result={result}")
            }
            SystemdSignal::StartupFinished(firmware, loader, kernel, initrd, userspace, total) => {
                format!(
                    "firmware={firmware} loader={loader} kernel={kernel} initrd={initrd} userspace={userspace} total={total}",
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
        log::error!("Error listening to jobs {err:?}");
    }
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
    path: &str,
    property_interface: &str,
    property: &str,
) -> Result<OwnedValue, SystemdErrors> {
    sysdbus::fetch_unit_properties(level, path, property_interface, property).await
}
