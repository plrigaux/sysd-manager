pub mod analyze;
mod dbus;
mod systemctl;

use std::collections::BTreeMap;
use std::process::Command;
use std::string::FromUtf8Error;

use self::dbus::msgbus::arg::ArgType;
use self::dbus::UnitType;
use gtk::glib::GString;
use log::{debug, error};
use std::fs::{self, File};
use std::io::{Read, Write};

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
        active_state: &String,
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
            active_state: active_state.clone(),
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
        &self.active_state
    }

    pub fn description(&self) -> &str {
        &self.description
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

pub fn restart_unit(unit: &LoadedUnit) -> Result<(), SystemdErrors> {
    dbus::restart_unit(&unit.primary)
}

pub fn enable_unit_files(sytemd_unit: &LoadedUnit) -> Result<std::string::String, SystemdErrors> {
    systemctl::enable_unit_files_path(&sytemd_unit.primary)
}

pub fn disable_unit_files(sytemd_unit: &LoadedUnit) -> Result<std::string::String, SystemdErrors> {
    systemctl::disable_unit_files_path(&sytemd_unit.primary)
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
pub fn get_unit_journal(unit: &LoadedUnit) -> String {
    let unit_path = unit.primary();

    let log = String::from_utf8(
        Command::new("journalctl")
            .arg("-b")
            .arg("-u")
            .arg(unit_path)
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
pub fn save_text_to_file(unit: &LoadedUnit, text: &GString) {
    let Some(file_path) = &unit.file_path else {
        error!("No file path for {}", unit.primary);
        return;
    };

    match fs::OpenOptions::new().write(true).open(file_path) {
        Ok(mut file) => match file.write(text.as_bytes()) {
            Ok(l) => error!("{l} bytes writen to {}", file_path),
            Err(err) => error!("Unable to write to file: {:?}", err),
        },
        Err(err) => error!("Unable to open file: {:?}", err),
    }
}

pub fn fetch_system_info() -> Result<BTreeMap<String, String>, SystemdErrors> {
    dbus::fetch_system_info()
}

pub fn fetch_system_unit_info(
    unit: &LoadedUnit,
) -> Result<BTreeMap<String, String>, SystemdErrors> {
    dbus::fetch_system_unit_info(&unit.object_path)
}

#[cfg(test)]
mod tests {
    use log::debug;

    use super::LoadedUnit;

    #[test]
    fn test_hello() {
        debug!("hello")
    }

    #[test]
    fn test_spliter() {
        let mut a = LoadedUnit::default();

        a.primary = "my_good.service".to_owned();

        assert_eq!("my_good", a.display_name());
        assert_eq!("service", a.unit_type())
    }
}
