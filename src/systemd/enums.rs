use super::sysdbus::INTERFACE_SYSTEMD_MANAGER;
use super::sysdbus::INTERFACE_SYSTEMD_UNIT;
use bitflags::bitflags;
use gettextrs::pgettext;
use gtk::glib::{self, EnumValue};
use gtk::prelude::*;
use log::{info, warn};
use std::cmp::Ordering;
use std::{cell::RefCell, fmt::Display};
use strum::EnumIter;
use zvariant::OwnedValue;

const MASKED: &str = "masked";
const ENABLED: &str = "enabled";
const LINKED: &str = "linked";

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, Default, Hash)]
pub enum Preset {
    #[default]
    UnSet = 0,
    Enabled = 1,
    Disabled = 2,
    Ignore = 3,
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

    pub fn label(&self) -> &'static str {
        match self {
            Preset::UnSet => "<i>not set</i>",
            _ => self.as_str(),
        }
    }

    pub fn discriminant(&self) -> u8 {
        // SAFETY: Because `Self` is marked `repr(u8)`, its layout is a `repr(C)` `union`
        // between `repr(C)` structs, each of which has the `u8` discriminant as its first
        // field, so we can read the discriminant without offsetting the pointer.
        unsafe { *<*const _>::from(self).cast::<u8>() }
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

impl From<u8> for Preset {
    fn from(value: u8) -> Self {
        match value {
            0 => Preset::UnSet,
            1 => Preset::Disabled,
            2 => Preset::Enabled,
            3 => Preset::Ignore,
            _ => Preset::UnSet,
        }
    }
}

impl PartialOrd for Preset {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Preset {
    fn cmp(&self, other: &Self) -> Ordering {
        let value = self.discriminant();
        let other_value = other.discriminant();
        value.cmp(&other_value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, Default, Hash)]
pub enum EnablementStatus {
    #[default]
    Unknown = 0,
    Alias = 1,
    Bad = 2,
    Disabled = 3,
    Enabled = 4,
    EnabledRuntime = 11,
    Generated = 5,
    Indirect = 6,
    Linked = 7,
    LinkedRuntime = 12,
    Masked = 8,
    MaskedRuntime = 13,
    Static = 9,
    Trancient = 10,
}

impl EnablementStatus {
    /// Takes the string containing the state information from the dbus message and converts it
    /// into a UnitType by matching the first character.
    pub fn from_str(enablement_status: &str) -> EnablementStatus {
        if enablement_status.is_empty() {
            info!("Empty Enablement Status: \"{}\"", enablement_status);
            return EnablementStatus::Unknown;
        }

        let c = enablement_status.chars().next().unwrap();

        match c {
            'a' => EnablementStatus::Alias,
            's' => EnablementStatus::Static,
            'd' => EnablementStatus::Disabled,
            'e' => {
                if enablement_status.len() == ENABLED.len() {
                    EnablementStatus::Enabled
                } else {
                    EnablementStatus::EnabledRuntime
                }
            }
            'i' => EnablementStatus::Indirect,
            'l' => {
                if enablement_status.len() == LINKED.len() {
                    EnablementStatus::Linked
                } else {
                    EnablementStatus::LinkedRuntime
                }
            }
            'm' => {
                if enablement_status.len() == MASKED.len() {
                    EnablementStatus::Masked
                } else {
                    EnablementStatus::MaskedRuntime
                }
            }
            'b' => EnablementStatus::Bad,
            'g' => EnablementStatus::Generated,
            't' => EnablementStatus::Trancient,
            _ => {
                warn!("Unknown State: {}", enablement_status);
                EnablementStatus::Unknown
            }
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            EnablementStatus::Alias => "alias",
            EnablementStatus::Bad => "bad",
            EnablementStatus::Disabled => "disabled",
            EnablementStatus::Enabled => ENABLED,
            EnablementStatus::Indirect => "indirect",
            EnablementStatus::Linked => LINKED,
            EnablementStatus::Masked => MASKED,
            EnablementStatus::Static => "static",
            EnablementStatus::Generated => "generated",
            EnablementStatus::Trancient => "trancient",
            EnablementStatus::EnabledRuntime => "enabled-runtime",
            EnablementStatus::LinkedRuntime => "linked-runtime",
            EnablementStatus::MaskedRuntime => "masked-runtime",
            EnablementStatus::Unknown => "",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            EnablementStatus::Unknown => "<i>unset</i>",
            _ => self.as_str(),
        }
    }

    pub fn tooltip_info(&self) -> Option<String> {
        match self {
            EnablementStatus::Alias => Some(pgettext(
                "list",
                "The name is an alias (symlink to another unit file).",
            )),
            EnablementStatus::Bad => Some(pgettext(
                "list",
                "The unit file is invalid or another error occurred.",
            )),
            EnablementStatus::Disabled => Some(pgettext(
                "list",
                "The unit file is not enabled, but contains an [Install] section with installation instructions.",
            )),
            EnablementStatus::Enabled => Some(pgettext(
                "list",
                "Enabled via <span fgcolor='#62a0ea'>.wants/</span>, <span fgcolor='#62a0ea'>.requires/</span> or <u>Alias=</u> symlinks (permanently in <span fgcolor='#62a0ea'>/etc/systemd/system/</span>, or transiently in <span fgcolor='#62a0ea'>/run/systemd/system/</span>).",
            )),
            EnablementStatus::Generated => Some(pgettext(
                "list",
                "The unit file was generated dynamically via a generator tool. See <b>man systemd.generator(7)</b>. Generated unit files may not be enabled, they are enabled implicitly by their generator.",
            )),
            EnablementStatus::Indirect => Some(pgettext(
                "list",
                "The unit file itself is not enabled, but it has a non-empty <u>Also=</u> setting in the [Install] unit file section, listing other unit files that might be enabled, or it has an alias under a different name through a symlink that is not specified in <u>Also=</u>. For template unit files, an instance different than the one specified in <u>DefaultInstance=</u> is enabled.",
            )),
            EnablementStatus::Linked => Some(pgettext(
                "list",
                "Made available through one or more symlinks to the unit file (permanently in <span fgcolor='#62a0ea'>/etc/systemd/system/</span> or transiently in <span fgcolor='#62a0ea'>/run/systemd/system/</span>), even though the unit file might reside outside of the unit file search path.",
            )),
            EnablementStatus::Masked => Some(pgettext(
                "list",
                "Completely disabled, so that any start operation on it fails (permanently in <span fgcolor='#62a0ea'>/etc/systemd/system/</span> or transiently in <span fgcolor='#62a0ea'>/run/systemd/systemd/</span>).",
            )),
            EnablementStatus::Static => Some(pgettext(
                "list",
                "The unit file is not enabled, and has no provisions for enabling in the [Install] unit file section.",
            )),
            EnablementStatus::Trancient => Some(pgettext(
                "list",
                "The unit file has been created dynamically with the runtime API. Transient units may not be enabled.",
            )),
            EnablementStatus::Unknown => None,
            EnablementStatus::EnabledRuntime => Some(pgettext(
                "list",
                "Enabled via <span fgcolor='#62a0ea'>.wants/</span>, <span fgcolor='#62a0ea'>.requires/</span> or <u>Alias=</u> symlinks (permanently in <span fgcolor='#62a0ea'>/etc/systemd/system/</span>, or transiently in <span fgcolor='#62a0ea'>/run/systemd/system/</span>).",
            )),
            EnablementStatus::LinkedRuntime => Some(pgettext(
                "list",
                "Made available through one or more symlinks to the unit file (permanently in <span fgcolor='#62a0ea'>/etc/systemd/system/</span> or transiently in <span fgcolor='#62a0ea'>/run/systemd/system/</span>), even though the unit file might reside outside of the unit file search path.",
            )),
            EnablementStatus::MaskedRuntime => Some(pgettext(
                "list",
                "Completely disabled, so that any start operation on it fails (permanently in <span fgcolor='#62a0ea'>/etc/systemd/system/</span> or transiently in <span fgcolor='#62a0ea'>/run/systemd/systemd/</span>).",
            )),
        }
    }
}

impl Display for EnablementStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{}", self.as_str())
    }
}

impl From<Option<String>> for EnablementStatus {
    fn from(value: Option<String>) -> Self {
        if let Some(str_val) = value {
            return EnablementStatus::from_str(&str_val);
        }
        EnablementStatus::Unknown
    }
}

impl From<&str> for EnablementStatus {
    fn from(value: &str) -> Self {
        EnablementStatus::from_str(value)
    }
}

impl From<EnablementStatus> for u8 {
    fn from(value: EnablementStatus) -> Self {
        value as u8
    }
}

impl From<u8> for EnablementStatus {
    fn from(value: u8) -> Self {
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
            11 => Self::EnabledRuntime,
            12 => Self::LinkedRuntime,
            13 => Self::MaskedRuntime,

            _ => Self::Unknown,
        }
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, EnumIter, Hash)]
#[repr(u8)]
pub enum ActiveState {
    Unknown = 0,
    Active = 1,
    Reloading = 2,
    #[default]
    Inactive = 3,
    Failed = 4,
    Activating = 5,
    Deactivating = 6,
    Maintenance = 7,
    Refreshing = 8,
}

