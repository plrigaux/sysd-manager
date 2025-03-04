use enums::UnitContolType;
//use glib::Object;
use gtk::{glib, subclass::prelude::*};

use crate::systemd::{
    data::UnitInfo,
    enums::{ActiveState, StartStopMode},
    errors::SystemdErrors,
};

use super::{app_window::AppWindow, InterPanelAction};

mod controls;
mod enums;
mod imp;

glib::wrapper! {
    pub struct UnitControlPanel(ObjectSubclass<imp::UnitControlPanelImpl>)
    @extends gtk::Box, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl UnitControlPanel {
    pub fn selection_change(&self, unit: Option<&UnitInfo>) {
        self.imp().selection_change(unit);
    }

    pub fn set_overlay(&self, app_window: &AppWindow, toast_overlay: &adw::ToastOverlay) {
        self.imp().set_overlay(app_window, toast_overlay);
    }

    pub fn toast_overlay(&self) -> Option<&adw::ToastOverlay> {
        self.imp().toast_overlay()
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

    pub fn set_inter_action(&self, action: &InterPanelAction) {
        self.imp().set_inter_action(action);
    }

    fn start_restart(
        &self,
        unit: &UnitInfo,
        start_results: Result<String, SystemdErrors>,
        action: UnitContolType,
        expected_active_state: ActiveState,
        mode: StartStopMode,
    ) {
        self.imp()
            .start_restart(unit, start_results, action, expected_active_state, mode);
    }

    pub fn unlink_child(&self, is_signal: bool) {
        self.imp().unlink_child(is_signal);
    }
}
