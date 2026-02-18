/* use super::sysdbus::INTERFACE_SYSTEMD_MANAGER;
use super::sysdbus::INTERFACE_SYSTEMD_UNIT; */

use enumflags2::_internal::RawBitFlags;
use enumflags2::bitflags;
use gettextrs::pgettext;
use glib::value::ToValue;
use glib::{self, EnumValue};
use std::str::FromStr;
use std::{cell::RefCell, fmt::Display};
use strum::EnumIter;
use strum::IntoEnumIterator;
use tracing::{info, warn};
use zvariant::OwnedValue;

use crate::errors::SystemdErrors;
use crate::sysdbus::{INTERFACE_SYSTEMD_MANAGER, INTERFACE_SYSTEMD_UNIT};

const MASKED: &str = "masked";
const ENABLED: &str = "enabled";
const LINKED: &str = "linked";

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, EnumIter, Default, Hash, glib::Enum, Ord, PartialOrd,
)]
#[enum_type(name = "Preset")]
pub enum Preset {
    #[default]
    UnSet,
    Enabled,
    Disabled,
    Ignore,
}

impl Preset {
    pub fn as_str(&self) -> &'static str {
        match self {
            Preset::UnSet => "",
            Preset::Ignore => "ignored",
            Preset::Disabled => "disabled",
            Preset::Enabled => ENABLED,
        }
    }

    pub fn as_str_op(&self) -> Option<&'static str> {
        match self {
            Preset::UnSet => None,
            Preset::Ignore => Some("ignored"),
            Preset::Disabled => Some("disabled"),
            Preset::Enabled => Some(ENABLED),
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Preset::UnSet => "<i>not set</i>",
            _ => self.as_str(),
        }
    }

    pub fn tooltip_info(&self) -> Option<&str> {
        None
    }
}

impl From<&str> for Preset {
    fn from(value: &str) -> Self {
        match value {
            ENABLED => Preset::Enabled,
            "disabled" => Preset::Disabled,
            "ignored" => Preset::Ignore,
            _ => Preset::UnSet,
        }
    }
}

impl From<String> for Preset {
    fn from(value: String) -> Self {
        value.as_str().into()
    }
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, EnumIter, Default, Hash, glib::Enum, PartialOrd, Ord,
)]
#[enum_type(name = "EnablementStatus")]
pub enum UnitFileStatus {
    #[default]
    Unknown,
    Alias,
    Bad,
    Disabled,
    Enabled,
    EnabledRuntime,
    Generated,
    Indirect,
    Linked,
    LinkedRuntime,
    Masked,
    MaskedRuntime,
    Static,
    Transient,
}

impl UnitFileStatus {
    /// Takes the string containing the state information from the dbus message and converts it
    /// into a UnitType by matching the first character.
    pub fn from_strr(enablement_status: &str) -> UnitFileStatus {
        if enablement_status.is_empty() {
            info!("Empty Enablement Status: \"{enablement_status}\"");
            return UnitFileStatus::Unknown;
        }

        let c = enablement_status.chars().next().unwrap();

        match c {
            'a' => UnitFileStatus::Alias,
            's' => UnitFileStatus::Static,
            'd' => UnitFileStatus::Disabled,
            'e' => {
                if enablement_status.len() == ENABLED.len() {
                    UnitFileStatus::Enabled
                } else {
                    UnitFileStatus::EnabledRuntime
                }
            }
            'i' => UnitFileStatus::Indirect,
            'l' => {
                if enablement_status.len() == LINKED.len() {
                    UnitFileStatus::Linked
                } else {
                    UnitFileStatus::LinkedRuntime
                }
            }
            'm' => {
                if enablement_status.len() == MASKED.len() {
                    UnitFileStatus::Masked
                } else {
                    UnitFileStatus::MaskedRuntime
                }
            }
            'b' => UnitFileStatus::Bad,
            'g' => UnitFileStatus::Generated,
            't' => UnitFileStatus::Transient,
            _ => {
                warn!("Unknown State: {enablement_status}");
                UnitFileStatus::Unknown
            }
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            UnitFileStatus::Alias => "alias",
            UnitFileStatus::Bad => "bad",
            UnitFileStatus::Disabled => "disabled",
            UnitFileStatus::Enabled => ENABLED,
            UnitFileStatus::Indirect => "indirect",
            UnitFileStatus::Linked => LINKED,
            UnitFileStatus::Masked => MASKED,
            UnitFileStatus::Static => "static",
            UnitFileStatus::Generated => "generated",
            UnitFileStatus::Transient => "transient",
            UnitFileStatus::EnabledRuntime => "enabled-runtime",
            UnitFileStatus::LinkedRuntime => "linked-runtime",
            UnitFileStatus::MaskedRuntime => "masked-runtime",
            UnitFileStatus::Unknown => "",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            UnitFileStatus::Unknown => "<i>unset</i>",
            _ => self.as_str(),
        }
    }

