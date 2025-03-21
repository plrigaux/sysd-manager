use gtk::{glib, subclass::prelude::*};

use crate::widget::{InterPanelAction, app_window::AppWindow};

use super::UnitControlPanel;

mod imp;

glib::wrapper! {
    pub struct SideControlPanel(ObjectSubclass<imp::SideControlPanelImpl>)
    @extends gtk::Box, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl SideControlPanel {
    pub fn new(parent: &UnitControlPanel) -> Self {
        let obj: SideControlPanel = glib::Object::new();
        obj.imp().set_parent(parent);
        obj
    }

    pub fn set_app_window(&self, app_window: &AppWindow) {
        self.imp().set_app_window(app_window);
    }

    pub fn unlink_child(&self, is_signal: bool) {
        self.imp().unlink_child(is_signal);
    }

    pub fn set_inter_action(&self, action: &InterPanelAction) {
        self.imp().set_inter_action(action);
    }

    pub fn add_toast_message(&self, message: &str, use_markup: bool) {
        self.imp().parent().add_toast_message(message, use_markup);
    }
}
