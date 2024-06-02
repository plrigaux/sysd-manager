pub mod analyze;
mod dbus;
mod systemctl;

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;
use std::string::FromUtf8Error;

use std::fs::File;
use std::io::Read;
use systemd::dbus::msgbus::arg::ArgType;
use systemd::dbus::UnitType;

#[derive(Debug)]
pub enum SystemdErrors {
    IoError(std::io::Error),
    Utf8Error(FromUtf8Error),
    SystemCtlError(String),
    DBusErrorStr(String),
    DBusError(dbus::msgbus::Error),
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

impl From<dbus::msgbus::Error> for SystemdErrors {
    fn from(error: dbus::msgbus::Error) -> Self {
        SystemdErrors::DBusError(error)
    }
}

#[derive(Clone, Debug)]
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

#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct LoadedUnit {
    primary: String,
    description: String,
    load_state: String,
    active_state: String,
    sub_state: String,
    followed_unit: String,
    object_path: String,
    file_path: Option<String>,
    enable_status: Option<String>,
    /*     job_id: u32,
    job_type: String,
    job_object_path: String, */
}

const STATUS_ENABLED: &str = "enabled";
const STATUS_DISABLED: &str = "disabled";

impl LoadedUnit {
    pub fn is_enable(&self) -> bool {
        match &self.enable_status {
            Some(enable_status) => STATUS_ENABLED == enable_status,
            None => false,
        }
    }

    pub fn enable_status(&self) -> &str {
        match &self.enable_status {
            Some(enable_status) => &enable_status,
            None => "",
        }
    }

    pub fn display_name(&self) -> &str {
        let mut split_char_index = self.primary.len();
        for (i, c) in self.primary.chars().enumerate() {
            if c == '.' {
                split_char_index = i;
                break;
            }
        }
        &self.primary[..split_char_index]
    }

    pub fn unit_type(&self) -> &str {
        let mut split_char_index = self.primary.len();
        for (i, c) in self.primary.chars().enumerate() {
            if c == '.' {
                split_char_index = i + 1;
                break;
            }
        }
        &self.primary[split_char_index..]
    }

    fn is_enable_or_disable(&self) -> bool {
        match &self.enable_status {
            Some(enable_status) => {
                STATUS_ENABLED == enable_status || STATUS_DISABLED == enable_status
            }
            None => false,
        }
    }
}

pub fn get_unit_file_state(sytemd_unit: &LoadedUnit) -> Result<EnablementStatus, SystemdErrors> {
    return dbus::get_unit_file_state_path(&sytemd_unit.primary);
}

pub fn list_units_description_and_state() -> Result<BTreeMap<String, LoadedUnit>, SystemdErrors> {
    return dbus::list_units_description_and_state();
}

/// Takes a unit name as input and attempts to start it
pub fn start_unit(unit: &LoadedUnit) -> Result<(), SystemdErrors> {
    dbus::start_unit(&unit.primary)
}

/// Takes a unit name as input and attempts to stop it.
pub fn stop_unit(unit: &LoadedUnit) -> Result<(), SystemdErrors> {
    dbus::stop_unit(&unit.primary)
}

pub fn enable_unit_files(sytemd_unit: &LoadedUnit) -> Result<std::string::String, SystemdErrors> {
    systemctl::enable_unit_files_path(&sytemd_unit.primary)
}

pub fn disable_unit_files(sytemd_unit: &LoadedUnit) -> Result<std::string::String, SystemdErrors> {
    systemctl::disable_unit_files_path(&sytemd_unit.primary)
}

/// Takes a `Vec<SystemdUnit>` as input and returns a new vector only containing services which can be enabled and
/// disabled.
pub fn collect_togglable_services(units: &Vec<LoadedUnit>) -> Vec<LoadedUnit> {
    units
        .iter()
        .filter(|x| x.unit_type() == "service" && x.is_enable_or_disable())
        .cloned()
        .collect()
}

/// Takes a `Vec<SystemdUnit>` as input and returns a new vector only containing sockets which can be enabled and
/// disabled.
pub fn collect_togglable_sockets(units: &[LoadedUnit]) -> Vec<LoadedUnit> {
    units
        .iter()
        .filter(|x| x.unit_type() == "socket" && x.is_enable_or_disable())
        .cloned()
        .collect()
}

/// Takes a `Vec<SystemdUnit>` as input and returns a new vector only containing timers which can be enabled and
/// disabled.
pub fn collect_togglable_timers(units: &[LoadedUnit]) -> Vec<LoadedUnit> {
    units
        .iter()
        .filter(|x| x.unit_type() == "timer" && x.is_enable_or_disable())
        .cloned()
        .collect()
}

/// Read the unit file and return it's contents so that we can display it
pub fn get_unit_info(unit: &LoadedUnit) -> String {
    let mut output = String::new();
    if let Some(file_path) = &unit.file_path {
        let mut file = File::open(file_path).unwrap();
        let _ = file.read_to_string(&mut output);
    }
    output
}

/// Obtains the journal log for the given unit.
pub fn get_unit_journal(unit_path: &str) -> String {
    let log = String::from_utf8(
        Command::new("journalctl")
            .arg("-b")
            .arg("-u")
            .arg(Path::new(unit_path).file_stem().unwrap().to_str().unwrap())
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    log.lines()
        .rev()
        .map(|x| x.trim())
        .fold(String::with_capacity(log.len()), |acc, x| acc + "\n" + x)
}

#[cfg(test)]
mod tests {
    use super::LoadedUnit;

    #[test]
    fn test_hello() {
        println!("hello")
    }

    #[test]
    fn test_spliter() {
        let mut a = LoadedUnit::default();

        a.primary = "my_good.service".to_owned();

        assert_eq!("my_good", a.display_name());
        assert_eq!("service", a.unit_type())
    }
}