    pub fn tooltip_info(&self) -> Option<String> {
        match self {
            //tooltip column cell
            UnitFileStatus::Alias => Some(pgettext(
                "list",
                "The name is an alias (symlink to another unit file).",
            )),
            //tooltip column cell
            UnitFileStatus::Bad => Some(pgettext(
                "list",
                "The unit file is invalid or another error occurred.",
            )),
            //tooltip column cell
            UnitFileStatus::Disabled => Some(pgettext(
                "list",
                "The unit file is not enabled, but contains an [Install] section with installation instructions.",
            )),
            //tooltip column cell
            UnitFileStatus::Enabled => Some(pgettext(
                "list",
                "Enabled via <span fgcolor='#62a0ea'>.wants/</span>, <span fgcolor='#62a0ea'>.requires/</span> or <u>Alias=</u> symlinks (permanently in <span fgcolor='#62a0ea'>/etc/systemd/system/</span>, or transiently in <span fgcolor='#62a0ea'>/run/systemd/system/</span>).",
            )),
            //tooltip column cell
            UnitFileStatus::Generated => Some(pgettext(
                "list",
                "The unit file was generated dynamically via a generator tool. See <b>man systemd.generator(7)</b>. Generated unit files may not be enabled, they are enabled implicitly by their generator.",
            )),
            //tooltip column cell
            UnitFileStatus::Indirect => Some(pgettext(
                "list",
                "The unit file itself is not enabled, but it has a non-empty <u>Also=</u> setting in the [Install] unit file section, listing other unit files that might be enabled, or it has an alias under a different name through a symlink that is not specified in <u>Also=</u>. For template unit files, an instance different than the one specified in <u>DefaultInstance=</u> is enabled.",
            )),
            //tooltip column cell
            UnitFileStatus::Linked => Some(pgettext(
                "list",
                "Made available through one or more symlinks to the unit file (permanently in <span fgcolor='#62a0ea'>/etc/systemd/system/</span> or transiently in <span fgcolor='#62a0ea'>/run/systemd/system/</span>), even though the unit file might reside outside of the unit file search path.",
            )),
            //tooltip column cell
            UnitFileStatus::Masked => Some(pgettext(
                "list",
                "Completely disabled, so that any start operation on it fails (permanently in <span fgcolor='#62a0ea'>/etc/systemd/system/</span> or transiently in <span fgcolor='#62a0ea'>/run/systemd/systemd/</span>).",
            )),
            //tooltip column cell
            UnitFileStatus::Static => Some(pgettext(
                "list",
                "The unit file is not enabled, and has no provisions for enabling in the [Install] unit file section.",
            )),
            //tooltip column cell
            UnitFileStatus::Transient => Some(pgettext(
                "list",
                "The unit file has been created dynamically with the runtime API. Transient units may not be enabled.",
            )),

            UnitFileStatus::Unknown => None,
            //tooltip column cell
            UnitFileStatus::EnabledRuntime => Some(pgettext(
                "list",
                "Enabled via <span fgcolor='#62a0ea'>.wants/</span>, <span fgcolor='#62a0ea'>.requires/</span> or <u>Alias=</u> symlinks (permanently in <span fgcolor='#62a0ea'>/etc/systemd/system/</span>, or transiently in <span fgcolor='#62a0ea'>/run/systemd/system/</span>).",
            )),
            //tooltip column cell
            UnitFileStatus::LinkedRuntime => Some(pgettext(
                "list",
                "Made available through one or more symlinks to the unit file (permanently in <span fgcolor='#62a0ea'>/etc/systemd/system/</span> or transiently in <span fgcolor='#62a0ea'>/run/systemd/system/</span>), even though the unit file might reside outside of the unit file search path.",
            )),
            //tooltip column cell
            UnitFileStatus::MaskedRuntime => Some(pgettext(
                "list",
                "Completely disabled, so that any start operation on it fails (permanently in <span fgcolor='#62a0ea'>/etc/systemd/system/</span> or transiently in <span fgcolor='#62a0ea'>/run/systemd/systemd/</span>).",
            )),
        }
    }

