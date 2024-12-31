pub mod analyze;
pub mod data;
pub mod journal;
pub mod journal_data;
mod sysdbus;

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::process::{Command, Stdio};
use std::string::FromUtf8Error;

use data::UnitInfo;
use enums::{EnablementStatus, KillWho, StartStopMode, UnitType};
use gtk::glib::GString;
use journal_data::JournalEvent;
use log::{error, info, warn};
use std::fs::{self, File};
use std::io::{ErrorKind, Read, Write};
use zvariant::OwnedValue;

use crate::widget::preferences::data::DbusLevel;
use crate::widget::preferences::data::PREFERENCES;

pub mod enums;

//const SYSDMNG_DIST_MODE: &str = "SYSDMNG_DIST_MODE";
//const FLATPACK: &str = "flatpak";
const FLATPAK_SPAWN: &str = "flatpak-spawn";

/* static IS_FLATPAK_MODE: LazyLock<bool> = LazyLock::new(|| match env::var(SYSDMNG_DIST_MODE) {
    Ok(val) => FLATPACK.eq(&val),
    Err(_) => false,
}); */

#[cfg(feature = "flatpak")]
const IS_FLATPAK_MODE : bool = false;

#[cfg(not(feature = "flatpak"))]
const IS_FLATPAK_MODE : bool = true;



#[derive(Debug)]
#[allow(unused)]
pub enum SystemdErrors {
    Custom(String),
    IoError(std::io::Error),
    Utf8Error(FromUtf8Error),
    SystemCtlError(String),
    //DBusErrorStr(String),
    Malformed,
    ZBusError(zbus::Error),
    ZBusFdoError(zbus::fdo::Error),
    CmdNoFlatpakSpawn,
    CmdNoFreedesktopFlatpakPermission(Vec<String>, String),
    JournalError(String),
    NoFilePathforUnit(String),
    FlatpakAccess(ErrorKind),
    NotAuthorized,
}

