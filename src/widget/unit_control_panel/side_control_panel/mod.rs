use gtk::{glib, prelude::*, subclass::prelude::*};

use crate::{
    systemd::{data::UnitInfo, errors::SystemdErrors},
    widget::{InterPanelMessage, app_window::AppWindow},
};
use base::enums::UnitDBusLevel;

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

    pub fn set_inter_message(&self, action: &InterPanelMessage) {
        self.imp().set_inter_message(action);
    }

    pub fn add_toast_message(&self, message: &str, use_markup: bool) {
        self.imp().parent().add_toast_message(message, use_markup);
    }

    pub fn call_method(
        &self,
        method_name: &str,
        need_selected_unit: bool,
        button: &impl IsA<gtk::Widget>,
        systemd_method: impl Fn(Option<(UnitDBusLevel, String)>) -> Result<(), SystemdErrors>
        + std::marker::Send
        + 'static,
        return_handle: impl Fn(&str, Option<&UnitInfo>, Result<(), SystemdErrors>, &UnitControlPanel)
        + 'static,
    ) {
        self.imp().parent().call_method(
            method_name,
            need_selected_unit,
            button,
            systemd_method,
            return_handle,
        );
    }
}