    pub fn is_runtime(&self) -> bool {
        matches!(
            self,
            UnitFileStatus::LinkedRuntime
                | UnitFileStatus::MaskedRuntime
                | UnitFileStatus::EnabledRuntime
        )
    }

    pub fn has_status(&self) -> bool {
        !matches!(self, UnitFileStatus::Unknown)
    }
}

impl Display for UnitFileStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{}", self.as_str())
    }
}

impl From<Option<String>> for UnitFileStatus {
    fn from(value: Option<String>) -> Self {
        if let Some(str_val) = value {
            return UnitFileStatus::from_str(&str_val).expect("always Status");
        }
        UnitFileStatus::Unknown
    }
}

impl From<String> for UnitFileStatus {
    fn from(value: String) -> Self {
        UnitFileStatus::from_str(&value).expect("always Status")
    }
}

impl From<&str> for UnitFileStatus {
    fn from(value: &str) -> Self {
        UnitFileStatus::from_str(value).expect("always Status")
    }
}

impl FromStr for UnitFileStatus {
    type Err = SystemdErrors;

    fn from_str(enablement_status: &str) -> Result<Self, Self::Err> {
        if enablement_status.is_empty() {
            info!("Empty Enablement Status: \"{enablement_status}\"");
            return Ok(UnitFileStatus::Unknown);
        }

        let c = enablement_status.chars().next().unwrap();

        let status = match c {
            'a' => UnitFileStatus::Alias,
            's' => UnitFileStatus::Static,
            'd' => UnitFileStatus::Disabled,
            'e' => {
                if enablement_status.len() == ENABLED.len() {
                    UnitFileStatus::Enabled
                } else {
                    UnitFileStatus::EnabledRuntime
                }
            }
            'i' => UnitFileStatus::Indirect,
            'l' => {
                if enablement_status.len() == LINKED.len() {
                    UnitFileStatus::Linked
                } else {
                    UnitFileStatus::LinkedRuntime
                }
            }
            'm' => {
                if enablement_status.len() == MASKED.len() {
                    UnitFileStatus::Masked
                } else {
                    UnitFileStatus::MaskedRuntime
                }
            }
            'b' => UnitFileStatus::Bad,
            'g' => UnitFileStatus::Generated,
            't' => UnitFileStatus::Transient,
            _ => {
                warn!("Unknown State: {enablement_status}");
                UnitFileStatus::Unknown
            }
        };
        Ok(status)
    }
}

#[derive(
    Clone, Copy, Default, Debug, PartialEq, Eq, EnumIter, Hash, glib::Enum, PartialOrd, Ord,
)]
#[enum_type(name = "ActiveState")]
pub enum ActiveState {
    Unknown,
    Active,
    Activating,
    Reloading,
    #[default]
    Inactive,
    Failed,
    Deactivating,
    Maintenance,
    Refreshing,
}

