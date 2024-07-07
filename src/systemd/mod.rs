pub mod analyze;
pub mod data;
mod sysdbus;
mod systemctl;

use std::collections::BTreeMap;
use std::fmt::Display;
use std::process::{Command, Stdio};
use std::string::FromUtf8Error;

use data::UnitInfo;
use gtk::glib::GString;
use log::{error, info, warn};
use std::fs::{self, File};
use std::io::{ErrorKind, Read, Write};
use sysdbus::dbus::arg::ArgType;
use sysdbus::UnitType;

use gtk::glib;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnablementStatus {
    Unasigned = 0,
    Bad = 1,
    Disabled = 2,
    Enabled = 3,
    Indirect = 4,
    Linked = 5,
    Masked = 6,
    Static = 7,
    Alias = 8,
    Generated = 9,
    Trancient = 10,
    Unknown = 11,
}

impl EnablementStatus {
    /// Takes the string containing the state information from the dbus message and converts it
    /// into a UnitType by matching the first character.
    pub fn new(enablement_status: &str) -> EnablementStatus {
        if enablement_status.is_empty() {
            error!("Empty Status: {}", enablement_status);
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
                info!("Unknown State: {}", enablement_status);
                EnablementStatus::Unknown
            }
        }
    }

    pub fn to_str(&self) -> &str {
        let str_label = match self {
            EnablementStatus::Bad => "bad",
            EnablementStatus::Disabled => "disabled",
            EnablementStatus::Enabled => "enabled",
            EnablementStatus::Indirect => "indirect",
            EnablementStatus::Linked => "linked",
            EnablementStatus::Masked => "masked",
            EnablementStatus::Static => "static",
            EnablementStatus::Alias => "alias",
            EnablementStatus::Generated => "generated",
            EnablementStatus::Trancient => "trancient",
            EnablementStatus::Unknown => "UNKNOWN",
            _ => "",
        };

        str_label
    }

    pub fn to_string(&self) -> String {
        self.to_str().to_owned()
    }
}

impl From<Option<String>> for EnablementStatus {
    fn from(value: Option<String>) -> Self {
        if let Some(str_val) = value {
            return EnablementStatus::new(&str_val);
        }
        return EnablementStatus::Unasigned;
    }
}

impl From<EnablementStatus> for u32 {
    fn from(value: EnablementStatus) -> Self {
        value as u32
    }
}

impl From<u32> for EnablementStatus {
    fn from(value: u32) -> Self {
        match value {
            0 => Self::Unasigned,
            1 => Self::Bad,
            2 => Self::Disabled,
            3 => Self::Enabled,
            4 => Self::Enabled,
            5 => Self::Linked,
            6 => Self::Masked,
            7 => Self::Static,
            8 => Self::Alias,
            9 => Self::Generated,
            10 => Self::Trancient,
            11 => Self::Unknown,
            _ => Self::Unknown,
        }
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "ActiveState")]
#[enum_dynamic]
pub enum ActiveState {
    Unknown = 0,
    Active = 1,
    #[default]
    Inactive = 2,
}

impl ActiveState {
    fn label(&self) -> &str {
        match self {
            ActiveState::Active => "active",
            ActiveState::Inactive => "inactive",
            ActiveState::Unknown => "unknown",
        }
    }

    pub fn icon_name(&self) -> &str {
        match self {
            ActiveState::Active => "object-select-symbolic",
            ActiveState::Inactive => "window-close-symbolic",
            ActiveState::Unknown => "action-unavailable-symbolic",
        }
    }

    fn from_str(input: &str) -> Self {
        match input {
            "active" => ActiveState::Active,
            "inactive" => ActiveState::Inactive,
            _ => ActiveState::Unknown,
        }
    }
}

impl Display for ActiveState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

impl From<u32> for ActiveState {
    fn from(value: u32) -> Self {
        match value {
            0 => Self::Unknown,
            1 => Self::Active,
            2 => Self::Inactive,
            _ => Self::Unknown,
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
                warn!("Can't open {file_path}, reason: {:?}", e);
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

    let outout = match Command::new("journalctl")
        .arg("-b")
        .arg("-u")
        .arg(unit_path)
        .output()
    {
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

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_enablement_status_any_number() {
        assert_eq!(
            <u32 as Into<EnablementStatus>>::into(1000),
            EnablementStatus::Unknown
        )
    }

    #[test]
    fn test_enablement_status_mapping() {
        assert_num_mapping(EnablementStatus::Unasigned);
        assert_num_mapping(EnablementStatus::Bad);
        assert_num_mapping(EnablementStatus::Enabled);
        assert_num_mapping(EnablementStatus::Disabled);
        assert_num_mapping(EnablementStatus::Linked);
        assert_num_mapping(EnablementStatus::Masked);
        assert_num_mapping(EnablementStatus::Static);
        assert_num_mapping(EnablementStatus::Alias);
        assert_num_mapping(EnablementStatus::Generated);
        assert_num_mapping(EnablementStatus::Trancient);
        assert_num_mapping(EnablementStatus::Unknown);
    }

    fn assert_num_mapping(status: EnablementStatus) {
        let val = status as u32;
        let convert: EnablementStatus = val.into();
        assert_eq!(convert, status)
    }

    #[test]
    fn test_active_state_any_number() {
        assert_eq!(<u32 as Into<ActiveState>>::into(1000), ActiveState::Unknown)
    }

    #[test]
    fn test_active_state_mapping() {
        assert_num_mapping_active_state(ActiveState::Unknown);
        assert_num_mapping_active_state(ActiveState::Active);
        assert_num_mapping_active_state(ActiveState::Inactive);
    }

    fn assert_num_mapping_active_state(status: ActiveState) {
        let val = status as u32;
        let convert: ActiveState = val.into();
        assert_eq!(convert, status)
    }
}
