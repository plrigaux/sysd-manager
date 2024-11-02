use glib::Object;
use gtk::{glib, subclass::prelude::*};

use crate::systemd::data::UnitInfo;

mod controls;
mod imp;

glib::wrapper! {
    pub struct UnitControlPanel(ObjectSubclass<imp::UnitControlPanelImpl>)
    @extends gtk::Box, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl UnitControlPanel {
    pub fn new(app: &adw::Application) -> Self {
        // Create new window
        Object::builder().property("application", app).build()
    }

    pub fn selection_change(&self, unit: &UnitInfo) {
        self.imp().selection_change(unit);
    }

    pub fn set_dark(&self, is_dark: bool) {
        self.imp().set_dark(is_dark);
    }

    pub fn set_overlay(&self, toast_overlay: &adw::ToastOverlay) {
        self.imp().set_overlay(toast_overlay);
    }
}
