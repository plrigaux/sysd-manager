pub mod data;
mod imp;

use gtk::{gio, glib};

// ANCHOR: mod
glib::wrapper! {
    pub struct PreferencesDialog(ObjectSubclass<imp::PreferencesDialog>)
        @extends adw::PreferencesDialog, adw::Dialog, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl PreferencesDialog {
    pub fn new() -> Self {
        // Create new window
        let obj: PreferencesDialog = glib::Object::new();

        obj
    }
}
