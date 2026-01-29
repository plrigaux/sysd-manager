use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

use crate::consts::FILTER_MARK;
use crate::systemd::data::UnitInfo;
use crate::widget::unit_list::filter::unit_prop_filter::{
    UnitPropertyAssessor, UnitPropertyFilter,
};
use crate::widget::unit_properties_selector::data_selection::UnitPropertySelection;

use super::InterPanelMessage;
use super::app_window::AppWindow;

use gtk::glib;
use gtk::subclass::prelude::*;

mod filter;
mod imp;
pub mod menus;
mod search_controls;

pub const COL_ID_UNIT: &str = "sysdm-unit";

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

    fn lazy_get_filter_assessor(
        &self,
        id: &str,
        propperty_type: Option<String>,
    ) -> Option<Rc<RefCell<Box<dyn UnitPropertyFilter>>>> {
        self.imp().lazy_get_filter_assessor(id, propperty_type)
    }

    fn filter_assessor_change(
        &self,
        id: &str,
        empty: Option<Box<dyn UnitPropertyAssessor>>,
        change_type: Option<gtk::FilterChange>,
        update_widget: bool,
    ) {
        self.imp()
            .filter_assessor_change(id, empty, change_type, update_widget);
    }

    fn clear_filters(&self, filter_key: &str) {
        self.imp().clear_filters(filter_key);
    }

    pub fn button_action(&self, action: &InterPanelMessage) {
        self.imp().button_action(action)
    }

    pub fn set_new_columns(&self, list: Vec<UnitPropertySelection>) {
        self.imp().set_new_columns(list);
    }

    pub fn current_columns(&self) -> Ref<'_, Vec<UnitPropertySelection>> {
        self.imp().current_columns()
    }

    pub fn current_columns_mut(&self) -> RefMut<'_, Vec<UnitPropertySelection>> {
        self.imp().current_columns_mut()
    }

    pub(super) fn default_displayed_columns(&self) -> &Vec<UnitPropertySelection> {
        self.imp().default_displayed_columns()
    }

    pub fn columns(&self) -> gio::ListModel {
        self.imp().columns()
    }

    pub fn print_scroll_adj_logs(&self) {
        self.imp().print_scroll_adj_logs();
    }

    pub fn save_config(&self) {
        self.imp().save_config();
    }
}

#[derive(Debug)]
pub struct CustomPropertyId<'a> {
    pub utype: &'a str,
    pub prop: &'a str,
}

impl<'a> CustomPropertyId<'a> {
    pub fn from_str(s: &'a str) -> Self {
        let Some((t, p)) = s.split_once('@') else {
            return Self { utype: "", prop: s };
        };

        Self { utype: t, prop: p }
    }

    fn generate_quark(&self) -> glib::Quark {
        glib::Quark::from_str(self.prop)
    }

    pub fn has_defined_type(&self) -> bool {
        !self.utype.is_empty()
    }

    pub fn quark(&self) -> glib::Quark {
        glib::Quark::from_str(self.prop)
    }
}

pub fn get_clean_col_title(title: &str) -> String {
    if title.starts_with(FILTER_MARK) {
        title
            .chars()
            .skip(1) //remove filter mark
            .skip_while(|c| c.is_whitespace())
            .collect()
    } else {
        title.trim().to_string()
    }
}
