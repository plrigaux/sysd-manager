use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

use crate::consts::*;
use crate::systemd::data::UnitInfo;
use crate::widget::unit_list::filter::unit_prop_filter::{
    UnitPropertyAssessor, UnitPropertyFilter,
};
use crate::widget::unit_properties_selector::data_selection::UnitPropertySelection;

use super::InterPanelMessage;
use super::app_window::AppWindow;

use gettextrs::pgettext;
use gtk::glib;
use gtk::subclass::prelude::*;
use strum::IntoEnumIterator;
use tracing::warn;

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

#[derive(Debug, Copy, Clone, Default, strum::EnumIter, glib::Enum)]
#[enum_type(name = "UnitListView")]
pub enum UnitListView {
    #[default]
    Defaut,
    ActiveUnit,
    UnitFiles,
    Timers,
    Sockets,
    Custom,
}

impl UnitListView {
    pub fn menu_items() -> gio::Menu {
        let menu_views = gio::Menu::new();

        for item in UnitListView::iter() {
            let (label, action) = item.menu_item();
            menu_views.append(Some(&label), Some(action));
        }

        menu_views
    }

    pub fn menu_item(&self) -> (String, &str) {
        match self {
            UnitListView::Defaut => (
                //List view
                pgettext("menu", "Default"),
                self.win_action(),
            ),
            UnitListView::ActiveUnit => (
                //List view
                pgettext("menu", "Active Units"),
                self.win_action(),
            ),
            UnitListView::UnitFiles => (
                //List view
                pgettext("menu", "Unit Files"),
                self.win_action(),
            ),
            UnitListView::Timers => (
                //List view
                pgettext("menu", "Timers"),
                self.win_action(),
            ),
            UnitListView::Sockets => (
                //List view
                pgettext("menu", "Socket"),
                self.win_action(),
            ),
            UnitListView::Custom => (
                //List view
                pgettext("menu", "Custom"),
                self.win_action(),
            ),
        }
    }

    pub fn action(&self) -> &str {
        &self.win_action()[4..]
    }

    pub fn id(&self) -> &str {
        let wa = &self.win_action();
        let len = wa.len();
        &self.win_action()[4..len - 15]
    }

    pub fn win_action(&self) -> &str {
        match self {
            UnitListView::Defaut => "win.default_unit_list_view",
            UnitListView::ActiveUnit => "win.active_unit_list_view",
            UnitListView::UnitFiles => "win.unit_file_unit_list_view",
            UnitListView::Timers => "win.timers_unit_list_view",
            UnitListView::Sockets => "win.sockets_unit_list_view",
            UnitListView::Custom => "win.custom_unit_list_view",
        }
    }
}

impl From<&glib::Variant> for UnitListView {
    fn from(value: &glib::Variant) -> Self {
        let value_str = value
            .try_get::<String>()
            .inspect_err(|e| warn!("Variant convertion Error {:?}", e))
            .unwrap_or("default".to_owned());

        for unit_list_view in UnitListView::iter() {
            if unit_list_view.id() == value_str {
                return unit_list_view;
            }
        }
        warn!("Value {value_str:?} has no match for UnitListView, fallback to \"default\"");

        UnitListView::Defaut
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_win_action_returns_correct_strings() {
        assert_eq!(
            UnitListView::Defaut.win_action(),
            "win.default_unit_list_view"
        );
        assert_eq!(
            UnitListView::ActiveUnit.win_action(),
            "win.active_unit_list_view"
        );
        assert_eq!(
            UnitListView::UnitFiles.win_action(),
            "win.unit_file_unit_list_view"
        );
        assert_eq!(
            UnitListView::Timers.win_action(),
            "win.timer_unit_list_view"
        );
        assert_eq!(
            UnitListView::Sockets.win_action(),
            "win.socket_unit_list_view"
        );
    }

    #[test]
    fn test_id_extracts_correct_substring() {
        assert_eq!(UnitListView::Defaut.id(), "default");
        assert_eq!(UnitListView::ActiveUnit.id(), "active");
        assert_eq!(UnitListView::UnitFiles.id(), "unit_file");
        assert_eq!(UnitListView::Timers.id(), "timers");
        assert_eq!(UnitListView::Sockets.id(), "sockets");
    }
}