impl ActiveState {
    pub fn discriminant(&self) -> u8 {
        // SAFETY: Because `Self` is marked `repr(u8)`, its layout is a `repr(C)` `union`
        // between `repr(C)` structs, each of which has the `u8` discriminant as its first
        // field, so we can read the discriminant without offsetting the pointer.
        unsafe { *<*const _>::from(self).cast::<u8>() }
    }

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

    pub(crate) fn glyph_str(&self) -> &str {
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

impl PartialOrd for ActiveState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ActiveState {
    fn cmp(&self, other: &Self) -> Ordering {
        let value = self.discriminant();
        let other_value = other.discriminant();
        value.cmp(&other_value)
    }
}

impl Display for ActiveState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
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

impl From<u8> for ActiveState {
    fn from(value: u8) -> Self {
        let state: ActiveState = (value as u32).into();
        state
    }
}

impl From<u32> for ActiveState {
    fn from(value: u32) -> Self {
        match value {
            0 => Self::Unknown,
            1 => Self::Active,
            2 => Self::Reloading,
            3 => Self::Inactive,
            4 => Self::Failed,
            5 => Self::Activating,
            6 => Self::Deactivating,
            7 => Self::Maintenance,
            8 => Self::Refreshing,
            _ => Self::Unknown,
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
            _ => {
                warn!("Unknown Unit Type: {}", unit_type);
                UnitType::Unknown(unit_type.to_string())
            }
        }
    }

    pub fn as_str(&self) -> &str {
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

            _ => INTERFACE_SYSTEMD_UNIT,
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

    pub(crate) fn description(&self) -> &str {
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

        write!(f, "{}", out)
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, glib::Enum)]
#[enum_type(name = "DependencyType")]
pub enum DependencyType {
    #[enum_value(name = "forward", nick = "Forward")]
    #[default]
    Forward = 0,
    #[enum_value(name = "reverse", nick = "Reverse")]
    Reverse = 1,
    #[enum_value(name = "after", nick = "After")]
    After = 2,
    #[enum_value(name = "before", nick = "Before")]
    Before = 3,
}

impl DependencyType {
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
                warn!("unknown start mode {:?}", unknown);
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

impl From<Option<glib::Object>> for StartStopMode {
    fn from(value: Option<glib::Object>) -> Self {
        let Some(object) = value else {
            return StartStopMode::default();
        };

        let enum_list_item = object
            .downcast::<adw::EnumListItem>()
            .expect("Needs to be EnumListItem");

        StartStopMode::from(enum_list_item.name().as_str())
    }
}

#[derive(
    Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Default, Hash, EnumIter, glib::Enum,
)]
#[enum_type(name = "UnitDBusLevel")]
pub enum UnitDBusLevel {
    #[default]
    #[enum_value(name = "system", nick = "System")]
    System = 0,
    #[enum_value(name = "user", nick = "User Session")]
    UserSession = 1,
}

impl UnitDBusLevel {
    pub fn short(&self) -> &str {
        match self {
            UnitDBusLevel::System => "s",
            UnitDBusLevel::UserSession => "u",
        }
    }

    pub fn as_str(&self) -> &'static str {
        let level_value: &glib::EnumValue = self.to_value().get().expect("it's an enum");
        level_value.name()
    }

    //used in browser table
    pub fn label(&self) -> &'static str {
        self.as_str()
    }

