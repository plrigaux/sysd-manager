use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

use crate::consts::*;
use crate::systemd::data::UnitInfo;
use crate::widget::unit_list::column::SysdColumn;
use crate::widget::unit_list::filter::unit_prop_filter::{
    UnitPropertyAssessor, UnitPropertyFilter,
};
use crate::widget::unit_properties_selector::data_selection::UnitPropertySelection;

use super::InterPanelMessage;
use super::app_window::AppWindow;

use gettextrs::pgettext;
use glib::variant::ToVariant;
use gtk::glib;
use gtk::subclass::prelude::*;
use strum::IntoEnumIterator;
use tracing::warn;

pub mod column;
mod filter;
mod imp;
pub mod menus;
mod search_controls;

pub const COL_ID_UNIT: &str = "sysdm-unit";
pub const COL_ID_UNIT_FULL: &str = "sysdm-unit-full";

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
        id: &SysdColumn,
    ) -> Option<Rc<RefCell<Box<dyn UnitPropertyFilter>>>> {
        self.imp().lazy_get_filter_assessor(id)
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
        self.imp().set_new_columns(list, true);
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

    pub fn save_column_config(&self) {
        self.imp().save_config();
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

#[derive(Debug, Copy, Clone, Default, strum::EnumIter, glib::Enum, Eq, PartialEq)]
#[enum_type(name = "UnitListView")]
pub enum UnitCuratedList {
    #[default]
    Defaut,
    LoadedUnit,
    UnitFiles,
    Timers,
    Sockets,
    Path,
    Automount,
    Custom,
    Favorite,
}

impl UnitCuratedList {
    pub const WIN_ACTION: &'static str = "win.unit-list-view";

    pub fn base_action() -> &'static str {
        &Self::WIN_ACTION[4..]
    }

    pub fn menu_items() -> gio::Menu {
        let menu_lists = gio::Menu::new();

        Self::add_menu_item(&menu_lists, UnitCuratedList::Defaut);
        Self::add_menu_item(&menu_lists, UnitCuratedList::LoadedUnit);
        Self::add_menu_item(&menu_lists, UnitCuratedList::UnitFiles);
        Self::add_menu_item(&menu_lists, UnitCuratedList::Custom);
        Self::add_menu_item(&menu_lists, UnitCuratedList::Favorite);

        let special_list = gio::Menu::new();

        Self::add_menu_item(&special_list, UnitCuratedList::Timers);
        Self::add_menu_item(&special_list, UnitCuratedList::Sockets);
        Self::add_menu_item(&special_list, UnitCuratedList::Path);
        Self::add_menu_item(&special_list, UnitCuratedList::Automount);

        menu_lists.insert_section(-1, Some("Special Lists"), &special_list);

        let menu_file = gio::Menu::new();

        let label = pgettext("menu", "Include Unit Files");
        let item = gio::MenuItem::new(Some(&label), Some(WIN_ACTION_INCLUDE_UNIT_FILES));
        menu_file.append_item(&item);
        menu_lists.insert_section(-1, None, &menu_file);

        menu_lists
    }

    fn add_menu_item(menu_views: &gio::Menu, item: UnitCuratedList) {
        let label = item.menu_item();

        let menu_item = gio::MenuItem::new(Some(&label), Some(Self::WIN_ACTION));
        menu_item.set_attribute_value(gio::MENU_ATTRIBUTE_TARGET, Some(&item.id().to_variant()));
        menu_views.append_item(&menu_item);
    }

    pub fn menu_item(&self) -> String {
        match self {
            UnitCuratedList::Defaut => {
                //Curated List View
                pgettext("menu", "Default")
            }
            UnitCuratedList::LoadedUnit => {
                //Curated List View
                pgettext("menu", "Loaded Units")
            }
            UnitCuratedList::UnitFiles => {
                //List view
                pgettext("menu", "Unit Files")
            }
            UnitCuratedList::Timers => {
                //Curated List View
                pgettext("menu", "Timers")
            }
            UnitCuratedList::Sockets => {
                //Curated List View
                pgettext("menu", "Sockets")
            }
            UnitCuratedList::Path => {
                //Curated List View
                pgettext("menu", "Path")
            }
            UnitCuratedList::Automount => {
                //Curated List View
                pgettext("menu", "Automounts")
            }
            UnitCuratedList::Custom => {
                //Curated List View
                pgettext("menu", "Customized")
            }
            UnitCuratedList::Favorite => {
                //Curated List View
                pgettext("menu", "Favorites")
            }
        }
    }

    pub fn id(&self) -> &str {
        match self {
            UnitCuratedList::Defaut => "default",
            UnitCuratedList::LoadedUnit => "loaded",
            UnitCuratedList::UnitFiles => "unit_file",
            UnitCuratedList::Timers => "timers",
            UnitCuratedList::Sockets => "sockets",
            UnitCuratedList::Path => "paths",
            UnitCuratedList::Automount => "automounts",
            UnitCuratedList::Custom => "custom",
            UnitCuratedList::Favorite => "favorite",
        }
    }

    pub fn win_accels(&self) -> [&str; 1] {
        match self {
            UnitCuratedList::Defaut => ["<Ctrl><Shift>d"],
            UnitCuratedList::LoadedUnit => ["<Ctrl><Shift>l"],
            UnitCuratedList::UnitFiles => ["<Ctrl><Shift>F"],
            UnitCuratedList::Timers => ["<Ctrl><Shift>t"],
            UnitCuratedList::Sockets => ["<Ctrl><Shift>S"],
            UnitCuratedList::Path => ["<Ctrl><Shift>p"],
            UnitCuratedList::Automount => ["<Ctrl><Shift>a"],
            UnitCuratedList::Custom => ["<Ctrl><Shift>C"],
            UnitCuratedList::Favorite => ["<Ctrl><Shift>b"],
        }
    }

    pub(crate) fn detailed_action(&self) -> String {
        format!("{}::{}", Self::WIN_ACTION, self.id())
    }
}

impl From<&glib::Variant> for UnitCuratedList {
    fn from(value: &glib::Variant) -> Self {
        let value_str = value
            .try_get::<String>()
            .inspect_err(|e| warn!("Variant convertion Error {:?}", e))
            .unwrap_or("default".to_owned());

        for unit_list_view in UnitCuratedList::iter() {
            if unit_list_view.id() == value_str {
                return unit_list_view;
            }
        }
        warn!("Value {value_str:?} has no match for UnitListView, fallback to \"default\"");

        UnitCuratedList::Defaut
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_extracts_correct_substring() {
        assert_eq!(UnitCuratedList::Defaut.id(), "default");
        assert_eq!(UnitCuratedList::LoadedUnit.id(), "active");
        assert_eq!(UnitCuratedList::UnitFiles.id(), "unit_file");
        assert_eq!(UnitCuratedList::Timers.id(), "timers");
        assert_eq!(UnitCuratedList::Sockets.id(), "sockets");
    }
}
