use gio::Settings;

use adw::subclass::prelude::*;
use gtk::glib::BoolError;
use gtk::{gio, glib, prelude::*};
use log::{info, warn};
use std::cell::OnceCell;

use crate::systemd_gui;

use super::data::{
    KEY_PREF_APP_FIRST_CONNECTION, KEY_PREF_JOURNAL_COLORS, KEY_PREF_JOURNAL_MAX_EVENTS,
    KEY_PREF_UNIT_FILE_HIGHLIGHTING, PREFERENCES,
};

#[derive(Debug, Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/preferences.ui")]
pub struct PreferencesDialog {
    pub settings: OnceCell<Settings>,

    #[template_child]
    pub journal_colors: TemplateChild<gtk::Switch>,

    #[template_child]
    pub unit_file_highlight: TemplateChild<gtk::Switch>,

    #[template_child]
    pub preference_banner: TemplateChild<adw::Banner>,

    #[template_child]
    journal_events: TemplateChild<adw::SpinRow>,
}

#[gtk::template_callbacks]
impl PreferencesDialog {
    fn setup_settings(&self) {
        let settings = gio::Settings::new(systemd_gui::APP_ID);
        {
            let settings1 = settings.clone();
            self.settings
                .set(settings1)
                .expect("`settings` should not be set before calling `setup_settings`.");
        }
    }

    fn settings(&self) -> &gio::Settings {
        self.settings
            .get()
            .expect("`settings` should be set in `setup_settings`.")
    }

    fn load_preferences_values(&self) {
        let journal_colors = PREFERENCES.journal_colors();
        let unit_file_colors = PREFERENCES.unit_file_colors();
        let is_app_first_connection = PREFERENCES.is_app_first_connection();

        self.journal_colors.set_state(journal_colors);
        self.journal_colors.set_active(journal_colors);

        let events = PREFERENCES.journal_events();

        self.journal_events.set_value(events as f64);

        self.unit_file_highlight.set_state(unit_file_colors);
        self.unit_file_highlight.set_active(unit_file_colors);

        self.preference_banner.set_revealed(is_app_first_connection);

        self.preference_banner.set_use_markup(true);
        self.preference_banner.set_title(
            "It's your first connection
You can set the application's Dbus level to <u>System</u> if you want to see all Systemd units.",
        );
    }

    #[template_callback]
    fn journal_switch_state_set(&self, state: bool) -> bool {
        info!("journal_colors_switch {}", state);

        self.journal_colors.set_state(state);
        PREFERENCES.set_journal_colors(state);

        true
    }

    #[template_callback]
    fn journal_events_changed(&self, spin: adw::SpinRow) {
        let value = spin.value();

        info!("journal_events_changed to {:?}", value);

        let v32 = if value > f64::from(i32::MAX) {
            u32::MAX
        } else if value < f64::from(i32::MIN) {
            u32::MIN
        } else {
            value.round() as u32
        };

        PREFERENCES.set_journal_events(v32);
    }

    #[template_callback]
    fn unit_file_highlighting_state_set(&self, state: bool) -> bool {
        info!("unit_file_highlighting_switch {}", state);

        self.unit_file_highlight.set_state(state);
        PREFERENCES.set_unit_file_highlighting(state);

        true
    }

    fn save_preference_settings(&self) -> Result<(), BoolError> {
        let settings = self.settings();

        let app_first_connection = PREFERENCES.is_app_first_connection();
        settings.set_boolean(KEY_PREF_APP_FIRST_CONNECTION, app_first_connection)?;

        let journal_colors = PREFERENCES.journal_colors();
        settings.set_boolean(KEY_PREF_JOURNAL_COLORS, journal_colors)?;

        let journal_events = PREFERENCES.journal_events();
        settings.set_uint(KEY_PREF_JOURNAL_MAX_EVENTS, journal_events)?;

        let unit_file_colors = PREFERENCES.unit_file_colors();
        settings.set_boolean(KEY_PREF_UNIT_FILE_HIGHLIGHTING, unit_file_colors)?;

        Ok(())
    }
}

#[glib::object_subclass]
impl ObjectSubclass for PreferencesDialog {
    const NAME: &'static str = "PreferencesWindow";
    type Type = super::PreferencesDialog;
    type ParentType = adw::PreferencesDialog;

    fn class_init(klass: &mut Self::Class) {
        // The layout manager determines how child widgets are laid out.
        klass.bind_template();
        klass.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for PreferencesDialog {
    fn constructed(&self) {
        self.parent_constructed();

        // Load latest window state
        self.setup_settings();
        self.load_preferences_values();
    }
}
impl WidgetImpl for PreferencesDialog {}
impl WindowImpl for PreferencesDialog {}

impl AdwDialogImpl for PreferencesDialog {
    fn closed(&self) {
        log::info!("Close preferences window");

        PREFERENCES.set_app_first_connection(false);

        if let Err(error) = self.save_preference_settings() {
            warn!("Save setting  error {:?}", error)
        }
    }
}

impl PreferencesDialogImpl for PreferencesDialog {}