    pub fn nice_label(&self) -> String {
        match self {
            UnitDBusLevel::UserSession => pgettext("dbus", "User Session"),
            UnitDBusLevel::System => pgettext("dbus", "System"),
        }
    }

    pub(crate) fn from_short(suffix: &str) -> Self {
        match suffix {
            "s" => UnitDBusLevel::System,
            "u" => UnitDBusLevel::UserSession,
            _ => UnitDBusLevel::System,
        }
    }

    pub fn tooltip_info(&self) -> Option<&str> {
        None
    }

    pub fn value(&self) -> i32 {
        let level_value: &glib::EnumValue = self.to_value().get().expect("it's an enum");
        level_value.value()
    }
}

impl From<u8> for UnitDBusLevel {
    fn from(level: u8) -> Self {
        match level {
            0 => UnitDBusLevel::System,
            _ => UnitDBusLevel::UserSession,
        }
    }
}

impl From<&str> for UnitDBusLevel {
    fn from(level: &str) -> Self {
        match level {
            "user" => UnitDBusLevel::UserSession,
            "system" => UnitDBusLevel::System,
            _ => {
                warn!("Unit dbus Level not found {:?}", level);
                UnitDBusLevel::default()
            }
        }
    }
}

impl From<Option<glib::Object>> for UnitDBusLevel {
    fn from(value: Option<glib::Object>) -> Self {
        let Some(object) = value else {
            return UnitDBusLevel::default();
        };

        let enum_list_item = object
            .downcast::<adw::EnumListItem>()
            .expect("Needs to be EnumListItem");

        UnitDBusLevel::from(enum_list_item.name().as_str())
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct DisEnableFlags : u8{
      const SD_SYSTEMD_UNIT_RUNTIME  = 1;
      const SD_SYSTEMD_UNIT_FORCE    = 1 << 1;
      const SD_SYSTEMD_UNIT_PORTABLE = 1 << 2;
    }
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
            CleanOption::Runtime => pgettext("clean", "_Runtime"),
            CleanOption::State => pgettext("clean", "_State"),
            CleanOption::Cache => pgettext("clean", "Cac_he"),
            CleanOption::Logs => pgettext("clean", "_Logs"),
            CleanOption::Configuration => pgettext("clean", "_Configuration"),
            CleanOption::Fdstore => pgettext("clean", "_File Descriptor Store"),
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, Hash, Default, Ord, PartialOrd)]
pub enum LoadState {
    #[default]
    Unknown = 0,
    Loaded = 1,
    NotFound = 2,
    BadSetting = 3,
    Error = 4,
    Masked = 5,
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

