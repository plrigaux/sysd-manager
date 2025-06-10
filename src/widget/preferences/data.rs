use gettextrs::pgettext;
use gtk::{
    gio::Settings,
    glib::{self, GString},
    pango::{self, FontDescription},
    prelude::SettingsExt,
};
use log::{info, warn};
use strum::EnumIter;

use std::sync::{LazyLock, RwLock};

use crate::{systemd_gui::new_settings, utils::th::TimestampStyle};

pub static PREFERENCES: LazyLock<Preferences> = LazyLock::new(|| {
    let settings = new_settings();
    Preferences::new_with_setting(settings)
});

const KEY_DBUS_LEVEL: &str = "pref-dbus-level";
pub const KEY_PREF_JOURNAL_COLORS: &str = "pref-journal-colors";
pub const KEY_PREF_JOURNAL_EVENTS_BATCH_SIZE: &str = "pref-journal-events-batch-size";
pub const KEY_PREF_JOURNAL_EVENT_MAX_SIZE: &str = "pref-journal-event-max-size";
pub const KEY_PREF_JOURNAL_DISPLAY_ORDER: &str = "pref-journal-display-order";
pub const KEY_PREF_JOURNAL_DISPLAY_FOLLOW: &str = "pref-journal-display-follow";
pub const KEY_PREF_UNIT_FILE_LINE_NUMBER: &str = "pref-unit-file-line-number";
pub const KEY_PREF_UNIT_FILE_STYLE_SCHEME: &str = "pref-unit-file-style-scheme";
pub const KEY_PREF_APP_FIRST_CONNECTION: &str = "pref-app-first-connection";
pub const KEY_PREF_TIMESTAMP_STYLE: &str = "pref-timestamp-style";
pub const KEY_PREF_STYLE_TEXT_FONT_FAMILY: &str = "pref-style-text-font-family";
pub const KEY_PREF_STYLE_TEXT_FONT_SIZE: &str = "pref-style-text-font-size";
pub const KEY_PREF_UNIT_LIST_DISPLAY_COLORS: &str = "pref-unit-list-display-colors";
pub const KEY_PREF_UNIT_LIST_DISPLAY_SUMMARY: &str = "pref-unit-list-display-summary";
pub const KEY_PREF_PREFERED_COLOR_SCHEME: &str = "pref-prefered-color-scheme";
pub const KEY_PREF_ORIENTATION_MODE: &str = "pref-window-orientaion-mode";

pub const FLAG_SHOW: u8 = 1;
pub const FLAG_WIDTH: u8 = 2;

pub const UNIT_LIST_COLUMNS_UNIT: u8 = 0;

pub const UNIT_LIST_COLUMNS: [(&str, &str, u8, u8); 9] = [
    ("Unit", "unit", UNIT_LIST_COLUMNS_UNIT, FLAG_WIDTH),
    ("Type", "type", 1, FLAG_SHOW | FLAG_WIDTH),
    ("Bus", "bus", 2, FLAG_SHOW | FLAG_WIDTH),
    ("State", "state", 3, FLAG_SHOW | FLAG_WIDTH),
    ("Preset", "preset", 4, FLAG_SHOW | FLAG_WIDTH),
    ("Load", "load", 5, FLAG_SHOW | FLAG_WIDTH),
    ("Active", "active", 6, FLAG_SHOW | FLAG_WIDTH),
    ("Sub", "sub", 7, FLAG_SHOW | FLAG_WIDTH),
    ("Description", "description", 8, FLAG_SHOW), //Because column "description" is expandable
];

pub const COL_SHOW_PREFIX: &str = "col-show-";
pub const COL_WIDTH_PREFIX: &str = "col-width-";

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, EnumIter)]
pub enum DbusLevel {
    #[default]
    UserSession = 0,
    System = 1,
    SystemAndSession = 2,
}

impl DbusLevel {
    pub fn as_str(&self) -> &str {
        match self {
            DbusLevel::UserSession => "session",
            DbusLevel::System => "system",
            DbusLevel::SystemAndSession => "system_session",
        }
    }

