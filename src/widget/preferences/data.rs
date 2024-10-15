use gtk::{
    gio::{self, Settings},
    glib::{self, GString},
    prelude::*,
};
use log::{info, warn};

use std::sync::{LazyLock, RwLock};

use crate::{errors::SysDManagerErrors, systemd_gui};

pub static PREFERENCES: LazyLock<Preferences> = LazyLock::new(|| {
    let settings = get_settings();
    let pref = Preferences::new_with_setting(&settings);

    pref
});

const KEY_DBUS_LEVEL: &str = "pref-dbus-level";
const KEY_PREF_JOURNAL_COLORS: &str = "pref-journal-colors";

pub fn build_preferences() -> Result<adw::PreferencesDialog, SysDManagerErrors> {
    let builder = gtk::Builder::from_resource("/io/github/plrigaux/sysd-manager/preferences.ui");

    let id_name = "preferences";
    let Some(window) = builder.object::<adw::PreferencesDialog>(id_name) else {
        return Err(SysDManagerErrors::GTKBuilderObjectNotfound(
            id_name.to_owned(),
        ));
    };

    let id_name = "dbus_level_dropdown";
    let Some(dbus_level_dropdown) = builder.object::<gtk::DropDown>(id_name) else {
        return Err(SysDManagerErrors::GTKBuilderObjectNotfound(
            id_name.to_owned(),
        ));
    };
    let settings = get_settings();
    {
        let settings = settings.clone();
        dbus_level_dropdown.connect_selected_notify(move |toggle_button| {
            let idx = toggle_button.selected();
            info!("Values Selected {:?}", idx);

            let level: DbusLevel = idx.into();

            //let settings = get_settings();
            if let Err(e) = set_dbus_level(&settings, level) {
                warn!("Error: {:?}", e);
                return;
            }
            info!(
                "Save setting '{KEY_DBUS_LEVEL}' with value {:?}",
                level.as_str()
            )
        });
    }

 

    let id_name = "journal_colors";
    let Some(journal_colors_switch) = builder.object::<gtk::Switch>(id_name) else {
        return Err(SysDManagerErrors::GTKBuilderObjectNotfound(
            id_name.to_owned(),
        ));
    };
    {
        let settings = settings.clone();
        journal_colors_switch.connect_state_notify(move |toggle_switch| {
            let state = toggle_switch.state();
            info!("Switch state {:?}", state);
            let _ = set_journal_colors(&settings, state);
        });
    }


    let level = PREFERENCES.dbus_level();
    let journal_colors = PREFERENCES.journal_colors();

    dbus_level_dropdown.set_selected(level as u32);


    journal_colors_switch.set_state(journal_colors);
    journal_colors_switch.set_active(journal_colors);

    Ok(window)
}

fn get_settings() -> Settings {
    gio::Settings::new(systemd_gui::APP_ID)
}

fn set_dbus_level(settings: &Settings, level: DbusLevel) -> Result<(), glib::BoolError> {
    let res = settings.set_string(KEY_DBUS_LEVEL, level.as_str());
    PREFERENCES.set_dbus_level(level);

    res
}

fn set_journal_colors(settings: &Settings, display: bool) -> Result<(), glib::BoolError> {
    let res = settings.set_boolean(KEY_PREF_JOURNAL_COLORS, display);
    PREFERENCES.set_journal_colors(display);

    res
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum DbusLevel {
    #[default]
    Session = 0,
    System = 1,
}

impl DbusLevel {
    fn as_str(&self) -> &str {
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
}

impl Preferences {
    pub fn dbus_level(&self) -> DbusLevel {
        *self.dbus_level.read().unwrap()
    }

    pub fn journal_colors(&self) -> bool {
        *self.journal_colors.read().unwrap()
    }

    pub fn new_with_setting(settings: &Settings) -> Self {
        let level = settings.string(KEY_DBUS_LEVEL).into();
        let journal_colors = settings.boolean(KEY_PREF_JOURNAL_COLORS);

        Preferences {
            dbus_level: RwLock::new(level),
            journal_colors: RwLock::new(journal_colors),
        }
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