    pub fn discriminant(&self) -> u8 {
        // SAFETY: Because `Self` is marked `repr(u8)`, its layout is a `repr(C)` `union`
        // between `repr(C)` structs, each of which has the `u8` discriminant as its first
        // field, so we can read the discriminant without offsetting the pointer.
        unsafe { *<*const _>::from(self).cast::<u8>() }
    }

    pub fn tooltip_info(&self) -> Option<&str> {
        None
    }
}

impl From<&str> for LoadState {
    fn from(value: &str) -> Self {
        match value {
            "loaded" => LoadState::Loaded,
            "not-found" => LoadState::NotFound,
            "bad-setting" => LoadState::BadSetting,
            "error" => LoadState::Error,
            "masked" => LoadState::Masked,
            _ => LoadState::Unknown,
        }
    }
}

impl From<u8> for LoadState {
    fn from(value: u8) -> Self {
        match value {
            1 => LoadState::Loaded,
            2 => LoadState::NotFound,
            3 => LoadState::BadSetting,
            4 => LoadState::Error,
            5 => LoadState::Masked,
            _ => LoadState::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {

    use strum::IntoEnumIterator;

    use super::*;

    #[test]
    fn test_enablement_status_any_number() {
        let num: u32 = 1000;
        let status: EnablementStatus = (num as u8).into();
        assert_eq!(status, EnablementStatus::Unknown)
    }

    #[test]
    fn test_enablement_status_mapping() {
        for status in EnablementStatus::iter() {
            assert_num_mapping_enablement_status(status);
        }
    }

    fn assert_num_mapping_enablement_status(status: EnablementStatus) {
        let val: u8 = status.into();
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

        for state in ActiveState::iter() {
            assert_num_mapping_active_state(state);
        }
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

    #[test]
    fn test_preset() {
        assert_eq!(Preset::UnSet.discriminant(), 0);
        assert_eq!(Preset::Enabled.discriminant(), 1);
        assert_eq!(Preset::Disabled.discriminant(), 2);
        assert_eq!(Preset::Ignore.discriminant(), 3);
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
