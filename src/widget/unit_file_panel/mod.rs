pub mod dosini;
pub mod flatpak;
mod imp;

use gtk::{glib, subclass::prelude::ObjectSubclassIsExt};

use super::{app_window::AppWindow, InterPanelAction};

// ANCHOR: mod
glib::wrapper! {
    pub struct UnitFilePanel(ObjectSubclass<imp::UnitFilePanelImp>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl UnitFilePanel {
    pub fn new() -> Self {
        // Create new window
        let obj: UnitFilePanel = glib::Object::new();
        obj
    }

    pub fn register(&self, app_window: &AppWindow, toast_overlay: &adw::ToastOverlay) {
        self.imp().register(app_window, toast_overlay);
    }

    pub fn refresh_panels(&self) {
        self.imp().refresh_panels();
    }

    pub fn set_inter_action(&self, action: &InterPanelAction) {
        self.imp().set_inter_action(action);
    }
}

impl Default for UnitFilePanel {
    fn default() -> Self {
        UnitFilePanel::new()
    }
}