    /*     pub fn as_unit_dbus(&self) -> UnitDBusLevel {
           match self {
               DbusLevel::UserSession => UnitDBusLevel::UserSession,
               DbusLevel::System => UnitDBusLevel::System,
               DbusLevel::SystemAndSession => UnitDBusLevel::System,
           }
       }
    */
    pub fn label(&self) -> String {
        match self {
            DbusLevel::UserSession => pgettext("dbus", "User Session Bus"),
            DbusLevel::System => pgettext("dbus", "User Session Bus"),
            DbusLevel::SystemAndSession => pgettext("dbus", "System & User Session Bus"),
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
        match level.to_ascii_lowercase().as_str() {
            "system" => DbusLevel::System,
            "session" => DbusLevel::UserSession,
            _ => DbusLevel::SystemAndSession,
        }
    }
}

impl From<u32> for DbusLevel {
    fn from(level: u32) -> Self {
        match level {
            0 => DbusLevel::UserSession,
            1 => DbusLevel::System,
            _ => DbusLevel::SystemAndSession,
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, EnumIter)]
pub enum PreferedColorScheme {
    #[default]
    Default,
    PreferDark,
    PreferLight,
    ForceDark,
    ForceLight,
}

impl PreferedColorScheme {
    pub fn text(&self) -> &str {
        match self {
            PreferedColorScheme::Default => "No Preference",
            PreferedColorScheme::PreferDark => "Prefer Dark",
            PreferedColorScheme::PreferLight => "Prefer Light",
            PreferedColorScheme::ForceDark => "Force Dark",
            PreferedColorScheme::ForceLight => "Force Light",
        }
    }

    pub fn color_scheme(&self) -> adw::ColorScheme {
        match self {
            PreferedColorScheme::Default => adw::ColorScheme::Default,
            PreferedColorScheme::PreferLight => adw::ColorScheme::PreferLight,
            PreferedColorScheme::ForceDark => adw::ColorScheme::ForceDark,
            PreferedColorScheme::ForceLight => adw::ColorScheme::ForceLight,
            PreferedColorScheme::PreferDark => adw::ColorScheme::PreferDark,
        }
    }
}

impl From<PreferedColorScheme> for adw::ColorScheme {
    fn from(value: PreferedColorScheme) -> Self {
        value.color_scheme()
    }
}

impl From<i32> for PreferedColorScheme {
    fn from(value: i32) -> Self {
        match value {
            adw::ffi::ADW_COLOR_SCHEME_DEFAULT => PreferedColorScheme::Default,
            adw::ffi::ADW_COLOR_SCHEME_FORCE_LIGHT => PreferedColorScheme::ForceLight,
            adw::ffi::ADW_COLOR_SCHEME_PREFER_LIGHT => PreferedColorScheme::PreferLight,
            adw::ffi::ADW_COLOR_SCHEME_PREFER_DARK => PreferedColorScheme::PreferDark,
            adw::ffi::ADW_COLOR_SCHEME_FORCE_DARK => PreferedColorScheme::ForceDark,
            _ => PreferedColorScheme::Default,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, EnumIter)]
pub enum OrientationMode {
    #[default]
    Automatic,
    ForceHorizontal,
    ForceVertical,
}

impl OrientationMode {
    pub fn label(&self) -> &str {
        match self {
            OrientationMode::Automatic => "Auto",
            OrientationMode::ForceHorizontal => "Side by Side",
            OrientationMode::ForceVertical => "Top Bottom",
        }
    }

    pub fn icon_name(&self) -> Option<&str> {
        match self {
            OrientationMode::Automatic => None,
            OrientationMode::ForceHorizontal => Some("side-by-side"),
            OrientationMode::ForceVertical => Some("top-down"),
        }
    }

    pub fn key(&self) -> &str {
        match self {
            OrientationMode::Automatic => "auto",
            OrientationMode::ForceHorizontal => "side-by-side",
            OrientationMode::ForceVertical => "top-down",
        }
    }

