use adw::subclass::window::AdwWindowImpl;
use gio::Settings;

use adw::subclass::prelude::*;
use gtk::subclass::prelude::{ObjectImplExt, ObjectSubclassExt};
use gtk::{gio, glib};
use std::cell::OnceCell;

use crate::systemd_gui;

#[derive(Debug, Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/preferences.ui")]
pub struct PreferencesDialog {
    pub settings: OnceCell<Settings>,

    #[template_child]
    pub dbus_level_dropdown: TemplateChild<gtk::DropDown>,

    #[template_child]
    pub journal_colors: TemplateChild<gtk::Switch>,
}

impl PreferencesDialog {
    fn setup_settings(&self) {
        let settings = gio::Settings::new(systemd_gui::APP_ID);
        self.settings
            .set(settings)
            .expect("`settings` should not be set before calling `setup_settings`.");
    }

    fn settings(&self) -> &gio::Settings {
        self.settings
            .get()
            .expect("`settings` should be set in `setup_settings`.")
    }
}

#[glib::object_subclass]
impl ObjectSubclass for PreferencesDialog {
    const NAME: &'static str = "PreferencesWindow";
    type Type = super::PreferencesDialog;
    type ParentType = adw::Window;

    fn class_init(klass: &mut Self::Class) {
        // The layout manager determines how child widgets are laid out.
        klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for PreferencesDialog {
    fn constructed(&self) {
        self.parent_constructed();
        // Load latest window state
        let obj = self.obj();
        self.setup_settings();
        //self.load_window_size();
    }
}
impl WidgetImpl for PreferencesDialog {}
impl WindowImpl for PreferencesDialog {
    // Save window state right before the window will be closed
    fn close_request(&self) -> glib::Propagation {
        // Save window size
        log::info!("Close window");
/*         self.obj()
            .save_window_size()
            .expect("Failed to save window state"); */
        // Allow to invoke other event handlers
        glib::Propagation::Proceed
    }
}
impl AdwWindowImpl for PreferencesDialog {}
//impl WindowImpl for PreferencesDialog {}
// ANCHOR_END: imp
