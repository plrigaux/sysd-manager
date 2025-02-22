mod imp;

use crate::systemd::data::UnitInfo;
use gtk::{gio, glib, subclass::prelude::*};

mod rowitem;

glib::wrapper! {
    pub struct InfoWindow(ObjectSubclass<imp::InfoWindowImp>)
        @extends gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl InfoWindow {
    pub fn new() -> Self {
        // Create new window
        //let zelf = Object::builder().build();
        let obj: InfoWindow = glib::Object::new();

        obj
    }

    pub fn fill_data(&self, unit: &UnitInfo) {
        self.imp().fill_data(unit);
    }

    pub fn fill_systemd_info(&self) {
        self.imp().fill_systemd_info();
    }
}

impl Default for InfoWindow {
    fn default() -> Self {
        Self::new()
    }
}
