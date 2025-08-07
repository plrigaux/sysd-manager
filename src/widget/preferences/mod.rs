pub mod data;
mod drop_down_elem;
mod imp;
pub mod style_scheme;

use gtk::{glib, subclass::prelude::*};

use super::app_window::AppWindow;

// ANCHOR: mod
glib::wrapper! {
    pub struct PreferencesDialog(ObjectSubclass<imp::PreferencesDialogImpl>)
        @extends adw::PreferencesDialog, adw::Dialog, gtk::Widget,
        @implements  gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
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
