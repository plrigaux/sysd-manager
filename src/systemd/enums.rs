use crate::gtk::prelude::*;
use gtk::glib;
use gtk::glib::EnumValue;
use log::info;
use log::warn;
use std::cell::RefCell;
use std::fmt::Display;
use strum::EnumIter;

use super::sysdbus::INTERFACE_SYSTEMD_MANAGER;
use super::sysdbus::INTERFACE_SYSTEMD_UNIT;

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, Default)]
pub enum EnablementStatus {
    #[default]
    Unknown = 0,
    Alias = 1,
    Bad = 2,
    Disabled = 3,
    Enabled = 4,
    Generated = 5,
    Indirect = 6,
    Linked = 7,
    Masked = 8,
    Static = 9,
    Trancient = 10,
}

impl EnablementStatus {
    /// Takes the string containing the state information from the dbus message and converts it
    /// into a UnitType by matching the first character.
    pub fn new(enablement_status: &str) -> EnablementStatus {
        if enablement_status.is_empty() {
            info!("Empty Enablement Status: \"{}\"", enablement_status);
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
                warn!("Unknown State: {}", enablement_status);
                EnablementStatus::Unknown
            }
        }
    }

    pub fn to_str(&self) -> &str {
        let str_label = match self {
            EnablementStatus::Alias => "alias",
            EnablementStatus::Bad => "bad",
            EnablementStatus::Disabled => "disabled",
            EnablementStatus::Enabled => "enabled",
            EnablementStatus::Indirect => "indirect",
            EnablementStatus::Linked => "linked",
            EnablementStatus::Masked => "masked",
            EnablementStatus::Static => "static",
            EnablementStatus::Generated => "generated",
            EnablementStatus::Trancient => "trancient",
            EnablementStatus::Unknown => "",
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
        return EnablementStatus::Unknown;
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
            0 => Self::Unknown,
            1 => Self::Alias,
            2 => Self::Bad,
            3 => Self::Disabled,
            4 => Self::Enabled,
            5 => Self::Generated,
            6 => Self::Indirect,
            7 => Self::Linked,
            8 => Self::Masked,
            9 => Self::Static,
            10 => Self::Trancient,
            _ => Self::Unknown,
        }
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, glib::Enum, EnumIter)]
#[enum_type(name = "ActiveState")]
#[enum_dynamic]
pub enum ActiveState {
    Unknown = 0,
    Active = 1,
    #[default]
    Inactive = 2,
}

impl ActiveState {
    pub fn label(&self) -> &str {
        match self {
            ActiveState::Active => "active",
            ActiveState::Inactive => "inactive",
            ActiveState::Unknown => "unknown",
        }
    }

    pub fn icon_name(&self) -> Option<&str> {
        match self {
            ActiveState::Active => Some("object-select-symbolic"),
            ActiveState::Inactive => Some("window-close-symbolic"),
            ActiveState::Unknown => None,
        }
    }