impl ActiveState {
    pub fn as_str(&self) -> &str {
        match self {
            ActiveState::Unknown => "unknown",
            ActiveState::Active => "active",
            ActiveState::Reloading => "reloading",
            ActiveState::Inactive => "inactive",
            ActiveState::Failed => "failed",
            ActiveState::Activating => "activating",
            ActiveState::Deactivating => "deactivating",
            ActiveState::Maintenance => "maintenance",
            ActiveState::Refreshing => "refreshing",
        }
    }

    pub fn label(&self) -> &str {
        match self {
            ActiveState::Unknown => "<i>unset</i>",
            _ => self.as_str(),
        }
    }

    pub fn icon_name(&self) -> Option<&'static str> {
        match self {
            ActiveState::Active
            | ActiveState::Reloading
            | ActiveState::Refreshing
            | ActiveState::Activating => Some("object-select-symbolic"),
            ActiveState::Inactive | ActiveState::Deactivating => Some("window-close-symbolic"),
            ActiveState::Failed => Some("computer-fail-symbolic"), //not sure of the icon choice
            ActiveState::Maintenance => Some("emblem-system-symbolic"), //not sure of the icon choice
            ActiveState::Unknown => None,
        }
    }

    pub fn is_inactive(&self) -> bool {
        matches!(
            self,
            ActiveState::Inactive | ActiveState::Deactivating | ActiveState::Unknown
        )
    }

    pub fn glyph_str(&self) -> &str {
        match self {
            ActiveState::Active => "●",
            ActiveState::Reloading => "↻",
            ActiveState::Inactive => "○",
            ActiveState::Failed => "×",
            ActiveState::Activating => "●",
            ActiveState::Deactivating => "●",
            ActiveState::Maintenance => "○",
            ActiveState::Refreshing => "↻",
            _ => " ",
        }
    }

    pub fn tooltip_info(&self) -> Option<&str> {
        let value = match self {
            ActiveState::Active => "Started, bound, plugged in, ..., depending on the unit type.",
            ActiveState::Reloading => {
                "Unit is <b>active</b> and it is reloading its configuration."
            }
            ActiveState::Inactive => {
                "Stopped, unbound, unplugged, ..., depending on the unit type."
            }
            ActiveState::Failed => {
                "Similar to inactive, but the unit failed in some way (process returned error code on exit, crashed, an operation timed out, or after too many restarts)."
            }
            ActiveState::Activating => "Changing from <b>inactive</b> to <b>active</b>. ",
            ActiveState::Deactivating => "Changing from <b>active</b> to <b>inactive</b>. ",
            ActiveState::Maintenance => {
                "Unit is inactive and a maintenance operation is in progress."
            }
            ActiveState::Refreshing => {
                "Unit is active and a new mount is being activated in its namespace."
            }
            _ => "",
        };

        Some(value)
    }
}

impl Display for ActiveState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<String> for ActiveState {
    fn from(value: String) -> Self {
        value.as_str().into()
    }
}

impl From<&str> for ActiveState {
    fn from(value: &str) -> Self {
        match value {
            "active" => ActiveState::Active,
            "reloading" => ActiveState::Reloading,
            "inactive" => ActiveState::Inactive,
            "failed" => ActiveState::Failed,
            "activating" => ActiveState::Activating,
            "deactivating" => ActiveState::Deactivating,
            "maintenance" => ActiveState::Maintenance,
            "refreshing" => ActiveState::Refreshing,
            _ => ActiveState::Unknown,
        }
    }
}

impl From<Option<&OwnedValue>> for ActiveState {
    fn from(value: Option<&OwnedValue>) -> Self {
        match value {
            Some(value) => {
                let state_str: &str = value.try_into().unwrap_or_default();
                ActiveState::from(state_str)
            }
            None => ActiveState::Unknown,
        }
    }
}

#[derive(
    Default, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, EnumIter, glib::Enum, Hash,
)]
#[enum_type(name = "UnitType")]
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
    Unit,
    #[default]
    Unknown,
}

