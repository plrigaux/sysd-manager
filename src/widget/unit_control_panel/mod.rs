//use glib::Object;
use gtk::{glib, subclass::prelude::*};

use crate::systemd::data::UnitInfo;

use super::app_window::AppWindow;

mod controls;
mod enums;
mod imp;

glib::wrapper! {
    pub struct UnitControlPanel(ObjectSubclass<imp::UnitControlPanelImpl>)
    @extends gtk::Box, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl UnitControlPanel {
    pub fn selection_change(&self, unit: &UnitInfo) {
        self.imp().selection_change(unit);
    }

    pub fn set_dark(&self, is_dark: bool) {
        self.imp().set_dark(is_dark);
    }

    pub fn set_overlay(&self, app_window: &AppWindow, toast_overlay: &adw::ToastOverlay) {
        self.imp().set_overlay(app_window, toast_overlay);
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
}