    pub fn from_str(input: &str) -> Self {
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

#[derive(Clone, Debug, PartialEq, Eq, EnumIter)]
pub enum UnitType {
    Automount,
    Busname,
    Device,
    Manager,
    Mount,
    Path,
    Scope,
    Service,
    Slice,
    Snapshot,
    Socket,
    Swap,
    Target,
    Timer,
    Unknown(String),
}

impl UnitType {
    /// Takes the pathname of the unit as input to determine what type of unit it is.
    pub fn new(system_type: &str) -> UnitType {
        match system_type {
            "automount" => UnitType::Automount,
            "busname" => UnitType::Busname,
            "device" => UnitType::Device,
            "mount" => UnitType::Mount,
            "path" => UnitType::Path,
            "scope" => UnitType::Scope,
            "service" => UnitType::Service,
            "slice" => UnitType::Slice,
            "socket" => UnitType::Socket,
            "swap" => UnitType::Swap,
            "target" => UnitType::Target,
            "timer" => UnitType::Timer,
            "snapshot" => UnitType::Snapshot,
            _ => {
                info!("Unknown Type: {}", system_type);
                UnitType::Unknown(system_type.to_string())
            }
        }
    }

    pub fn to_str(&self) -> &str {
        let str_label = match self {
            Self::Automount => "automount",
            Self::Busname => "busname",
            Self::Device => "device",
            Self::Manager => "manager",
            Self::Mount => "mount",
            Self::Path => "path",
            Self::Scope => "scope",
            Self::Service => "service",
            Self::Slice => "slice",
            Self::Socket => "socket",
            Self::Target => "target",
            Self::Timer => "timer",
            Self::Swap => "swap",
            Self::Snapshot => "snapshot",         
            _ => "",
        };

        str_label
    }

    pub fn interface(&self) -> &str {
        let str_label = match self {
            Self::Automount => "org.freedesktop.systemd1.Automount",
            //Self::Busname => "busname",
            Self::Device => "org.freedesktop.systemd1.Device",
            Self::Manager => INTERFACE_SYSTEMD_MANAGER,
            Self::Mount => "org.freedesktop.systemd1.Mount",
            Self::Path => "org.freedesktop.systemd1.Path",
            Self::Scope => "org.freedesktop.systemd1.Sscope",
            Self::Service => "org.freedesktop.systemd1.Service",
            Self::Slice => "org.freedesktop.systemd1.Slice",
            Self::Snapshot => "org.freedesktop.systemd1.Snapshot",
            Self::Socket => "org.freedesktop.systemd1.Socket",
            Self::Swap => "org.freedesktop.systemd1.Swap",
            Self::Target => "org.freedesktop.systemd1.Target",
            Self::Timer => "org.freedesktop.systemd1.Timer",

            _ => INTERFACE_SYSTEMD_UNIT,
        };

        str_label
    }

    pub(crate) fn extends_unit(&self) -> bool {
        let extends_unit = match self {
            Self::Automount => true,
            //Self::Busname => "busname",
            Self::Device => true,
            Self::Manager => false,
            Self::Mount => true,
            Self::Path => true,
            Self::Scope => true,
            Self::Service => true,
            Self::Slice => true,
            Self::Snapshot => true,
            Self::Socket => true,
            Self::Swap => true,
            Self::Target => true,
            Self::Timer => true,

            _ => false,
        };

        extends_unit
    }
}

/// KillUnit() may be used to kill (i.e. send a signal to) all processes of a unit.
/// Takes the unit name, an enum who and a UNIX signal number to send.
/// The who enum is one of "main", "control" or "all". If "main", only the main process of a unit is killed. If "control" only the control process of the unit is killed, if "all" all processes are killed. A "control" process is for example a process that is configured via ExecStop= and is spawned in parallel to the main daemon process, in order to shut it down.
#[derive(Debug, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "KillWho")]
pub enum KillWho {
    /// If "main", only the main process of a unit is killed.
    #[enum_value(name = "main", nick = "Only the main unit's process")]
    Main,

    ///If "control" only the control process of the unit is killed
    /// A "control" process is for example a process that is configured via ExecStop= and is spawned in parallel to the main daemon process, in order to shut it down.
    #[enum_value(name = "control", nick = "Only the unit's controled processes")]
    Control,

    ///If "all" all processes are killed.
    #[enum_value(name = "all", nick = "All unit's processes")]
    All,
}

impl KillWho {
    pub fn to_string(&self) -> String {
        let value: glib::Value = self.to_value();

        let out = if let Some(enum_value) = EnumValue::from_value(&value) {
            enum_value.1.name()
        } else {
            ""
        };
        out.to_string()
    }

    pub fn as_str(&self) -> &str {
        let str_label = match self {
            Self::Main => "main",
            Self::Control => "control",
            Self::All => "all",
        };

        str_label
    }
}

impl From<u32> for KillWho {
    fn from(value: u32) -> Self {
        match value {
            0 => KillWho::Main,
            1 => KillWho::Control,
            2 => KillWho::All,
            _ => KillWho::Main,
        }
    }
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
        //assert_num_mapping(EnablementStatus::Unasigned);
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

    #[test]
    fn test_kill_who_glib() {
        assert_kill(KillWho::All);
        assert_kill(KillWho::Main);
        assert_kill(KillWho::Control);
    }

    fn assert_kill(kill: KillWho) {
        assert_eq!(kill.as_str(), kill.to_string())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, glib::Enum, EnumIter)]
#[enum_type(name = "EnableUnitFileMode")]
//#[allow(dead_code)]
pub enum StartStopMode {
    ///If "replace" the call will start the unit and its dependencies,
    /// possibly replacing already queued jobs that conflict with this.
    Replace,

    ///If "fail" the call will start the unit and its dependencies, but will fail if this
    ///would change an already queued job.
    #[default]
    Fail,

    ///If "isolate" the call will start the unit in
    ///question and terminate all units that aren't dependencies of it.
    ///Note that "isolate" mode is invalid for method **StopUnit**.
    Isolate,

    ///If "ignore-dependencies" it will start a unit but ignore all its dependencies.
    IgnoreDependencies,

    ///If "ignore-requirements" it will start a unit but only ignore the requirement dependencies.
    IgnoreRequirements,
}

impl StartStopMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            StartStopMode::Fail => "fail",
            StartStopMode::Replace => "replace",
            StartStopMode::Isolate => "isolate",
            StartStopMode::IgnoreDependencies => "ignore-dependencies",
            StartStopMode::IgnoreRequirements => "ignore-requirements",
        }
    }
}

impl From<&RefCell<String>> for StartStopMode {
    fn from(value: &RefCell<String>) -> Self {
        let s = value.borrow();

        let mode = match s.as_str() {
            "fail" => StartStopMode::Fail,
            "replace" => StartStopMode::Replace,
            "isolate" => StartStopMode::Isolate,
            "ignore-dependencies" => StartStopMode::IgnoreDependencies,
            "ignore-requirements" => StartStopMode::IgnoreRequirements,

            unknown => {
                warn!("unknown  mode {:?}", unknown);
                StartStopMode::Fail
            }
        };
        mode
    }
}