impl UnitType {
    /// Takes the pathname of the unit as input to determine what type of unit it is.
    pub fn new(unit_type: &str) -> UnitType {
        match unit_type {
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
            "unit" => UnitType::Unit,
            _ => {
                warn!("Unknown Unit Type name: {unit_type}");
                UnitType::Unknown
            }
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
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
            Self::Unit => "unit",
            _ => "",
        }
    }

    pub fn interface(&self) -> &str {
        match self {
            Self::Automount => "org.freedesktop.systemd1.Automount",
            //Self::Busname => "busname",
            Self::Device => "org.freedesktop.systemd1.Device",
            Self::Manager => INTERFACE_SYSTEMD_MANAGER,
            Self::Mount => "org.freedesktop.systemd1.Mount",
            Self::Path => "org.freedesktop.systemd1.Path",
            Self::Scope => "org.freedesktop.systemd1.Scope",
            Self::Service => "org.freedesktop.systemd1.Service",
            Self::Slice => "org.freedesktop.systemd1.Slice",
            Self::Snapshot => "org.freedesktop.systemd1.Snapshot",
            Self::Socket => "org.freedesktop.systemd1.Socket",
            Self::Swap => "org.freedesktop.systemd1.Swap",
            Self::Target => "org.freedesktop.systemd1.Target",
            Self::Timer => "org.freedesktop.systemd1.Timer",
            Self::Unit => INTERFACE_SYSTEMD_UNIT,
            Self::Unknown => "",

            _ => INTERFACE_SYSTEMD_UNIT,
        }
    }

    pub fn from_intreface(interface: &str) -> UnitType {
        match interface {
            "org.freedesktop.systemd1.Automount" => UnitType::Automount,
            "org.freedesktop.systemd1.Device" => UnitType::Device,
            "org.freedesktop.systemd1.Mount" => UnitType::Mount,
            "org.freedesktop.systemd1.Path" => UnitType::Path,
            "org.freedesktop.systemd1.Scope" => UnitType::Scope,
            "org.freedesktop.systemd1.Service" => UnitType::Service,
            "org.freedesktop.systemd1.Slice" => UnitType::Slice,
            "org.freedesktop.systemd1.Snapshot" => UnitType::Snapshot,
            "org.freedesktop.systemd1.Socket" => UnitType::Socket,
            "org.freedesktop.systemd1.Swap" => UnitType::Swap,
            "org.freedesktop.systemd1.Target" => UnitType::Target,
            "org.freedesktop.systemd1.Timer" => UnitType::Timer,
            INTERFACE_SYSTEMD_UNIT => UnitType::Unit,
            _ => {
                warn!("Unknown Unit Type: {interface}");
                UnitType::Unknown
            }
        }
    }

    pub(crate) fn extends_unit(&self) -> bool {
        match self {
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
        }
    }
}

impl From<&str> for UnitType {
    fn from(value: &str) -> Self {
        UnitType::new(value)
    }
}

impl From<String> for UnitType {
    fn from(value: String) -> Self {
        UnitType::new(&value)
    }
}

/// KillUnit() may be used to kill (i.e. send a signal to) all processes of a unit.
/// Takes the unit name, an enum who and a UNIX signal number to send.
/// The who enum is one of "main", "control" or "all". If "main", only the main process of a unit is killed. If "control" only the control process of the unit is killed, if "all" all processes are killed. A "control" process is for example a process that is configured via ExecStop= and is spawned in parallel to the main daemon process, in order to shut it down.
#[derive(Debug, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "KillWho")]
pub enum KillWho {
    /// If "main", only the main process of a unit is killed.
    #[enum_value(name = "main", nick = "Main"/* "Only the main unit's process" */)]
    Main,

    ///If "control" only the control process of the unit is killed
    /// A "control" process is for example a process that is configured via ExecStop= and is spawned in parallel to the main daemon process, in order to shut it down.
    #[enum_value(name = "control", nick = "Control"/* "Only the unit's controled processes" */)]
    Control,

    ///If "all" all processes are killed.
    #[enum_value(name = "all", nick =  "All" /* "All unit's processes" */)]
    All,
}