    pub fn from_key(key: &str) -> Self {
        match key {
            "side-by-side" => OrientationMode::ForceHorizontal,
            "top-down" => OrientationMode::ForceVertical,
            _ => OrientationMode::Automatic,
        }
    }
}

impl From<u32> for OrientationMode {
    fn from(value: u32) -> Self {
        match value {
            0 => OrientationMode::Automatic,
            1 => OrientationMode::ForceHorizontal,
            2 => OrientationMode::ForceVertical,
            _ => OrientationMode::Automatic,
        }
    }
}

pub struct Preferences {
    dbus_level: RwLock<DbusLevel>,
    journal_colors: RwLock<bool>,
    journal_events_batch_size: RwLock<u32>,
    journal_event_max_size: RwLock<u32>,
    unit_file_line_number: RwLock<bool>,
    unit_file_style_scheme: RwLock<String>,
    app_first_connection: RwLock<bool>,
    timestamp_style: RwLock<TimestampStyle>,
    font_family: RwLock<String>,
    font_size: RwLock<u32>,
}

impl Preferences {
    pub fn new_with_setting(settings: Settings) -> Self {
        let level = settings.string(KEY_DBUS_LEVEL).into();
        let journal_colors = settings.boolean(KEY_PREF_JOURNAL_COLORS);
        let journal_events_batch_size = settings.uint(KEY_PREF_JOURNAL_EVENTS_BATCH_SIZE);
        let journal_event_max_size = settings.uint(KEY_PREF_JOURNAL_EVENT_MAX_SIZE);
        let unit_file_colors = settings.boolean(KEY_PREF_UNIT_FILE_LINE_NUMBER);
        let app_first_connection = settings.boolean(KEY_PREF_APP_FIRST_CONNECTION);
        let timestamp_style = settings.string(KEY_PREF_TIMESTAMP_STYLE).into();
        let font_family = settings.string(KEY_PREF_STYLE_TEXT_FONT_FAMILY);
        let font_size = settings.uint(KEY_PREF_STYLE_TEXT_FONT_SIZE);
        let unit_file_style_scheme = settings.string(KEY_PREF_UNIT_FILE_STYLE_SCHEME);

        Preferences {
            dbus_level: RwLock::new(level),
            journal_colors: RwLock::new(journal_colors),
            journal_events_batch_size: RwLock::new(journal_events_batch_size),
            journal_event_max_size: RwLock::new(journal_event_max_size),
            unit_file_line_number: RwLock::new(unit_file_colors),
            app_first_connection: RwLock::new(app_first_connection),
            timestamp_style: RwLock::new(timestamp_style),
            font_family: RwLock::new(font_family.to_string()),
            font_size: RwLock::new(font_size),
            unit_file_style_scheme: RwLock::new(unit_file_style_scheme.to_string()),
        }
    }

    pub fn dbus_level(&self) -> DbusLevel {
        *self.dbus_level.read().unwrap()
    }

    pub fn journal_colors(&self) -> bool {
        *self.journal_colors.read().unwrap()
    }

    pub fn journal_max_events_batch_size(&self) -> u32 {
        *self.journal_events_batch_size.read().unwrap()
    }

    pub fn journal_event_max_size(&self) -> u32 {
        *self.journal_event_max_size.read().unwrap()
    }

    pub fn unit_file_line_number(&self) -> bool {
        *self.unit_file_line_number.read().unwrap()
    }

    pub fn is_app_first_connection(&self) -> bool {
        *self.app_first_connection.read().unwrap()
    }

    pub fn timestamp_style(&self) -> TimestampStyle {
        *self.timestamp_style.read().unwrap()
    }

    pub fn font_family(&self) -> String {
        let read = self.font_family.read().unwrap();
        read.clone()
    }

    pub fn unit_file_style_scheme(&self) -> String {
        let read = self.unit_file_style_scheme.read().unwrap();
        read.clone()
    }

    pub fn font_size(&self) -> u32 {
        *self.font_size.read().unwrap()
    }

    pub fn set_dbus_level(&self, dbus_level: DbusLevel) {
        info!("set_dbus_level: {}", dbus_level.as_str());

        let mut self_dbus_level = self.dbus_level.write().expect("supposed to write");
        *self_dbus_level = dbus_level;
    }

