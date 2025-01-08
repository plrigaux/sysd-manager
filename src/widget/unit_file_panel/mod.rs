pub mod dosini;
pub mod flatpak;
mod imp;

use gtk::{glib, subclass::prelude::ObjectSubclassIsExt};

use super::app_window::AppWindow;

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

        /*         let system_manager = adw::StyleManager::default();

        let is_dark = system_manager.is_dark();

        obj.set_dark(is_dark); */

        obj
    }

    pub fn register(&self, app_window: &AppWindow, toast_overlay: &adw::ToastOverlay) {
        self.imp().register(app_window, toast_overlay);
    }
}