impl KillWho {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Main => "main",
            Self::Control => "control",
            Self::All => "all",
        }
    }

    pub fn description(&self) -> &str {
        match self {
            Self::Main => "Only the main unit's process",
            Self::Control => "Only the unit's controled processes",
            Self::All => "All unit's processes",
        }
    }
}

impl Display for KillWho {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value: glib::Value = self.to_value();

        let out = if let Some(enum_value) = EnumValue::from_value(&value) {
            enum_value.1.name()
        } else {
            ""
        };

        write!(f, "{out}")
    }
}

impl From<i32> for KillWho {
    fn from(value: i32) -> Self {
        match value {
            0 => KillWho::Main,
            1 => KillWho::Control,
            2 => KillWho::All,
            _ => KillWho::Main,
        }
    }
}

impl From<u32> for KillWho {
    fn from(value: u32) -> Self {
        (value as i32).into()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, strum::EnumIter)]

pub enum DependencyType {
    #[default]
    Forward = 0,
    Reverse = 1,
    After = 2,
    Before = 3,
}

impl DependencyType {
    pub fn label(&self) -> String {
        match self {
            //menu option
            DependencyType::Forward => pgettext("dependency", "Forward"),
            //menu option
            DependencyType::Reverse => pgettext("dependency", "Reverse"),
            //menu option
            DependencyType::After => pgettext("dependency", "After"),
            //menu option
            DependencyType::Before => pgettext("dependency", "Before"),
        }
    }

    pub(super) fn properties(&self) -> &[&str] {
        let properties: &[&str] = match self {
            DependencyType::Forward => &[
                "Requires",
                "Requisite",
                "Wants",
                "ConsistsOf",
                "BindsTo",
                "Upholds",
            ],
            DependencyType::Reverse => &[
                "RequiredBy",
                "RequisiteOf",
                "WantedBy",
                "PartOf",
                "BoundBy",
                "UpheldBy",
            ],
            DependencyType::After => &["After"],
            DependencyType::Before => &["Before"],
        };
        properties
    }
}

impl From<u32> for DependencyType {
    fn from(dtype: u32) -> Self {
        match dtype {
            0 => DependencyType::Forward,
            1 => DependencyType::Reverse,
            2 => DependencyType::After,
            3 => DependencyType::Before,
            _ => DependencyType::Forward,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, glib::Enum, EnumIter)]
#[enum_type(name = "EnableUnitFileMode")]
//#[allow(dead_code)]
pub enum StartStopMode {
    ///If "replace" the call will start the unit and its dependencies,
    /// possibly replacing already queued jobs that conflict with this.
    #[enum_value(name = "replace")]
    Replace,

    ///If "fail" the call will start the unit and its dependencies, but will fail if this
    ///would change an already queued job.
    #[default]
    #[enum_value(name = "fail")]
    Fail,

    ///If "isolate" the call will start the unit in
    ///question and terminate all units that aren't dependencies of it.
    ///Note that "isolate" mode is invalid for method **StopUnit**.
    #[enum_value(name = "isolate")]
    Isolate,

    ///If "ignore-dependencies" it will start a unit but ignore all its dependencies.
    #[enum_value(name = "ignore-dependencies")]
    IgnoreDependencies,

    ///If "ignore-requirements" it will start a unit but only ignore the requirement dependencies.
    #[enum_value(name = "ignore-requirements")]
    IgnoreRequirements,
}

impl StartStopMode {
    pub fn as_str(&self) -> &'static str {
        let enum_value: &glib::EnumValue = self.to_value().get().expect("it's an enum");
        enum_value.name()
    }

    pub fn discriminant(&self) -> u32 {
        let enum_value: &glib::EnumValue = self.to_value().get().expect("it's an enum");
        enum_value.value() as u32
    }
}

impl From<&RefCell<String>> for StartStopMode {
    fn from(value: &RefCell<String>) -> Self {
        let borrowed = value.borrow();
        StartStopMode::from(borrowed.as_str())
    }
}