    pub fn set_timestamp_style(&self, timestamp_style: TimestampStyle) {
        info!("set_timestamp_style: {}", timestamp_style);

        let mut self_timestamp_style = self.timestamp_style.write().expect("supposed to write");
        *self_timestamp_style = timestamp_style;
    }

    pub fn save_dbus_level(&self, settings: &Settings) {
        let level = self.dbus_level();
        match settings.set_string(KEY_DBUS_LEVEL, level.as_str()) {
            Ok(()) => info!(
                "Save setting '{KEY_DBUS_LEVEL}' with value {:?}",
                level.as_str()
            ),
            Err(e) => warn!("Save setting Error {}", e),
        }
    }

    pub fn set_journal_events_batch_size(&self, journal_events_batch_size_new: u32) {
        info!("set_journal_events: {journal_events_batch_size_new}");

        let mut journal_events_batch_size = self
            .journal_events_batch_size
            .write()
            .expect("supposed to write");
        *journal_events_batch_size = journal_events_batch_size_new;
    }

    pub fn set_journal_event_max_size(&self, journal_event_max_size_new: u32) {
        info!("journal_event_max_size: {journal_event_max_size_new}");

        let mut journal_event_max_size = self
            .journal_event_max_size
            .write()
            .expect("supposed to write");
        *journal_event_max_size = journal_event_max_size_new;
    }

    pub fn set_journal_colors(&self, display: bool) {
        info!("set_journal_colors: {display}");

        let mut journal_colors = self.journal_colors.write().expect("supposed to write");
        *journal_colors = display;
    }

    pub fn set_unit_file_line_number(&self, display: bool) {
        info!("set_unit_file_highlighting: {display}");

        let mut unit_file_colors = self
            .unit_file_line_number
            .write()
            .expect("supposed to write");
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

    fn set_font_family(&self, font_family: &str) {
        let mut font_family_rw = self.font_family.write().expect("supposed to write");
        *font_family_rw = font_family.to_string();
    }

    pub fn set_unit_file_style_scheme(&self, style_scheme: &str) {
        let mut unit_file_style_scheme_rw = self
            .unit_file_style_scheme
            .write()
            .expect("supposed to write");
        *unit_file_style_scheme_rw = style_scheme.to_string()
    }

    fn set_font_size(&self, font_size: u32) {
        let mut font_size_rw = self.font_size.write().expect("supposed to write");
        *font_size_rw = font_size;
    }

    pub fn set_font(&self, font_description: &FontDescription) {
        let family = font_description.family().map_or(GString::new(), |f| f);
        self.set_font_family(&family);

        let size = font_description.size() / pango::SCALE;
        self.set_font_size(size as u32);
    }

    pub fn set_font_default(&self) {
        self.set_font_family("");
        self.set_font_size(0);
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::gtk::glib::translate::IntoGlib;

    #[test]
    fn test_dbus_level_any_number() {
        assert_eq!(
            <u32 as Into<DbusLevel>>::into(1000),
            DbusLevel::SystemAndSession
        )
    }

    #[test]
    fn test_dbus_level_int_mapping() {
        //assert_num_mapping(EnablementStatus::Unasigned);
        assert_num_mapping(DbusLevel::UserSession);
        assert_num_mapping(DbusLevel::System);
        assert_num_mapping(DbusLevel::SystemAndSession);
    }

    #[test]
    fn test_dbus_level_string_mapping() {
        //assert_num_mapping(EnablementStatus::Unasigned);
        assert_string_mapping(DbusLevel::UserSession, "Session");
        assert_string_mapping(DbusLevel::System, "System");
        assert_string_mapping(DbusLevel::SystemAndSession, "system_session");
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

    #[test]
    fn test_prefered_color_sheme() {
        let list = [
            PreferedColorScheme::Default,
            PreferedColorScheme::ForceLight,
            PreferedColorScheme::PreferDark,
            PreferedColorScheme::PreferLight,
            PreferedColorScheme::ForceDark,
        ];

        for p_color in list {
            let a_color: adw::ColorScheme = p_color.into();

            let i = a_color.into_glib();

            let b_color: PreferedColorScheme = i.into();

            assert_eq!(p_color, b_color);
        }
    }
}
