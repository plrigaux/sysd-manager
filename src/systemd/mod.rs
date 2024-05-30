pub mod analyze;
mod dbus;
mod systemctl;

use std::string::FromUtf8Error;

use systemd::dbus::UnitType;

#[derive(Debug)]
pub enum SystemdErrors {
    IoError(std::io::Error),
    Utf8Error(FromUtf8Error),
    SystemCtlError(String)
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

#[derive(Clone, Debug)]
pub struct SystemdUnit {
    pub name: String,
    pub state: EnablementStatus,
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

pub fn get_unit_file_state(sytemd_unit: &SystemdUnit) -> EnablementStatus {
    return dbus::get_unit_file_state_path(sytemd_unit.full_name());
}

pub fn list_unit_files() -> Vec<SystemdUnit> {
    return dbus::list_unit_files();
}

/// Takes a unit name as input and attempts to start it
pub fn start_unit(unit: &SystemdUnit) -> Option<String> {
    dbus::start_unit(unit.full_name())
}

/// Takes a unit name as input and attempts to stop it.
pub fn stop_unit(unit: &SystemdUnit) -> Option<String> {
    dbus::stop_unit(unit.full_name())
}

pub fn enable_unit_files(sytemd_unit: &SystemdUnit) -> Result<std::string::String, SystemdErrors> {
    systemctl::enable_unit_files_path(sytemd_unit.full_name())
}

pub fn disable_unit_files(sytemd_unit: &SystemdUnit) -> Result<std::string::String, SystemdErrors> {
    systemctl::disable_unit_files_path(sytemd_unit.full_name())
}

/// Takes a `Vec<SystemdUnit>` as input and returns a new vector only containing services which can be enabled and
/// disabled.
pub fn collect_togglable_services(units: &Vec<SystemdUnit>) -> Vec<SystemdUnit> {
    units
        .iter()
        .filter(|x| {
            x.utype == UnitType::Service
                && (x.state == EnablementStatus::Enabled || x.state == EnablementStatus::Disabled)
            // && !x.path.contains("/etc/")
        })
        .cloned()
        .collect()
}

/// Takes a `Vec<SystemdUnit>` as input and returns a new vector only containing sockets which can be enabled and
/// disabled.
pub fn collect_togglable_sockets(units: &[SystemdUnit]) -> Vec<SystemdUnit> {
    units
        .iter()
        .filter(|x| {
            x.utype == UnitType::Socket
                && (x.state == EnablementStatus::Enabled || x.state == EnablementStatus::Disabled)
        })
        .cloned()
        .collect()
}

/// Takes a `Vec<SystemdUnit>` as input and returns a new vector only containing timers which can be enabled and
/// disabled.
pub fn collect_togglable_timers(units: &[SystemdUnit]) -> Vec<SystemdUnit> {
    units
        .iter()
        .filter(|x| {
            x.utype == UnitType::Timer
                && (x.state == EnablementStatus::Enabled || x.state == EnablementStatus::Disabled)
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_hello() {
        println!("hello")
    }
}