impl From<&str> for StartStopMode {
    fn from(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "fail" => StartStopMode::Fail,
            "replace" => StartStopMode::Replace,
            "isolate" => StartStopMode::Isolate,
            "ignore-dependencies" => StartStopMode::IgnoreDependencies,
            "ignore-requirements" => StartStopMode::IgnoreRequirements,

            unknown => {
                warn!("unknown start mode {unknown:?}");
                StartStopMode::default()
            }
        }
    }
}

impl From<&glib::Variant> for StartStopMode {
    fn from(value: &glib::Variant) -> Self {
        let Some(value) = value.get::<String>() else {
            warn!("Variant not String");
            return StartStopMode::Fail;
        };

        StartStopMode::from(value.as_str())
    }
}

impl From<glib::Variant> for StartStopMode {
    fn from(value: glib::Variant) -> Self {
        StartStopMode::from(&value)
    }
}

/* impl From<Option<glib::Object>> for StartStopMode {
    fn from(value: Option<glib::Object>) -> Self {
        let Some(object) = value else {
            return StartStopMode::default();
        };

        let enum_list_item = object
            .downcast::<adw::EnumListItem>()
            .expect("Needs to be EnumListItem");

        StartStopMode::from(enum_list_item.name().as_str())
    }
} */

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone)]
pub enum DisEnableFlags {
    SdSystemdUnitRuntime = 1,
    SdSystemdUnitForce = 1 << 1,
    SdSystemdUnitPortable = 1 << 2,
}

impl DisEnableFlags {
    pub fn as_u64(&self) -> u64 {
        self.bits() as u64
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, EnumIter)]
pub enum CleanOption {
    Runtime,
    State,
    Cache,
    Logs,
    Configuration,
    Fdstore,
    All,
}

impl CleanOption {
    pub fn label(&self) -> String {
        match &self {
            //clean options
            CleanOption::Runtime => pgettext("clean", "_Runtime"),
            //clean options
            CleanOption::State => pgettext("clean", "_State"),
            //clean options
            CleanOption::Cache => pgettext("clean", "Cac_he"),
            //clean options
            CleanOption::Logs => pgettext("clean", "_Logs"),
            //clean options
            CleanOption::Configuration => pgettext("clean", "_Configuration"),
            //clean options
            CleanOption::Fdstore => pgettext("clean", "_File Descriptor Store"),
            //clean options
            CleanOption::All => pgettext("clean", "_All"),
        }
    }

    pub fn code(&self) -> &str {
        match &self {
            CleanOption::Runtime => "runtime",
            CleanOption::State => "state",
            CleanOption::Cache => "cache",
            CleanOption::Logs => "logs",
            CleanOption::Configuration => "configuration",
            CleanOption::Fdstore => "fdstore",
            CleanOption::All => "all",
        }
    }
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, EnumIter, Hash, Default, Ord, PartialOrd, glib::Enum,
)]
#[enum_type(name = "LoadState")]
pub enum LoadState {
    #[default]
    Unknown,
    Loaded,
    NotFound,
    BadSetting,
    Error,
    Masked,
}

impl LoadState {
    pub fn as_str(&self) -> &'static str {
        match self {
            LoadState::Unknown => "",
            LoadState::Loaded => "loaded",
            LoadState::NotFound => "not-found",
            LoadState::BadSetting => "bad-setting",
            LoadState::Error => "error",
            LoadState::Masked => "masked",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            LoadState::Unknown => "<i>not set</i>",
            _ => self.as_str(),
        }
    }

    pub fn tooltip_info(&self) -> Option<&str> {
        None
    }
}

impl From<String> for LoadState {
    fn from(value: String) -> Self {
        Some(value.as_str()).into()
    }
}

impl From<&str> for LoadState {
    fn from(value: &str) -> Self {
        Some(value).into()
    }
}

