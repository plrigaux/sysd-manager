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
use log::{debug, error, info, warn};
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
    pub state: EnablementStatus,
    pub utype: UnitType,
    pub path: String,
    enable_status: String,
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
                debug!("Unknown State: {}", enablement_status);
                EnablementStatus::Unknown
            }
        }
    }

    pub fn to_string(&self) -> String {
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
            _ => "",
        };

        str_label.to_owned()
    }
}

impl From<Option<String>> for EnablementStatus {
    fn from(value: Option<String>) -> Self {
        if let Some(str_val) = value {
            return EnablementStatus::new(&str_val);
        }
        return EnablementStatus::Unknown;
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
/*
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct LoadedUnit {
    primary: String,
    description: String,
    load_state: String,
    active_state: ActiveState,
    sub_state: String,
    followed_unit: String,
    object_path: String,
    file_path: Option<String>,
    enable_status: Option<String>,
    separator: usize, /*     job_id: u32,
                      job_type: String,
                      job_object_path: String, */
}

/* const STATUS_ENABLED: &str = "enabled";
const STATUS_DISABLED: &str = "disabled"; */

impl LoadedUnit {
    pub fn new(
        primary: &String,
        description: &String,
        load_state: &String,
        active_state: ActiveState,
        sub_state: &String,
        followed_unit: &String,
        object_path: String,
    ) -> Self {
        let mut split_char_index = primary.len();
        for (i, c) in primary.chars().enumerate() {
            if c == '.' {
                split_char_index = i;
            }
        }

        Self {
            primary: primary.clone(),
            description: description.clone(),
            load_state: load_state.clone(),
            active_state: active_state,
            sub_state: sub_state.clone(),
            followed_unit: followed_unit.clone(),
            object_path: object_path.to_string(),
            enable_status: None,
            file_path: None,
            separator: split_char_index, /*                   job_id: job_id,
                                         job_type: job_type.clone(),
                                         job_object_path: job_object_path.to_string(), */
        }
    }
    pub fn primary(&self) -> &str {
        &self.primary
    }

    /*     pub fn is_enable(&self) -> bool {
        match &self.enable_status {
            Some(enable_status) => STATUS_ENABLED == enable_status,
            None => false,
        }
    } */

    pub fn enable_status(&self) -> &str {
        match &self.enable_status {
            Some(enable_status) => &enable_status,
            None => "",
        }
    }

    pub fn display_name(&self) -> &str {
        &self.primary[..self.separator]
    }

    pub fn unit_type(&self) -> &str {
        &self.primary[(self.separator + 1)..]
    }

    pub fn active_state(&self) -> &str {
        &self.active_state.label()
    }

    pub fn set_active_state(&mut self, state: ActiveState) {
        self.active_state = state;
    }

    pub fn active_state_icon(&self) -> &str {
        &self.active_state.icon_name()
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn file_path(&self) -> Option<&String> {
        self.file_path.as_ref()
    }

    /*     fn is_enable_or_disable(&self) -> bool {
        match &self.enable_status {
            Some(enable_status) => {
                STATUS_ENABLED == enable_status || STATUS_DISABLED == enable_status
            }
            None => false,
        }
    } */
}
*/

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
    use log::debug;

    #[test]
    fn test_hello() {
        debug!("hello")
    }
}
