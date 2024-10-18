use gtk::{
    gio::{self, Settings},
    glib::GString,
    prelude::SettingsExt,
};
use log::info;

use std::sync::{LazyLock, RwLock};

use crate::systemd_gui;

pub static PREFERENCES: LazyLock<Preferences> = LazyLock::new(|| {
    let settings = gio::Settings::new(systemd_gui::APP_ID);
    let pref = Preferences::new_with_setting(&settings);

    pref
});

pub const KEY_DBUS_LEVEL: &str = "pref-dbus-level";
pub const KEY_PREF_JOURNAL_COLORS: &str = "pref-journal-colors";
pub const KEY_PREF_UNIT_FILE_HIGHLIGHTING: &str = "pref-unit-file-highlighting";

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum DbusLevel {
    #[default]
    Session = 0,
    System = 1,
}

impl DbusLevel {
    pub fn as_str(&self) -> &str {
        match self {
            DbusLevel::Session => "Session",
            DbusLevel::System => "System",
        }
    }
}

impl From<GString> for DbusLevel {
    fn from(level: GString) -> Self {
        level.as_str().into()
    }
}

impl From<&str> for DbusLevel {
    fn from(level: &str) -> Self {
        if "System".eq(level) {
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

pub struct Preferences {
    dbus_level: RwLock<DbusLevel>,
    journal_colors: RwLock<bool>,
    unit_file_colors: RwLock<bool>,
}

impl Preferences {
    pub fn new_with_setting(settings: &Settings) -> Self {
        let level = settings.string(KEY_DBUS_LEVEL).into();
        let journal_colors = settings.boolean(KEY_PREF_JOURNAL_COLORS);
        let unit_file_colors = settings.boolean(KEY_PREF_UNIT_FILE_HIGHLIGHTING);

        Preferences {
            dbus_level: RwLock::new(level),
            journal_colors: RwLock::new(journal_colors),
            unit_file_colors: RwLock::new(unit_file_colors),
        }
    }

    pub fn dbus_level(&self) -> DbusLevel {
        *self.dbus_level.read().unwrap()
    }

    pub fn journal_colors(&self) -> bool {
        *self.journal_colors.read().unwrap()
    }

    pub fn unit_file_colors(&self) -> bool {
        *self.unit_file_colors.read().unwrap()
    }

    pub fn set_dbus_level(&self, dbus_level: DbusLevel) {
        info!("set_dbus_level: {}", dbus_level.as_str());

        let mut self_dbus_level = self.dbus_level.write().expect("supposed to write");
        *self_dbus_level = dbus_level;
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