impl SystemdErrors {
    pub fn gui_description(&self) -> Option<String> {
        let desc = match self {
            SystemdErrors::CmdNoFlatpakSpawn => {
                let value = "The program <b>flatpack-spawn</b> is needed if you use the application from Flatpack.\n
Please install it to enable all features.";
                Some(value.to_owned())
            }
            SystemdErrors::CmdNoFreedesktopFlatpakPermission(cmdl, _file_path) => {
                let msg = format!(
                "Requires permission to talk to <b>org.freedesktop.Flatpak</b> D-Bus interface when the program is a Flatpak.\n
<b>Option 1:</b> You can use Flatseal. Under Session Bus Talks add <b>org.freedesktop.Flatpak</b> and restart the program\n
<b>Option 2:</b> In your terminal, run the command: <u>{}</u>", cmdl.join(" "));
                Some(msg)
            }
            _ => None,
        };

        desc
    }
}

impl From<std::io::Error> for SystemdErrors {
    fn from(error: std::io::Error) -> Self {
        SystemdErrors::IoError(error)
    }
}

impl From<FromUtf8Error> for SystemdErrors {
    fn from(error: FromUtf8Error) -> Self {
        SystemdErrors::Utf8Error(error)
    }
}

impl From<zbus::Error> for SystemdErrors {
    fn from(error: zbus::Error) -> Self {
        SystemdErrors::ZBusError(error)
    }
}

impl From<zbus::fdo::Error> for SystemdErrors {
    fn from(error: zbus::fdo::Error) -> Self {
        SystemdErrors::ZBusFdoError(error)
    }
}

impl From<Box<dyn std::error::Error>> for SystemdErrors {
    fn from(error: Box<dyn std::error::Error>) -> Self {
        let msg = format!("{}", error);

        SystemdErrors::JournalError(msg)
    }
}

#[derive(Clone, Debug)]
#[allow(unused)]
pub struct SystemdUnit {
    pub name: String,
    pub status_code: EnablementStatus,
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

#[derive(Default, Clone, PartialEq)]
pub enum BootFilter {
    #[default]
    Current,
    All,
    Id(String),
}

pub fn get_unit_file_state(sytemd_unit: &UnitInfo) -> Result<EnablementStatus, SystemdErrors> {
    let level: DbusLevel = PREFERENCES.dbus_level().into();
    return sysdbus::get_unit_file_state_path(level, &sytemd_unit.primary());
}

pub fn list_units_description_and_state() -> Result<BTreeMap<String, UnitInfo>, SystemdErrors> {
    let level: DbusLevel = PREFERENCES.dbus_level().into();

    match sysdbus::list_units_description_and_state(level) {
        Ok(map) => Ok(map),
        Err(e) => {
            warn!("{:?}", e);
            Err(e)
        }
    }
}

/// Takes a unit name as input and attempts to start it
pub fn start_unit(unit: &UnitInfo, mode: StartStopMode) -> Result<String, SystemdErrors> {
    let level: DbusLevel = PREFERENCES.dbus_level().into();
    sysdbus::start_unit(level, &unit.primary(), mode)
}

/// Takes a unit name as input and attempts to stop it.
pub fn stop_unit(unit: &UnitInfo, mode: StartStopMode) -> Result<String, SystemdErrors> {
    let level: DbusLevel = PREFERENCES.dbus_level().into();
    sysdbus::stop_unit(level, &unit.primary(), mode)
}

pub fn restart_unit(unit: &UnitInfo, mode: StartStopMode) -> Result<String, SystemdErrors> {
    let level: DbusLevel = PREFERENCES.dbus_level().into();
    sysdbus::restart_unit(level, &unit.primary(), mode)
}

pub fn get_unit_object_path(unit: &UnitInfo) -> Result<String, SystemdErrors> {
    let level: DbusLevel = PREFERENCES.dbus_level().into();
    sysdbus::get_unit_object_path(level, &unit.primary())
}

pub fn enable_unit_files(unit: &UnitInfo) -> Result<(EnablementStatus, String), SystemdErrors> {
    let level: DbusLevel = PREFERENCES.dbus_level().into();
    let msg_return = sysdbus::enable_unit_files(level, &unit.primary())?;

    let msg = if msg_return.vec.len() > 0 {
        let a = &msg_return.vec[0];
        format!(
            "Created {} '{}' → '{}'",
            a.change_type, a.file_name, a.destination
        )
    } else {
        "Success".to_string()
    };

    Ok((EnablementStatus::Enabled, msg))
}

pub fn disable_unit_files(unit: &UnitInfo) -> Result<(EnablementStatus, String), SystemdErrors> {
    let level: DbusLevel = PREFERENCES.dbus_level().into();
    let msg_return = sysdbus::disable_unit_files(level, &[&unit.primary()])?;

    let msg = if msg_return.len() > 0 {
        let a = &msg_return[0];
        format!("{} '{}'", a.change_type, a.file_name,)
    } else {
        "Success".to_string()
    };
    Ok((EnablementStatus::Disabled, msg))
}

/// Read the unit file and return it's contents so that we can display it
pub fn get_unit_file_info(unit: &UnitInfo) -> Result<String, SystemdErrors> {
    let Some(file_path) = &unit.file_path() else {
        info!("No file path for {}", unit.primary());
        return Ok(String::new());
    };

    match file_open_get_content(file_path) {
        Ok(content) => Ok(content),
        Err(err) => {
            if IS_FLATPAK_MODE {
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
            } else {
                Err(err)
            }
        }
    }
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
    in_color: bool,
    oldest_first: bool,
    max_events: u32,
    boot_filter: BootFilter,
) -> Result<Vec<JournalEvent>, SystemdErrors> {
    journal::get_unit_journal(unit, in_color, oldest_first, max_events, boot_filter)
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
                    return Err(SystemdErrors::CmdNoFreedesktopFlatpakPermission(vec, String::new()));
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

pub fn commander(prog_n_args: &[&str], environment_variables: Option<&[(&str, &str)]>) -> Command {
    let command = if IS_FLATPAK_MODE {
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
    } else {
        let mut cmd = Command::new(prog_n_args[0]);

        for i in 1..prog_n_args.len() {
            cmd.arg(prog_n_args[i]);
        }

        if let Some(envs) = environment_variables {
            for env in envs {
                cmd.env(env.0, env.1);
            }
        }

        cmd
    };

    command
}

pub fn save_text_to_file(unit: &UnitInfo, text: &GString) -> Result<(String, usize), SystemdErrors> {
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
                        write_with_priviledge(file_path, host_file_path, text).map(|bytes_written| (file_path.clone(), bytes_written))
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

                match code {
                    1 => {
                        if IS_FLATPAK_MODE {
                            let vec = prog_n_args.iter().map(|s| s.to_string()).collect();
                            return Err(SystemdErrors::CmdNoFreedesktopFlatpakPermission(vec, host_file_path.to_string()));
                        }
                    }
                    126 | 127 => return Err(SystemdErrors::NotAuthorized),
                    _ => {
                        return Err(SystemdErrors::Custom(format!(
                            "Subprocess exit code: {code}"
                        )))
                    }
                };

             
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
/// Limit to /usr for the leat access principle
pub fn flatpak_host_file_path(file_path: &str) -> Cow<'_, str> {
    let host_file_path =
        if IS_FLATPAK_MODE && (file_path.starts_with("/usr") || file_path.starts_with("/etc")) {
            Cow::from(format!("/run/host{file_path}"))
        } else {
            Cow::from(file_path)
        };
    host_file_path
}

pub fn generate_file_uri(file_path: &str) -> String {
    let flatpak_host_file_path = flatpak_host_file_path(file_path);
    let uri = format!("file://{}", flatpak_host_file_path);
    uri
}

pub fn fetch_system_info() -> Result<BTreeMap<String, String>, SystemdErrors> {
    let level: DbusLevel = PREFERENCES.dbus_level().into();

    sysdbus::fetch_system_info(level)
}

pub fn fetch_system_unit_info(unit: &UnitInfo) -> Result<BTreeMap<String, String>, SystemdErrors> {
    let level: DbusLevel = PREFERENCES.dbus_level().into();
    let unit_type: UnitType = UnitType::new(&unit.unit_type());
    sysdbus::fetch_system_unit_info(level, &unit.object_path(), unit_type)
}

pub fn fetch_system_unit_info_native(
    unit: &UnitInfo,
) -> Result<HashMap<String, OwnedValue>, SystemdErrors> {
    let level: DbusLevel = PREFERENCES.dbus_level().into();
    let unit_type: UnitType = UnitType::new(&unit.unit_type());
    sysdbus::fetch_system_unit_info_native(level, &unit.object_path(), unit_type)
}

pub fn kill_unit(unit: &UnitInfo, who: KillWho, signal: i32) -> Result<(), SystemdErrors> {
    let level: DbusLevel = PREFERENCES.dbus_level().into();
    sysdbus::kill_unit(level, &unit.primary(), who, signal)
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
    let level: DbusLevel = PREFERENCES.dbus_level().into();
    sysdbus::reload_all_units(level)
}
