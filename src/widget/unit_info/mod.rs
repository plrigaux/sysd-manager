mod construct_info;
mod imp;

use gtk::{glib, subclass::prelude::ObjectSubclassIsExt};

use super::{InterPanelMessage, app_window::AppWindow};

glib::wrapper! {
    pub struct UnitInfoPanel(ObjectSubclass<imp::UnitInfoPanelImp>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl UnitInfoPanel {
    pub fn new() -> Self {
        // Create new window
        let obj: UnitInfoPanel = glib::Object::new();

        obj
    }

    pub fn register(&self, app_window: &AppWindow) {
        self.imp().register(app_window);
    }

    pub fn refresh_panels(&self) {
        self.imp().refresh_panels();
    }

    pub fn set_inter_message(&self, action: &InterPanelMessage) {
        self.imp().set_inter_message(action);
    }
}

impl Default for UnitInfoPanel {
    fn default() -> Self {
        Self::new()
    }
}
