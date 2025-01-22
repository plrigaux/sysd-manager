pub mod data;
mod imp;

use gtk::{gio, glib, subclass::prelude::*};

use super::app_window::AppWindow;

// ANCHOR: mod
glib::wrapper! {
    pub struct PreferencesDialog(ObjectSubclass<imp::PreferencesDialog>)
        @extends adw::PreferencesDialog, adw::Dialog, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl PreferencesDialog {
    pub fn new(app_window: Option<&AppWindow>) -> Self {
        // Create new window
        let obj: PreferencesDialog = glib::Object::new();

        obj.imp().set_app_window(app_window);
        obj
    }
}

impl Default for PreferencesDialog {
    fn default() -> Self {
        PreferencesDialog::new(None)
    }
}
