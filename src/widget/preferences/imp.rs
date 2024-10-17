use gio::Settings;

use adw::subclass::prelude::*;
use gtk::prelude::SettingsExt;
use gtk::subclass::prelude::ObjectImplExt;
use gtk::{gio, glib};
use log::{info, warn};
use std::cell::OnceCell;

use crate::systemd_gui;
use crate::widget::preferences::data::{DbusLevel, KEY_DBUS_LEVEL, KEY_PREF_JOURNAL_COLORS};

use super::data::PREFERENCES;

#[derive(Debug, Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/preferences.ui")]
pub struct PreferencesDialog {
    pub settings: OnceCell<Settings>,

    #[template_child]
    pub dbus_level_dropdown: TemplateChild<gtk::DropDown>,

    #[template_child]
    pub journal_colors: TemplateChild<gtk::Switch>,
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
        {
            //let setting2 = settings.clone();
            self.dbus_level_dropdown
                .connect_selected_notify(move |dropdown| {
                    let idx = dropdown.selected();

                    info!("Values Selected {:?}", idx,);

                    let level: DbusLevel = idx.into();

                    if let Err(e) = settings.set_string(KEY_DBUS_LEVEL, level.as_str()) {
                        warn!("{}", e)
                    }

                    PREFERENCES.set_dbus_level(level);

                    info!("Save setting '{KEY_DBUS_LEVEL}' with value {:?}", level.as_str());
                });
        }
    }

    fn settings(&self) -> &gio::Settings {
        self.settings
            .get()
            .expect("`settings` should be set in `setup_settings`.")
    }

    fn load_preferences_values(&self) {
        let level = PREFERENCES.dbus_level();
        let journal_colors = PREFERENCES.journal_colors();

        self.dbus_level_dropdown.set_selected(level as u32);

        self.journal_colors.set_state(journal_colors);
        self.journal_colors.set_active(journal_colors);
    }

    #[template_callback]
    fn dbus_level_dropdown_activate(&self, dd: &gtk::DropDown) {
        info!("dd {:?}", dd);
    }

    #[template_callback]
    fn journal_switch_state_set(&self, state: bool) -> bool {
        info!("journal_colors_switch {}", state);

        self.journal_colors.set_state(state);
        PREFERENCES.set_journal_colors(state);
        if let Err(e) = self.settings().set_boolean(KEY_PREF_JOURNAL_COLORS, state) {
            warn!("{}", e)
        }

        true
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
        log::info!("Close preferences");
    }
}

impl PreferencesDialogImpl for PreferencesDialog {}

// ANCHOR_END: imp