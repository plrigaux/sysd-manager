use gtk::{glib, prelude::*, subclass::prelude::*};

use crate::systemd::{data::UnitInfo, errors::SystemdErrors};

use super::{InterPanelMessage, app_window::AppWindow};

mod controls;
mod enums;
mod imp;
pub mod side_control_panel;

glib::wrapper! {
    pub struct UnitControlPanel(ObjectSubclass<imp::UnitControlPanelImpl>)
    @extends gtk::Box, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl UnitControlPanel {
    pub fn selection_change(&self, unit: Option<&UnitInfo>) {
        self.imp().selection_change(unit);
    }

    pub fn set_app_window(&self, app_window: &AppWindow) {
        self.imp().set_overlay(app_window);
    }

    pub(super) fn add_toast_message(&self, message: &str, use_markup: bool) {
        self.imp().add_toast_message(message, use_markup);
    }

    pub fn display_info_page(&self) {
        self.imp().display_info_page();
    }

    pub fn display_dependencies_page(&self) {
        self.imp().display_dependencies_page();
    }

    pub fn display_journal_page(&self) {
        self.imp().display_journal_page();
    }

    pub fn display_definition_file_page(&self) {
        self.imp().display_definition_file_page();
    }

    pub fn refresh_panels(&self) {
        self.imp().refresh_panels();
    }

    pub fn set_inter_message(&self, action: &InterPanelMessage) {
        self.imp().set_inter_message(action);
    }

    pub fn unlink_child(&self, is_signal: bool) {
        self.imp().unlink_child(is_signal);
    }

    pub(super) fn call_method(
        &self,
        method_name: &str,
        button: &impl IsA<gtk::Widget>,
        systemd_method: impl Fn(&UnitInfo) -> Result<(), SystemdErrors> + std::marker::Send + 'static,
        return_handle: impl Fn(&UnitInfo, Result<(), SystemdErrors>, &UnitControlPanel) + 'static,
    ) {
        self.imp()
            .call_method(method_name, button, systemd_method, return_handle);
    }
}
