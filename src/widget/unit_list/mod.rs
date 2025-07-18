use std::cell::RefCell;
use std::rc::Rc;

use crate::systemd::data::UnitInfo;

use super::InterPanelMessage;
use super::app_window::AppWindow;
use filter::{UnitPropertyAssessor, UnitPropertyFilter};
use gtk::glib;
use gtk::subclass::prelude::*;

mod filter;
mod imp;
mod search_controls;

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

    pub fn fill_store(&self) {
        self.imp().fill_store()
    }

    pub fn button_search_toggled(&self, toggle_button_is_active: bool) {
        self.imp().button_search_toggled(toggle_button_is_active);
    }

    pub fn set_unit(&self, unit: Option<&UnitInfo>) -> Option<UnitInfo> {
        self.imp().set_unit(unit)
    }

    pub fn selected_unit(&self) -> Option<UnitInfo> {
        self.imp().selected_unit()
    }

    pub fn set_inter_message(&self, action: &InterPanelMessage) {
        self.imp().set_inter_message(action);
    }

    fn try_get_filter_assessor(
        &self,
        num_id: u8,
    ) -> Option<&Rc<RefCell<Box<dyn UnitPropertyFilter>>>> {
        self.imp()
            .unit_property_filters
            .get()
            .expect("not None")
            .get(&num_id)
    }

    fn filter_assessor_change(
        &self,
        id: u8,
        empty: Option<Box<dyn UnitPropertyAssessor>>,
        change_type: Option<gtk::FilterChange>,
        update_widget: bool,
    ) {
        self.imp()
            .filter_assessor_change(id, empty, change_type, update_widget);
    }

    fn clear_unit_list_filter_window_dependancy(&self) {
        self.imp().clear_unit_list_filter_window_dependancy();
    }
}
