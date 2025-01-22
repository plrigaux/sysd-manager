use crate::systemd::data::UnitInfo;

mod construct_info;
mod imp;

use gtk::{glib, subclass::prelude::ObjectSubclassIsExt};

use super::app_window::AppWindow;

glib::wrapper! {
    pub struct UnitInfoPanel(ObjectSubclass<imp::UnitInfoPanelImp>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl UnitInfoPanel {
    pub fn new(is_dark: bool) -> Self {
        // Create new window
        let obj: UnitInfoPanel = glib::Object::new();

        obj.set_dark(is_dark);

        obj
    }

    pub fn display_unit_info(&self, unit: &UnitInfo) {
        self.imp().display_unit_info(unit);
    }

    pub fn set_dark(&self, is_dark: bool) {
        self.imp().set_dark(is_dark)
    }

    pub fn register(&self, app_window: &AppWindow) {
        self.imp().register(app_window);
    }

    pub fn refresh_panels(&self) {
        self.imp().refresh_panels();
    }
}
