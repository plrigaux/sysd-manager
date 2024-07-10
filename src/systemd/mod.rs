pub mod analyze;
pub mod data;
mod sysdbus;
mod systemctl;

use std::collections::BTreeMap;
use std::env;
use std::process::{Command, Stdio};
use std::string::FromUtf8Error;

use data::UnitInfo;
use enums::{EnablementStatus, UnitType};
use gtk::glib::GString;
use log::{error, info, warn};
use std::fs::{self, File};
use std::io::{ErrorKind, Read, Write};
use sysdbus::dbus::arg::ArgType;

pub mod enums;

const SYSDMNG_DIST_MODE: &str = "SYSDMNG_DIST_MODE";
const FLATPACK: &str = "flatpack";

#[derive(Debug)]
#[allow(unused)]
pub enum SystemdErrors {
    IoError(std::io::Error),
    Utf8Error(FromUtf8Error),
    SystemCtlError(String),
    DBusErrorStr(String),
    DBusError(sysdbus::dbus::Error),
    Malformed,
    MalformedWrongArgType(ArgType),
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

impl From<sysdbus::dbus::Error> for SystemdErrors {
    fn from(error: sysdbus::dbus::Error) -> Self {
        SystemdErrors::DBusError(error)
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

pub fn get_unit_file_state(sytemd_unit: &UnitInfo) -> Result<EnablementStatus, SystemdErrors> {
    return sysdbus::get_unit_file_state_path(&sytemd_unit.primary());
}

pub fn list_units_description_and_state() -> Result<BTreeMap<String, UnitInfo>, SystemdErrors> {
    return sysdbus::list_units_description_and_state();
}

/// Takes a unit name as input and attempts to start it
pub fn start_unit(unit: &UnitInfo) -> Result<(), SystemdErrors> {
    sysdbus::start_unit(&unit.primary())
}

/// Takes a unit name as input and attempts to stop it.
pub fn stop_unit(unit: &UnitInfo) -> Result<(), SystemdErrors> {
    sysdbus::stop_unit(&unit.primary())
}

pub fn restart_unit(unit: &UnitInfo) -> Result<(), SystemdErrors> {
    sysdbus::restart_unit(&unit.primary())
}

pub fn enable_unit_files(sytemd_unit: &UnitInfo) -> Result<EnablementStatus, SystemdErrors> {
    match systemctl::enable_unit_files_path(&sytemd_unit.primary()) {
        Ok(_) => Ok(EnablementStatus::Enabled),
        Err(e) => Err(e),
    }
}

pub fn disable_unit_files(sytemd_unit: &UnitInfo) -> Result<EnablementStatus, SystemdErrors> {
    match systemctl::disable_unit_files_path(&sytemd_unit.primary()) {
        Ok(_) => Ok(EnablementStatus::Disabled),
        Err(e) => Err(e),
    }
}

/// Read the unit file and return it's contents so that we can display it
pub fn get_unit_info(unit: &UnitInfo) -> String {
    let mut output = String::new();
    if let Some(file_path) = &unit.file_path() {
        let mut file = match File::open(file_path) {
            Ok(f) => f,
            Err(e) => {
                warn!("Can't open file \"{file_path}\", reason: {:?}", e);
                return output;
            }
        };
        let _ = file.read_to_string(&mut output);
    }
    output
}

/// Obtains the journal log for the given unit.
pub fn get_unit_journal(unit: &UnitInfo) -> String {
    let unit_path = unit.primary();

    let output = if is_flatpak_mode() {
        Command::new("flatpak-spawn").arg("--host").arg("journalctl").arg("-b").arg("-u").arg(unit_path).output()
    } else {
        Command::new("journalctl").arg("-b").arg("-u").arg(unit_path).output()
    };

    let outout = match output {
        Ok(output) => output.stdout,
        Err(e) => {
            warn!("Can't retreive journal:  {:?}", e);
            return String::new();
        }
    };

    let logs = match String::from_utf8(outout) {
        Ok(logs) => logs,
        Err(e) => {
            warn!("Can't retreive journal:  {:?}", e);
            return String::new();
        }
    };

    logs.lines()
        .rev()
        .map(|x| x.trim())
        .fold(String::with_capacity(logs.len()), |acc, x| acc + "\n" + x)
}

fn is_flatpak_mode() -> bool {
    match env::var(SYSDMNG_DIST_MODE) {
        Ok(val) => FLATPACK.eq(&val),
        Err(_) => false,
    }
}

pub fn save_text_to_file(unit: &UnitInfo, text: &GString) {
    let Some(file_path) = &unit.file_path() else {
        error!("No file path for {}", unit.primary());
        return;
    };

    let mut file = match fs::OpenOptions::new().write(true).open(file_path) {
        Ok(file) => file,
        Err(err) => {
            if err.kind() == ErrorKind::PermissionDenied {
                write_with_priviledge(file_path, text);
            } else {
                error!("Unable to open file: {:?}", err);
            }

            return;
        }
    };

    match file.write(text.as_bytes()) {
        Ok(l) => error!("{l} bytes writen to {}", file_path),
        Err(err) => error!("Unable to write to file: {:?}", err),
    }
}

fn write_with_priviledge(file_path: &String, text: &GString) {
    let mut child = std::process::Command::new("pkexec")
        .arg("tee")
        .arg(file_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .expect("failed to execute pkexec tee");

    let child_stdin = match child.stdin.as_mut() {
        Some(cs) => cs,
        None => {
            error!("Unable to write to file: No stdin");
            return;
        }
    };

    match child_stdin.write_all(text.as_bytes()) {
        Ok(_) => info!("Write content as root on {}", file_path),
        Err(e) => error!("Write error: {:?}", e),
    }

    match child.wait() {
        Ok(exit) => info!("Subprocess exit code: {:?}", exit),
        Err(e) => error!("Failed to wait suprocess: {:?}", e),
    }
}

pub fn fetch_system_info() -> Result<BTreeMap<String, String>, SystemdErrors> {
    sysdbus::fetch_system_info()
}

pub fn fetch_system_unit_info(unit: &UnitInfo) -> Result<BTreeMap<String, String>, SystemdErrors> {
    sysdbus::fetch_system_unit_info(&unit.object_path())
}
