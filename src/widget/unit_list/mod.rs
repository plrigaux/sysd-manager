use crate::systemd::data::UnitInfo;

use super::InterPanelMessage;
use super::app_window::AppWindow;
use gtk::glib;
use gtk::subclass::prelude::*;

mod filter;
mod imp;

glib::wrapper! {
    pub struct UnitListPanel(ObjectSubclass<imp::UnitListPanelImp>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl UnitListPanel {
    pub fn register_selection_change(
        &self,
        app_window: &AppWindow,
        refresh_unit_list_button: &gtk::Button,
    ) {
        let obj = self.imp();
        obj.register_selection_change(app_window, refresh_unit_list_button);
    }

    pub fn search_bar(&self) -> gtk::SearchBar {
        self.imp().search_bar()
    }

    pub fn fill_store(&self) {
        self.imp().fill_store()
    }

    pub fn button_search_toggled(&self, toggle_button_is_active: bool) {
        self.imp().button_search_toggled(toggle_button_is_active);
    }

    pub fn set_unit(&self, unit: Option<&UnitInfo>) {
        self.imp().set_unit(unit);
    }

    pub fn selected_unit(&self) -> Option<UnitInfo> {
        self.imp().selected_unit()
    }

    pub fn set_inter_message(&self, action: &InterPanelMessage) {
        self.imp().set_inter_message(action);
    }

    fn set_sorter(&self) {
        self.imp().set_sorter();
    }
}