impl From<Option<&str>> for LoadState {
    fn from(value: Option<&str>) -> Self {
        match value {
            Some("loaded") => LoadState::Loaded,
            Some("not-found") => LoadState::NotFound,
            Some("bad-setting") => LoadState::BadSetting,
            Some("error") => LoadState::Error,
            Some("masked") => LoadState::Masked,
            _ => LoadState::Unknown,
        }
    }
}

impl From<Option<String>> for LoadState {
    fn from(value: Option<String>) -> Self {
        match value {
            Some(s) => s.as_str().into(),
            None => LoadState::Unknown,
        }
    }
}

impl From<Option<&OwnedValue>> for LoadState {
    fn from(value: Option<&OwnedValue>) -> Self {
        let value: Option<&zvariant::Value> = value.map(|v| &**v);
        match value {
            Some(zvariant::Value::Str(zvalue)) => zvalue.as_str().into(),
            _ => LoadState::Unknown,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, glib::Enum, EnumIter)]
#[enum_type(name = "StrMatchType")]
//#[allow(dead_code)]
pub enum StrMatchType {
    #[default]
    #[enum_value(name = "contains")]
    Contains,

    #[enum_value(name = "start_with")]
    StartWith,

    #[enum_value(name = "end_with")]
    EndWith,

    #[enum_value(name = "equals")]
    Equals,
}

impl StrMatchType {
    pub fn as_str(&self) -> &'static str {
        let enum_value: &glib::EnumValue = self.to_value().get().expect("it's an enum");
        enum_value.name()
    }

    pub fn position(&self) -> u32 {
        match self {
            StrMatchType::Contains => 0,
            StrMatchType::StartWith => 1,
            StrMatchType::EndWith => 2,
            StrMatchType::Equals => 3,
        }
    }
}

impl From<u32> for StrMatchType {
    fn from(value: u32) -> Self {
        for (idx, mt) in StrMatchType::iter().enumerate() {
            if idx == value as usize {
                return mt;
            }
        }
        StrMatchType::default()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, glib::Enum, EnumIter)]
#[enum_type(name = "NumMatchType")]
//#[allow(dead_code)]
pub enum NumMatchType {
    #[default]
    #[enum_value(name = "equals")]
    Equals,

    #[enum_value(name = "greater")]
    Greater,

    #[enum_value(name = "smaller")]
    Smaller,

    #[enum_value(name = "greater equals")]
    GreaterEquals,

    #[enum_value(name = "smaller equals")]
    SmallerEquals,
}

impl NumMatchType {
    pub fn as_str(&self) -> &'static str {
        let enum_value: &glib::EnumValue = self.to_value().get().expect("it's an enum");
        enum_value.name()
    }

    pub fn position(&self) -> u32 {
        match self {
            NumMatchType::Equals => 0,
            NumMatchType::Greater => 1,
            NumMatchType::Smaller => 2,
            NumMatchType::GreaterEquals => 3,
            &NumMatchType::SmallerEquals => 4,
        }
    }
}

impl From<u32> for NumMatchType {
    fn from(value: u32) -> Self {
        for (idx, mt) in NumMatchType::iter().enumerate() {
            if idx == value as usize {
                return mt;
            }
        }
        NumMatchType::default()
    }
}

#[cfg(test)]
mod tests {

    use base::enums::UnitDBusLevel;

    use super::*;

    #[test]
    fn test_kill_who_glib() {
        assert_kill(KillWho::All);
        assert_kill(KillWho::Main);
        assert_kill(KillWho::Control);
    }

    fn assert_kill(kill: KillWho) {
        assert_eq!(kill.as_str(), kill.to_string())
    }

    #[test]
    fn test_unit_level() {
        fn test(ul: UnitDBusLevel) {
            let num_val = ul.value();
            let ul2: UnitDBusLevel = (num_val as u8).into();
            assert_eq!(ul, ul2);

            let str_val = ul.as_str();
            let ul3: UnitDBusLevel = str_val.into();
            assert_eq!(ul, ul3);
        }
        test(UnitDBusLevel::System);
        test(UnitDBusLevel::UserSession);
    }
}
