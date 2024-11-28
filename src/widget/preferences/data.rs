use gtk::{
    gio::{self, Settings},
    glib::{self, GString},
    prelude::{SettingsExt, ToValue},
};
use log::{debug, info};

use std::sync::{LazyLock, RwLock};

use crate::systemd_gui;

pub static PREFERENCES: LazyLock<Preferences> = LazyLock::new(|| {
    let settings = gio::Settings::new(systemd_gui::APP_ID);
    let pref = Preferences::new_with_setting(&settings);

    pref
});

pub const KEY_DBUS_LEVEL: &str = "pref-dbus-level";
pub const KEY_PREF_JOURNAL_COLORS: &str = "pref-journal-colors";
pub const KEY_PREF_JOURNAL_MAX_EVENTS: &str = "pref-journal-max-events";
pub const KEY_PREF_UNIT_FILE_HIGHLIGHTING: &str = "pref-unit-file-highlighting";
pub const KEY_PREF_APP_FIRST_CONNECTION: &str = "pref-app-first-connection";

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, glib::Enum)]
#[enum_type(name = "DbusLevel")]
pub enum DbusLevel {
    #[enum_value(name = "session", nick = "Session Bus")]
    #[default]
    Session = 0,
    #[enum_value(name = "system", nick = "System Bus")]
    System = 1,
}

impl DbusLevel {
    pub fn as_str(&self) -> &str {
        let level_value: &glib::EnumValue = self.to_value().get().expect("it's an enum");

        level_value.name()
    }
}

impl From<GString> for DbusLevel {
    fn from(level: GString) -> Self {
        level.as_str().into()
    }
}

impl From<&str> for DbusLevel {
    fn from(level: &str) -> Self {
        if "system".eq(&level.to_lowercase()) {
            DbusLevel::System
        } else {
            DbusLevel::Session
        }
    }
}

impl From<u32> for DbusLevel {
    fn from(level: u32) -> Self {
        match level {
            1 => DbusLevel::System,
            _ => DbusLevel::Session,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, glib::Enum)]
#[enum_type(name = "EnableUnitFileMode")]
pub enum EnableUnitFileMode {
    #[default]
    Command = 0,
    DBus = 1,
}

impl EnableUnitFileMode {
    pub fn as_str(&self) -> &str {
        match self {
            EnableUnitFileMode::Command => "Subprocess call",
            EnableUnitFileMode::DBus => "D-bus call",
        }
    }
}

impl From<GString> for EnableUnitFileMode {
    fn from(level: GString) -> Self {
        level.as_str().into()
    }
}

impl From<&str> for EnableUnitFileMode {
    fn from(level: &str) -> Self {
        if "System".eq(level) {
            EnableUnitFileMode::Command
        } else {
            EnableUnitFileMode::DBus
        }
    }
}

impl From<u32> for EnableUnitFileMode {
    fn from(level: u32) -> Self {
        match level {
            1 => EnableUnitFileMode::DBus,
            _ => EnableUnitFileMode::Command,
        }
    }
}

pub struct Preferences {
    dbus_level: RwLock<DbusLevel>,
    journal_colors: RwLock<bool>,
    journal_events: RwLock<u32>,
    unit_file_colors: RwLock<bool>,
    app_first_connection: RwLock<bool>,
}

impl Preferences {
    pub fn new_with_setting(settings: &Settings) -> Self {
        let level_str = settings.string(KEY_DBUS_LEVEL);
        let level = settings.string(KEY_DBUS_LEVEL).into();
        debug!("level {:?} {:?}", level_str, level);
        let journal_colors = settings.boolean(KEY_PREF_JOURNAL_COLORS);
        let journal_events = settings.uint(KEY_PREF_JOURNAL_MAX_EVENTS);

        let unit_file_colors = settings.boolean(KEY_PREF_UNIT_FILE_HIGHLIGHTING);
        let app_first_connection = settings.boolean(KEY_PREF_APP_FIRST_CONNECTION);

        Preferences {
            dbus_level: RwLock::new(level),
            journal_colors: RwLock::new(journal_colors),
            journal_events: RwLock::new(journal_events),
            unit_file_colors: RwLock::new(unit_file_colors),
            app_first_connection: RwLock::new(app_first_connection),
        }
    }

    pub fn dbus_level(&self) -> DbusLevel {
        *self.dbus_level.read().unwrap()
    }

    pub fn journal_colors(&self) -> bool {
        *self.journal_colors.read().unwrap()
    }

    pub fn journal_events(&self) -> u32 {
        *self.journal_events.read().unwrap()
    }

    pub fn unit_file_colors(&self) -> bool {
        *self.unit_file_colors.read().unwrap()
    }

    pub fn is_app_first_connection(&self) -> bool {
        *self.app_first_connection.read().unwrap()
    }

    pub fn set_dbus_level(&self, dbus_level: DbusLevel) {
        info!("set_dbus_level: {}", dbus_level.as_str());

        let mut self_dbus_level = self.dbus_level.write().expect("supposed to write");
        *self_dbus_level = dbus_level;
    }

    pub fn set_journal_events(&self, journal_events_new: u32) {
        info!("set_journal_events: {journal_events_new}");

        let mut journal_events = self.journal_events.write().expect("supposed to write");
        *journal_events = journal_events_new;
    }

    pub fn set_journal_colors(&self, display: bool) {
        info!("set_journal_colors: {display}");

        let mut journal_colors = self.journal_colors.write().expect("supposed to write");
        *journal_colors = display;
    }

    pub fn set_unit_file_highlighting(&self, display: bool) {
        info!("set_unit_file_highlighting: {display}");

        let mut unit_file_colors = self.unit_file_colors.write().expect("supposed to write");
        *unit_file_colors = display;
    }

    pub fn set_app_first_connection(&self, app_first_connection_new: bool) {
        info!("set_app_first_connection: {app_first_connection_new}");

        let mut app_first_connection = self
            .app_first_connection
            .write()
            .expect("supposed to write");
        *app_first_connection = app_first_connection_new;
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_dbus_level_any_number() {
        assert_eq!(<u32 as Into<DbusLevel>>::into(1000), DbusLevel::Session)
    }

    #[test]
    fn test_dbus_level_int_mapping() {
        //assert_num_mapping(EnablementStatus::Unasigned);
        assert_num_mapping(DbusLevel::Session);
        assert_num_mapping(DbusLevel::System);
    }

    #[test]
    fn test_dbus_level_string_mapping() {
        //assert_num_mapping(EnablementStatus::Unasigned);
        assert_string_mapping(DbusLevel::Session, "Session");
        assert_string_mapping(DbusLevel::System, "System");
    }

    fn assert_num_mapping(level: DbusLevel) {
        let val = level as u32;
        let convert: DbusLevel = val.into();
        assert_eq!(convert, level)
    }

    fn assert_string_mapping(level: DbusLevel, key: &str) {
        let convert: DbusLevel = key.into();
        assert_eq!(convert, level)
    }
}
