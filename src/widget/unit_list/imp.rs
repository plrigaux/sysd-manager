mod column_factories;
#[macro_use]
mod construct;
pub mod pop_menu;

use std::{
    cell::{Cell, OnceCell, Ref, RefCell},
    collections::{HashMap, HashSet},
    rc::Rc,
    time::Duration,
};

use gtk::{
    Adjustment, TemplateChild,
    gio::{self, glib::VariantTy},
    glib::{self, Properties, Quark},
    prelude::*,
    subclass::{
        box_::BoxImpl,
        prelude::*,
        widget::{
            CompositeTemplateCallbacksClass, CompositeTemplateClass,
            CompositeTemplateInitializingExt, WidgetImpl,
        },
    },
};
use zvariant::OwnedValue;

use crate::{
    consts::ACTION_UNIT_LIST_FILTER_CLEAR,
    systemd::{
        self, SystemdUnitFile,
        data::UnitInfo,
        enums::{LoadState, UnitDBusLevel, UnitType},
        errors::SystemdErrors,
        runtime,
    },
    systemd_gui,
    widget::{
        InterPanelMessage,
        app_window::AppWindow,
        preferences::data::{
            COL_SHOW_PREFIX, COL_WIDTH_PREFIX, DbusLevel, FLAG_SHOW, FLAG_WIDTH,
            KEY_PREF_UNIT_LIST_DISPLAY_COLORS, KEY_PREF_UNIT_LIST_DISPLAY_SUMMARY, PREFERENCES,
            UNIT_LIST_COLUMNS, UNIT_LIST_COLUMNS_UNIT,
        },
        unit_list::{
            COL_ID_UNIT,
            filter::{
                UnitListFilterWindow, filter_active_state, filter_bus_level, filter_enable_status,
                filter_load_state, filter_preset, filter_sub_state, filter_unit_description,
                filter_unit_name, filter_unit_type,
                unit_prop_filter::{
                    FilterElement, FilterText, UnitPropertyAssessor, UnitPropertyFilter,
                },
            },
            search_controls::UnitListSearchControls,
        },
        unit_properties_selector::{
            data_selection::UnitPropertySelection,
            save::{self},
        },
    },
};
use log::{debug, error, info, warn};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct UnitKey {
    level: UnitDBusLevel,
    primary: String,
}

impl UnitKey {
    fn new(unit: &UnitInfo) -> Self {
        Self::new2(unit.dbus_level(), unit.primary())
    }

    fn new2(level: UnitDBusLevel, primary: String) -> Self {
        UnitKey { level, primary }
    }
}

type UnitPropertyFiltersContainer = OnceCell<HashMap<u8, Rc<RefCell<Box<dyn UnitPropertyFilter>>>>>;
type AppliedUnitPropertyFilters = OnceCell<Rc<RefCell<Vec<Box<dyn UnitPropertyAssessor>>>>>;

#[derive(Default, gtk::CompositeTemplate, Properties)]
#[template(resource = "/io/github/plrigaux/sysd-manager/unit_list_panel.ui")]
#[properties(wrapper_type = super::UnitListPanel)]
pub struct UnitListPanelImp {
    list_store: OnceCell<gio::ListStore>,

    units_map: Rc<RefCell<HashMap<UnitKey, UnitInfo>>>,

    unit_list_sort_list_model: RefCell<gtk::SortListModel>,

    units_browser: RefCell<gtk::ColumnView>,

    single_selection: RefCell<gtk::SingleSelection>,

    #[template_child]
    search_bar: TemplateChild<gtk::SearchBar>,

    filter_list_model: RefCell<gtk::FilterListModel>,

    #[template_child]
    panel_stack: TemplateChild<adw::ViewStack>,

    #[template_child]
    scrolled_window: TemplateChild<gtk::ScrolledWindow>,

    #[template_child]
    summary: TemplateChild<gtk::Box>,

    #[template_child]
    unit_files_number: TemplateChild<gtk::Label>,

    #[template_child]
    loaded_units_count: TemplateChild<gtk::Label>,

    #[template_child]
    unit_filtered_count: TemplateChild<gtk::Label>,

    search_controls: OnceCell<UnitListSearchControls>,

    refresh_unit_list_button: OnceCell<gtk::Button>,

    unit: RefCell<Option<UnitInfo>>,

    pub force_selected_index: Cell<Option<u32>>,

    #[property(name = "display-color", get, set)]
    pub display_color: Cell<bool>,

    pub unit_property_filters: UnitPropertyFiltersContainer,

    pub applied_unit_property_filters: AppliedUnitPropertyFilters,

    app_window: OnceCell<AppWindow>,

    #[property(name = "is-dark", get)]
    is_dark: Cell<bool>,

    default_column_view_column_list: OnceCell<Vec<gtk::ColumnViewColumn>>,

    current_column_view_column_definition_list: RefCell<Vec<UnitPropertySelection>>,

    default_column_view_column_definition_list: OnceCell<Vec<UnitPropertySelection>>,
}

macro_rules! update_search_entry {
    ($self:expr, $id:expr, $update_widget:expr, $text:expr) => {{
        if $update_widget && $id == UNIT_LIST_COLUMNS_UNIT {
            $self.search_entry_set_text($text);
        }
    }};
}

struct UnitProperty {
    interface: String,
    unit_property: String,
    unit_type: UnitType,
}

#[gtk::template_callbacks]
impl UnitListPanelImp {
    #[template_callback]
    fn sections_changed(&self, position: u32) {
        info!("sections_changed {position}");
    }

    #[template_callback]
    fn legend_button_clicked(&self, _button: gtk::Button) {
        self.summary.set_visible(false);
    }
}

impl UnitListPanelImp {
    pub(super) fn register_selection_change(
        &self,
        app_window: &AppWindow,
        refresh_unit_list_button: &gtk::Button,
    ) {
        // let settings = systemd_gui::new_settings();

        self.app_window
            .set(app_window.clone())
            .expect("app_window set once");

        let app_window_clone = app_window.clone();
        let unit_list = self.obj().clone();

        self.single_selection
            .borrow()
            .connect_selected_item_notify(move |single_selection| {
                info!(
                    "connect_selected_notify idx {}",
                    single_selection.selected()
                );
                let Some(object) = single_selection.selected_item() else {
                    warn!("No object selected");
                    return;
                };

                let Some(unit) = object.downcast_ref::<UnitInfo>() else {
                    error!("Object.downcast::<UnitInfo>");
                    return;
                };

                info!("Selection changed, new unit {}", unit.primary());

                unit_list.imp().set_unit_internal(unit);
                app_window_clone.selection_change(Some(unit));
            }); // FOR THE SEARCH

        self.refresh_unit_list_button
            .set(refresh_unit_list_button.clone())
            .expect("refresh_unit_list_button was already set!");

        self.fill_store();

        let units_browser = self.units_browser.borrow().clone();
        let action_entry = {
            gio::ActionEntry::builder("hide_unit_col")
                .activate(move |_application: &AppWindow, _b, target_value| {
                    if let Some(value) = target_value {
                        let key = Some(value.get::<String>().expect("variant always be String"));

                        let columns_list_model = units_browser.columns();

                        for index in 0..columns_list_model.n_items() {
                            let Some(cur_column) = columns_list_model
                                .item(index)
                                .and_downcast::<gtk::ColumnViewColumn>()
                            else {
                                warn!("Column w/ id {key:?} do not Exists");
                                continue;
                            };

                            if cur_column.id().map(|s| s.to_string()) == key {
                                cur_column.set_visible(false);
                            }
                        }
                    }
                })
                .parameter_type(Some(VariantTy::STRING))
                .build()
        };

        let list_filter_action_entry = {
            //  let settings = settings.clone();
            let unit_list_panel = self.obj().clone();
            gio::ActionEntry::builder("unit_list_filter")
                .activate(move |_application: &AppWindow, _b, target_value| {
                    let column_id = target_value
                        .map(|var| var.get::<String>().expect("variant always be String"));
                    debug!("Filter list, col {column_id:?}");

                    let filter_win = UnitListFilterWindow::new(column_id, &unit_list_panel);
                    filter_win.construct_filter_dialog();
                    filter_win.present();
                })
                .parameter_type(Some(VariantTy::STRING))
                .build()
        };

        let list_filter_action_entry_blank = {
            //  let settings = settings.clone();
            let unit_list_panel = self.obj().clone();
            gio::ActionEntry::builder("unit_list_filter_blank")
                .activate(move |_application: &AppWindow, _b, _target_value| {
                    let filter_win = UnitListFilterWindow::new(None, &unit_list_panel);
                    filter_win.construct_filter_dialog();
                    filter_win.present();
                })
                .build()
        };

        let list_filter_clear_action_entry = {
            //  let settings = settings.clone();
            let unit_list_panel = self.obj().clone();
            gio::ActionEntry::builder(ACTION_UNIT_LIST_FILTER_CLEAR)
                .activate(move |_application: &AppWindow, _b, _target_value| {
                    unit_list_panel.imp().clear_filters();
                })
                .parameter_type(Some(VariantTy::STRING))
                .build()
        };

        app_window.add_action_entries([
            action_entry,
            list_filter_action_entry,
            list_filter_action_entry_blank,
            list_filter_clear_action_entry,
        ]);
    }

    fn generate_column_map(&self) -> HashMap<glib::GString, gtk::ColumnViewColumn> {
        let list_model: gio::ListModel = self.units_browser.borrow().columns();

        let mut col_map = HashMap::new();
        for col_idx in 0..list_model.n_items() {
            let item_out = list_model
                .item(col_idx)
                .expect("Expect item x to be not None");

            let column_view_column = item_out
                .downcast_ref::<gtk::ColumnViewColumn>()
                .expect("item.downcast_ref::<gtk::ColumnViewColumn>()");

            if let Some(id) = column_view_column.id() {
                col_map.insert(id, column_view_column.clone());
            } else {
                warn!("Column {col_idx} has no id.")
            }
        }
        col_map
    }

    fn generate_column_list(&self) -> Vec<gtk::ColumnViewColumn> {
        let list_model: gio::ListModel = self.units_browser.borrow().columns();

        let mut col_list = Vec::with_capacity(list_model.n_items() as usize);
        for col_idx in 0..list_model.n_items() {
            let item_out = list_model
                .item(col_idx)
                .expect("Expect item x to be not None");

            let column_view_column = item_out
                .downcast_ref::<gtk::ColumnViewColumn>()
                .expect("item.downcast_ref::<gtk::ColumnViewColumn>()");

            col_list.push(column_view_column.clone());
        }
        col_list
    }

    pub(super) fn fill_store(&self) {
        let list_store = self.list_store.get().expect("LIST STORE NOT NONE").clone();
        let unit_map = self.units_map.clone();
        let panel_stack = self.panel_stack.clone();
        let single_selection = self.single_selection.borrow().clone();
        let unit_list = self.obj().clone();
        let units_browser = self.units_browser.borrow().clone();

        let refresh_unit_list_button = self
            .refresh_unit_list_button
            .get()
            .expect("Supposed to be set")
            .clone();

        //Rem sorting before adding lot of items for performance reasons
        self.unit_list_sort_list_model
            .borrow()
            .set_sorter(None::<&gtk::Sorter>);

        let int_level = PREFERENCES.dbus_level();

        glib::spawn_future_local(async move {
            refresh_unit_list_button.set_sensitive(false);
            panel_stack.set_visible_child_name("spinner");

            let (unit_desc, unit_from_files) = match go_fetch_data(int_level).await {
                Ok(value) => value,
                Err(err) => {
                    warn!("Fail fetch unit list {err:?}");
                    panel_stack.set_visible_child_name("error");
                    return;
                }
            };

            unit_list
                .imp()
                .loaded_units_count
                .set_label(&unit_desc.len().to_string());
            unit_list
                .imp()
                .unit_files_number
                .set_label(&unit_from_files.len().to_string());

            let n_items = list_store.n_items();
            list_store.remove_all();
            let mut unit_map1 = unit_map.borrow_mut();
            unit_map1.clear();

            let mut all_units = HashMap::with_capacity(unit_desc.len() + unit_from_files.len());

            for system_unit_file in unit_from_files.into_iter() {
                if let Some(loaded_unit) = unit_desc.get(&system_unit_file.full_name) {
                    loaded_unit.update_from_unit_file(system_unit_file);
                } else {
                    let unit = UnitInfo::from_unit_file(system_unit_file);
                    list_store.append(&unit);
                    unit_map1.insert(UnitKey::new(&unit), unit.clone());
                    all_units.insert(unit.primary(), unit);
                }
            }

            for (_key, unit) in unit_desc.into_iter() {
                list_store.append(&unit);
                unit_map1.insert(UnitKey::new(&unit), unit.clone());
                all_units.insert(unit.primary(), unit);
            }

            // The sort function needs to be the same of the  first column sorter
            let sort_func = construct::column_filter_lambda!(primary, dbus_level);

            list_store.sort(sort_func);

            info!("Unit list refreshed! list size {}", list_store.n_items());

            let mut force_selected_index = gtk::INVALID_LIST_POSITION;

            let selected_unit = unit_list.selected_unit();
            if let Some(selected_unit) = selected_unit {
                let selected_unit_name = selected_unit.primary();

                debug!(
                    "LS items-n {} name {}",
                    list_store.n_items(),
                    selected_unit_name
                );

                if let Some(index) = list_store.find_with_equal_func(|object| {
                    let unit = object
                        .downcast_ref::<UnitInfo>()
                        .expect("Needs to be UnitInfo");

                    unit.primary().eq(&selected_unit_name)
                }) {
                    info!(
                        "Force selection to index {index:?} to select unit {selected_unit_name:?}"
                    );
                    single_selection.select_item(index, true);
                    //unit_list.set_force_to_select(index);
                    force_selected_index = index;
                }
            }
            debug!("IM HERRE");
            unit_list
                .imp()
                .force_selected_index
                .set(Some(force_selected_index));
            refresh_unit_list_button.set_sensitive(true);
            unit_list.imp().set_sorter();

            //cause no scrollwindow v adjustment
            if n_items > 0 {
                focus_on_row(&unit_list, &units_browser);
            }
            panel_stack.set_visible_child_name("unit_list");

            glib::spawn_future_local(async move {
                //let (sender, receiver) = tokio::sync::oneshot::channel();

                debug!("IM HERRE 3 map len {}", all_units.len());
                let (sender, mut receiver) = tokio::sync::mpsc::channel(100);

                let mut list = Vec::with_capacity(all_units.len());
                for unit in all_units.values() {
                    let level = unit.dbus_level();
                    let primary = unit.primary();
                    let path = unit.object_path();
                    debug!("path {path}");
                    list.push((level, primary, path));
                }

                runtime().spawn(async move {
                    const BATCH_SIZE: usize = 5;
                    let mut batch = Vec::with_capacity(BATCH_SIZE);
                    for (idx, triple) in (1..).zip(list.into_iter()) {
                        batch.push(triple);

                        if idx % BATCH_SIZE == 0 {
                            call_complete_unit(&sender, batch).await;

                            batch = Vec::with_capacity(BATCH_SIZE);
                        }
                    }

                    call_complete_unit(&sender, batch).await;
                });

                while let Some(updates) = receiver.recv().await {
                    for update in updates {
                        let Some(unit) = all_units.get(&update.primary) else {
                            continue;
                        };

                        unit.update_from_unit_info(update);
                    }
                }
                unit_list.imp().fetch_custom_unit_properties();
            });
        });
    }

    pub(super) fn button_search_toggled(&self, toggle_button_is_active: bool) {
        self.search_bar.set_search_mode(toggle_button_is_active);

        if toggle_button_is_active {
            let s_controls = self.search_controls.get().unwrap();
            s_controls.grab_focus_on_search_entry();
        }
    }

    pub fn set_unit_internal(&self, unit: &UnitInfo) {
        let _ = self.unit.replace(Some(unit.clone()));
    }

    pub fn set_unit(&self, unit: Option<&UnitInfo>) -> Option<UnitInfo> {
        let unit = match unit {
            Some(u) => u,
            None => {
                self.unit.replace(None);
                return None;
            }
        };

        let old = self.unit.replace(Some(unit.clone()));
        if let Some(old) = old
            && old.primary() == unit.primary()
        {
            info!("List {} == {}", old.primary(), unit.primary());
            return Some(old);
        }

        let unit_name = unit.primary();

        info!(
            "Unit List {} list_store {} filter {} sort_model {}",
            unit_name,
            self.list_store.get().unwrap().n_items(),
            self.filter_list_model.borrow().n_items(),
            self.unit_list_sort_list_model.borrow().n_items()
        );

        /*  let finding = self.list_store.find_with_equal_func(|object| {
            let unit_item = object
                .downcast_ref::<UnitBinding>()
                .expect("item.downcast_ref::<UnitBinding>()");

            unit_name == unit_item.primary()
        });

        if let Some(position) = finding {
            //TODO move where needed i.e. enable unit dialog
            if let Some(item) = self.list_store.item(position) {
                let unit_item = item
                    .downcast_ref::<UnitBinding>()
                    .expect("item.downcast_ref::<UnitBinding>()");
                //for constitency ensure that is the unit from the list
                self.unit.replace(Some( de.unit_ref().clone()));
            }
        } else {
            info!("Unit not found {unit_name:?} try to Add");

            self.add_one_unit(unit);
        } */

        if let Some(unit2) = self.units_map.borrow().get(&UnitKey::new(unit)) {
            self.unit.replace(Some(unit2.clone()));
        } else {
            self.add_one_unit(unit);
        }

        //Don't select and focus if filter out
        if let Some(filter) = self.filter_list_model.borrow().filter()
            && !filter.match_(unit)
        {
            //Unselect
            self.single_selection
                .borrow()
                .set_selected(gtk::INVALID_LIST_POSITION);
            info!("Unit {unit_name} no Match");
            return Some(unit.clone());
        }

        let finding = self
            .list_store
            .get()
            .expect("LIST STORE NOT NONE")
            .find_with_equal_func(|object| {
                let Some(unit_item) = object.downcast_ref::<UnitInfo>() else {
                    error!("item.downcast_ref::<UnitBinding>()");
                    return false;
                };

                unit_name == unit_item.primary()
            });

        if let Some(row) = finding {
            info!("Scroll to row {row}");

            self.units_browser.borrow().scroll_to(
                row, // to centerish on the selected unit
                None,
                gtk::ListScrollFlags::FOCUS | gtk::ListScrollFlags::SELECT,
                None,
            );
        }

        Some(unit.clone())
    }

    fn add_one_unit(&self, unit: &UnitInfo) {
        self.list_store.get().unwrap().append(unit);
        let mut unit_map = self.units_map.borrow_mut();
        unit_map.insert(UnitKey::new(unit), unit.clone());

        if LoadState::Loaded == unit.load_state()
            && let Ok(my_int) = self.loaded_units_count.label().parse::<i32>()
        {
            self.loaded_units_count.set_label(&(my_int + 1).to_string());
        }

        if unit.file_path().is_some()
            && let Ok(my_int) = self.unit_files_number.label().parse::<i32>()
        {
            self.unit_files_number.set_label(&(my_int + 1).to_string());
        }
    }

    pub fn selected_unit(&self) -> Option<UnitInfo> {
        self.unit.borrow().clone()
    }

    pub fn set_inter_message(&self, action: &InterPanelMessage) {
        if let InterPanelMessage::IsDark(is_dark) = action {
            self.is_dark.set(*is_dark)
        }
    }

    fn set_sorter(&self) {
        let sorter = self.units_browser.borrow().sorter();

        self.unit_list_sort_list_model
            .borrow()
            .set_sorter(sorter.as_ref());

        let item_out = self
            .units_browser
            .borrow()
            .columns()
            .item(0)
            .expect("Expect item x to be not None");

        //Sort on first column
        let c1 = item_out
            .downcast_ref::<gtk::ColumnViewColumn>()
            .expect("item.downcast_ref::<gtk::ColumnViewColumn>()");

        self.units_browser
            .borrow()
            .sort_by_column(Some(c1), gtk::SortType::Ascending);
    }

    pub(super) fn filter_assessor_change(
        &self,
        id: u8,
        new_assessor: Option<Box<dyn UnitPropertyAssessor>>,
        change_type: Option<gtk::FilterChange>,
        update_widget: bool,
    ) {
        debug!("Assessor Change {new_assessor:?} {change_type:?}");
        let applied_assessors = self
            .applied_unit_property_filters
            .get()
            .expect("applied_assessors not null");

        if let Some(new_assessor) = new_assessor {
            //add

            update_search_entry!(self, id, update_widget, new_assessor.text());

            let mut vect = applied_assessors.borrow_mut();

            if let Some(index) = vect.iter().position(|x| x.id() == id) {
                vect[index] = new_assessor;
            } else {
                vect.push(new_assessor);
            }
        } else {
            //remove
            applied_assessors.borrow_mut().retain(|x| x.id() != id);

            update_search_entry!(self, id, update_widget, "");
        }

        if let Some(change_type) = change_type {
            if let Some(filter) = self.filter_list_model.borrow().filter() {
                filter.changed(change_type);
            } else {
                let custom_filter = self.create_custom_filter();
                self.filter_list_model
                    .borrow()
                    .set_filter(Some(&custom_filter));
            }
        }

        let search_controls = self.search_controls.get().expect("Not Null");
        search_controls
            .imp()
            .set_filter_is_set(!applied_assessors.borrow().is_empty());
    }

    pub(super) fn clear_filters(&self) {
        let applied_assessors = self
            .applied_unit_property_filters
            .get()
            .expect("applied_assessors not null");

        let mut applied_assessors = applied_assessors.borrow_mut();
        applied_assessors.clear();

        for property_filter in self.unit_property_filters.get().expect("Not None").values() {
            let mut prop_filter_mut = property_filter.borrow_mut();
            prop_filter_mut.clear_filter();
        }

        self.filter_list_model
            .borrow()
            .set_filter(None::<&gtk::Filter>); //FIXME this workaround prevents core dump

        let search_controls = self.search_controls.get().expect("Not Null");
        search_controls.imp().clear();
    }

    pub(super) fn button_action(&self, action: &InterPanelMessage) {
        let Some(app_window) = self.app_window.get() else {
            warn!("No app window");
            return;
        };

        app_window.set_inter_message(action);
    }

    fn search_entry_set_text(&self, text: &str) {
        let search_controls = self.search_controls.get().expect("Not Null");

        search_controls.imp().set_search_entry_text(text);
    }

    pub(super) fn update_unit_name_search(&self, text: &str, update_widget: bool) {
        debug!("update_unit_name_search {text}");
        let mut filter = self
            .unit_property_filters
            .get()
            .expect("Not None")
            .get(&UNIT_LIST_COLUMNS_UNIT)
            .expect("Always unit")
            .borrow_mut();

        let filter = filter
            .as_any_mut()
            .downcast_mut::<FilterText>()
            .expect("downcast to FilterText");

        filter.set_filter_elem(text, update_widget);
    }

    pub(super) fn clear_unit_list_filter_window_dependancy(&self) {
        for property_filter in self.unit_property_filters.get().expect("Not None").values() {
            property_filter.borrow_mut().clear_widget_dependancy();
        }
    }

    fn create_custom_filter(&self) -> gtk::CustomFilter {
        let applied_assessors = self
            .applied_unit_property_filters
            .get()
            .expect("not none")
            .clone();
        gtk::CustomFilter::new(move |object| {
            let Some(unit) = object.downcast_ref::<UnitInfo>() else {
                error!("some wrong downcast_ref to UnitBinding  {object:?}");
                return false;
            };

            for asserror in applied_assessors.borrow().iter() {
                if !asserror.filter_unit(unit) {
                    return false;
                }
            }
            true
        })
    }

    pub(super) fn set_new_columns(&self, property_list: Vec<UnitPropertySelection>) {
        let columns_list_model = self.units_browser.borrow().columns();

        //Get the current column
        let cur_n_items = columns_list_model.n_items();
        let mut current_columns = Vec::with_capacity(columns_list_model.n_items() as usize);
        for position in (property_list.len() as u32)..columns_list_model.n_items() {
            let Some(c) = columns_list_model
                .item(position)
                .and_downcast::<gtk::ColumnViewColumn>()
            else {
                warn!("Col None");
                continue;
            };
            current_columns.push(c);
        }

        for (idx, unit_property) in property_list.iter().enumerate() {
            let new_column = unit_property.column();

            let idx_32 = idx as u32;
            if idx_32 < cur_n_items {
                let Some(cur_column) = columns_list_model
                    .item(idx_32)
                    .and_downcast::<gtk::ColumnViewColumn>()
                else {
                    warn!("Col None");
                    continue;
                };

                UnitPropertySelection::copy_col_to_col(&new_column, &cur_column);
                unit_property.set_column(cur_column);
            } else {
                info!("Append {:?} {:?}", new_column.id(), new_column.title());
                self.units_browser.borrow().append_column(&new_column);
            }
        }

        force_expand_on_the_last_visible_column(&columns_list_model);

        self.current_column_view_column_definition_list
            .replace(property_list);

        //remove all columns that exeed the new ones
        for col in current_columns.iter() {
            col.set_visible(false);
        }

        self.fetch_custom_unit_properties();
    }

    fn fetch_custom_unit_properties(&self) {
        let property_list = self.current_column_view_column_definition_list.borrow();

        if property_list.is_empty() {
            return;
        }

        let list_len = property_list.len();
        let mut property_list_send = Vec::with_capacity(property_list.len());
        let mut property_list_keys = Vec::with_capacity(property_list.len());
        let mut types = HashSet::with_capacity(16);
        let mut is_unit_type = false;
        for unit_property in property_list.iter() {
            if unit_property.is_custom() {
                //add custom factory

                let u_prop = unit_property.unit_property();
                let key = Quark::from_str(&u_prop);
                property_list_keys.push(key);

                property_list_send.push(UnitProperty {
                    interface: unit_property.interface(),
                    unit_property: u_prop,
                    unit_type: unit_property.unit_type(),
                });
            }

            match unit_property.unit_type() {
                UnitType::Unit => is_unit_type |= true,
                UnitType::Unknown => { //Do nothing
                }
                unit_type => {
                    types.insert(unit_type);
                }
            }
        }

        let units_browser = self.units_browser.borrow().clone();
        let units_map = self.units_map.clone();
        let display_color = self.display_color.get();

        //TODO fetch oly new properties look at properties already fetched
        glib::spawn_future_local(async move {
            let units_list: Vec<_> = units_map
                .borrow()
                .values()
                .filter(|unit| is_unit_type || types.contains(&unit.unit_type()))
                .map(|unit| {
                    (
                        unit.dbus_level(),
                        unit.primary(),
                        unit.object_path(),
                        unit.unit_type(),
                    )
                })
                .collect();

            let (sender, mut receiver) = tokio::sync::mpsc::channel(100);
            runtime().spawn(async move {
                info!("Fetching properties START for {} units", units_list.len());
                for (level, primary_name, object_path, unit_type) in units_list {
                    let mut property_value_list = Vec::with_capacity(list_len);
                    for unit_property in &property_list_send {
                        if unit_property.unit_type != UnitType::Unit
                            && unit_type != unit_property.unit_type
                        {
                            continue;
                        }
                        debug!("Fetch {} {}", primary_name, unit_property.unit_property);
                        match systemd::fetch_unit_properties(
                            level,
                            &object_path,
                            &unit_property.interface,
                            &unit_property.unit_property,
                        )
                        .await
                        {
                            Ok(value) => property_value_list.push(Some(value)),
                            Err(err) => {
                                property_value_list.push(None);
                                info!(
                                    "PROP {} {} {object_path} {err:?}",
                                    unit_property.interface, unit_property.unit_property
                                );
                            }
                        }
                    }

                    if let Err(err) = sender
                        .send((UnitKey::new2(level, primary_name), property_value_list))
                        .await
                    {
                        error!("The channel needs to be open. {err:?}");
                        break;
                    }
                }
            });

            info!("Fetching properties WAIT");
            while let Some((key, property_value_list)) = receiver.recv().await {
                let map_ref = units_map.borrow();
                let Some(unit) = map_ref.get(&key) else {
                    continue;
                };

                for (index, value) in property_value_list.into_iter().enumerate() {
                    let key = property_list_keys.get(index).expect("Should never fail");

                    match value {
                        Some(value) => unsafe { unit.set_qdata(*key, value) },
                        None => unsafe {
                            unit.steal_qdata::<OwnedValue>(*key);
                        },
                    }
                }
            }
            info!("Fetching properties FINISHED");

            //Force the factory to display data
            let columns_list_model = units_browser.columns();
            for position in 0..columns_list_model.n_items() {
                let Some(column) = columns_list_model
                    .item(position)
                    .and_downcast::<gtk::ColumnViewColumn>()
                else {
                    warn!("Col None");
                    continue;
                };

                let Some(id) = column.id() else {
                    warn!("No column id");
                    continue;
                };

                //identify custom properties
                let Some((_type, prop)) = id.split_once('@') else {
                    continue;
                };

                let factory = column_factories::get_custom_factory(prop, display_color);
                column.set_factory(Some(&factory));
            }
        });
    }

    pub fn print_scroll_adj_logs(&self) {
        let va = self.scrolled_window.vadjustment();
        self.print_adjustment("VER", &va);
        let va = self.scrolled_window.hadjustment();
        self.print_adjustment("HON", &va);
    }

    fn print_adjustment(&self, id: &str, adj: &Adjustment) {
        info!(
            "{} lower={} + page={} <= upper={} gap {} step_inc {} page_inc {}",
            id,
            adj.lower(),
            adj.upper(),
            adj.page_size(),
            adj.upper() - (adj.lower() + adj.page_size()),
            adj.step_increment(),
            adj.page_increment()
        );
    }

    pub(super) fn current_columns(&self) -> Ref<'_, Vec<UnitPropertySelection>> {
        self.current_column_view_column_definition_list.borrow()
    }

    pub(super) fn default_displayed_columns(&self) -> &Vec<UnitPropertySelection> {
        let mut list = self.default_column_view_column_definition_list.get();

        if list.is_none() {
            let column_view_column_definition_list =
                construct::default_column_definition_list(self.display_color.get());

            self.default_column_view_column_definition_list
                .set(column_view_column_definition_list.clone())
                .expect("Set only once");

            list = self.default_column_view_column_definition_list.get();
        }

        list.unwrap()
    }

    pub(super) fn default_columns(&self) -> &Vec<gtk::ColumnViewColumn> {
        self.default_column_view_column_list
            .get()
            .expect("Need to be set")
    }

    pub(super) fn save_config(&self) {
        save::save_column_config(&self.current_columns());
    }
}

fn force_expand_on_the_last_visible_column(columns_list_model: &gio::ListModel) {
    for index in (0..columns_list_model.n_items()).rev() {
        if let Some(column) = columns_list_model
            .item(index)
            .and_downcast::<gtk::ColumnViewColumn>()
        {
            //Force to fill the widget gap in the scroll window
            if column.is_visible() {
                column.set_expand(true);
                break;
            }
        }
    }
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for UnitListPanelImp {
    const NAME: &'static str = "UnitListPanel";
    type Type = super::UnitListPanel;
    type ParentType = gtk::Box;

    fn class_init(klass: &mut Self::Class) {
        // The layout manager determines how child widgets are laid out.
        klass.bind_template();
        klass.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

#[glib::derived_properties]
impl ObjectImpl for UnitListPanelImp {
    fn constructed(&self) {
        self.parent_constructed();

        let settings = systemd_gui::new_settings();

        let unit_list = self.obj().clone();

        settings
            .bind(
                KEY_PREF_UNIT_LIST_DISPLAY_COLORS,
                &unit_list,
                "display-color",
            )
            .build();

        let unit_list = self.obj().clone();

        let list_store = gio::ListStore::new::<UnitInfo>();
        self.list_store
            .set(list_store.clone())
            .expect("Set only Once");

        let (units_browser, single_selection, filter_list_model, sort_list_model, generated) =
            construct::construct_column(list_store, self.display_color.get());

        self.scrolled_window.set_child(Some(&units_browser));
        self.units_browser.replace(units_browser);
        self.single_selection.replace(single_selection);
        self.filter_list_model.replace(filter_list_model);
        self.unit_list_sort_list_model.replace(sort_list_model);

        let column_view_column_list = self.generate_column_list();

        let mut column_view_column_definition_list =
            Vec::with_capacity(column_view_column_list.len());

        for col in column_view_column_list.iter() {
            let unit_property_selection: UnitPropertySelection =
                UnitPropertySelection::from_column_view_column(col.clone());
            column_view_column_definition_list.push(unit_property_selection);
        }

        self.current_column_view_column_definition_list
            .replace(column_view_column_definition_list);

        column_factories::setup_factories(&unit_list, &column_view_column_list);
        self.default_column_view_column_list
            .set(column_view_column_list)
            .expect("Set only once");

        settings.connect_changed(
            Some(KEY_PREF_UNIT_LIST_DISPLAY_COLORS),
            move |_settings, _key| {
                let display_color = unit_list.display_color();
                info!("Change preference setting \"display color\" to {display_color}");
                let column_view_column_list = unit_list.imp().generate_column_list();
                column_factories::setup_factories(&unit_list, &column_view_column_list);
            },
        );

        let mut filter_assessors: HashMap<u8, Rc<RefCell<Box<dyn UnitPropertyFilter>>>> =
            HashMap::with_capacity(UNIT_LIST_COLUMNS.len());

        let unit_list_panel: glib::BorrowedObject<'_, crate::widget::unit_list::UnitListPanel> =
            self.obj();
        for (_, key, num_id, _) in &*UNIT_LIST_COLUMNS {
            let filter: Option<Box<dyn UnitPropertyFilter>> = match *key {
                COL_ID_UNIT => Some(Box::new(FilterText::new(
                    *num_id,
                    filter_unit_name,
                    &unit_list_panel,
                ))),
                "sysdm-bus" => Some(Box::new(FilterElement::new(
                    *num_id,
                    filter_bus_level,
                    &unit_list_panel,
                ))),
                "sysdm-type" => Some(Box::new(FilterElement::new(
                    *num_id,
                    filter_unit_type,
                    &unit_list_panel,
                ))),
                "sysdm-state" => Some(Box::new(FilterElement::new(
                    *num_id,
                    filter_enable_status,
                    &unit_list_panel,
                ))),
                "sysdm-preset" => Some(Box::new(FilterElement::new(
                    *num_id,
                    filter_preset,
                    &unit_list_panel,
                ))),
                "sysdm-load" => Some(Box::new(FilterElement::new(
                    *num_id,
                    filter_load_state,
                    &unit_list_panel,
                ))),
                "sysdm-active" => Some(Box::new(FilterElement::new(
                    *num_id,
                    filter_active_state,
                    &unit_list_panel,
                ))),
                "sysdm-sub" => Some(Box::new(FilterElement::new(
                    *num_id,
                    filter_sub_state,
                    &unit_list_panel,
                ))),
                "sysdm-description" => Some(Box::new(FilterText::new(
                    *num_id,
                    filter_unit_description,
                    &unit_list_panel,
                ))),
                _ => {
                    error!("Key {key}");
                    None
                }
            };

            if let Some(filter) = filter {
                filter_assessors.insert(*num_id, Rc::new(RefCell::new(filter)));
            }
        }

        let _ = self.unit_property_filters.set(filter_assessors);

        let _ = self
            .applied_unit_property_filters
            .set(Rc::new(RefCell::new(Vec::new())));

        // let search_entry = self.fill_search_bar();

        let custom_filter = self.create_custom_filter();
        self.filter_list_model
            .borrow()
            .set_filter(Some(&custom_filter));

        self.filter_list_model
            .borrow()
            .bind_property::<gtk::Label>("n-items", self.unit_filtered_count.as_ref(), "label")
            .build();

        let search_controls = UnitListSearchControls::new(&self.obj());
        self.search_bar.set_child(Some(&search_controls));

        self.search_controls
            .set(search_controls)
            .expect("Search entry set once");

        self.obj().action_set_enabled("win.col", true);

        {
            let unit_list = self.obj().clone();
            let units_browser = self.units_browser.borrow().clone();
            self.scrolled_window
                .vadjustment()
                .connect_changed(move |_adjustment| {
                    focus_on_row(&unit_list, &units_browser);

                    //UnitListPanelImp::print_scroll_adj_logs(unit_list.imp())
                });
        }

        settings
            .bind(
                KEY_PREF_UNIT_LIST_DISPLAY_SUMMARY,
                &self.summary.get(),
                "visible",
            )
            .build();

        self.units_browser
            .borrow()
            .connect_activate(|_a, row| info!("Unit row position {row}")); //TODO make selection

        pop_menu::setup_popup_menu(
            &self.units_browser.borrow(),
            &self.filter_list_model.borrow(),
            &self.obj(),
        );

        //TODO Code to be removed when migration to Toml will finish
        if generated {
            let col_map = self.generate_column_map();

            for (_, key, _, flags) in &*UNIT_LIST_COLUMNS {
                let Some(column_view_column) = col_map.get(*key) else {
                    warn!("Can't bind setting key {key} to column {key}");
                    continue;
                };

                if flags & FLAG_SHOW != 0 {
                    let setting_key = format!("{COL_SHOW_PREFIX}{key}");

                    let visible = settings.boolean(&setting_key);
                    column_view_column.set_visible(visible);
                    /*  let action = settings.create_action(&setting_key);
                    app_window.add_action(&action);

                    settings
                        .bind(&setting_key, column_view_column, "visible")
                        .build(); */
                }

                if flags & FLAG_WIDTH != 0 {
                    let setting_key = format!("{COL_WIDTH_PREFIX}{key}");

                    let width = settings.int(&setting_key);
                    column_view_column.set_fixed_width(width);
                    /*             settings
                    .bind(&setting_key, column_view_column, "fixed-width")
                    .build(); */
                }
            }
        }
        force_expand_on_the_last_visible_column(&self.units_browser.borrow().columns());
    }
}

fn focus_on_row(unit_list: &super::UnitListPanel, units_browser: &gtk::ColumnView) {
    let Some(mut force_selected_index) = unit_list.imp().force_selected_index.get() else {
        return;
    };

    unit_list.imp().force_selected_index.set(None);

    if force_selected_index == gtk::INVALID_LIST_POSITION {
        force_selected_index = 0;
    }

    info!("Focus on selected unit list row (index {force_selected_index})");

    let units_browser = units_browser.clone();
    //needs a bit of time to rendering  the list, then I found this hack
    glib::spawn_future_local(async move {
        // Deactivate the button until the operation is done
        gio::spawn_blocking(move || {
            let sleep_duration = Duration::from_millis(100);
            std::thread::sleep(sleep_duration);
        })
        .await
        .expect("Task needs to finish successfully.");

        // Set sensitivity of button to `enable_button`

        units_browser.scroll_to(
            force_selected_index, // to centerish on the selected unit
            None,
            gtk::ListScrollFlags::FOCUS,
            None,
        );
    });
}

impl WidgetImpl for UnitListPanelImp {}
impl BoxImpl for UnitListPanelImp {}

async fn call_complete_unit(
    sender: &tokio::sync::mpsc::Sender<Vec<systemd::UpdatedUnitInfo>>,
    batch: Vec<(UnitDBusLevel, String, String)>,
) {
    let updates = match systemd::complete_unit_information(batch).await {
        Ok(updates) => updates,
        Err(error) => {
            warn!("Complete Unit Information Error: {error:?}");
            vec![]
        }
    };

    sender
        .send(updates)
        .await
        .expect("The channel needs to be open.");
}

async fn go_fetch_data(
    int_level: DbusLevel,
) -> Result<(HashMap<String, UnitInfo>, Vec<SystemdUnitFile>), SystemdErrors> {
    match int_level {
        DbusLevel::SystemAndSession => {
            let level_syst = UnitDBusLevel::System;
            let level_user = UnitDBusLevel::UserSession;

            let (sender_syst, receiver_syst) = tokio::sync::oneshot::channel();
            let (sender_user, receiver_user) = tokio::sync::oneshot::channel();

            runtime().spawn(async move {
                let t_syst =
                    tokio::spawn(systemd::list_units_description_and_state_async(level_syst));
                let t_user =
                    tokio::spawn(systemd::list_units_description_and_state_async(level_user));

                let joined = tokio::join!(t_syst, t_user);

                sender_syst
                    .send(joined.0)
                    .expect("The channel needs to be open.");
                sender_user
                    .send(joined.1)
                    .expect("The channel needs to be open.");
            });

            let (loaded_unit_system, mut unit_file_system) =
                receiver_syst.await.expect("Tokio receiver works")??;
            let (loaded_unit_user, mut unit_file_user) =
                receiver_user.await.expect("Tokio receiver works")??;

            let mut hmap =
                HashMap::with_capacity(loaded_unit_system.len() + loaded_unit_user.len());

            for listed_unit in loaded_unit_system.into_iter() {
                let unit = UnitInfo::from_listed_unit(listed_unit, level_syst);
                hmap.insert(unit.primary(), unit);
            }

            for listed_unit in loaded_unit_user.into_iter() {
                let unit = UnitInfo::from_listed_unit(listed_unit, level_user);
                hmap.insert(unit.primary(), unit);
            }

            unit_file_system.append(&mut unit_file_user);
            Ok((hmap, unit_file_system))
        }
        dlevel => {
            let level: UnitDBusLevel = if dlevel == DbusLevel::System {
                UnitDBusLevel::System
            } else {
                UnitDBusLevel::UserSession
            };

            let (sender, receiver) = tokio::sync::oneshot::channel();

            runtime().spawn(async move {
                // let response = systemd::list_units_description_and_state_async().await;

                let response = systemd::list_units_description_and_state_async(level).await;
                sender
                    .send(response)
                    .expect("The channel needs to be open.");
            });

            let (loaded_unit, unit_files) = receiver.await.expect("Tokio receiver works")?;

            let mut hmap = HashMap::with_capacity(loaded_unit.len());
            for listed_unit in loaded_unit.into_iter() {
                let unit = UnitInfo::from_listed_unit(listed_unit, level);
                hmap.insert(unit.primary(), unit);
            }
            Ok((hmap, unit_files))
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_reverse() {
        for i in (0..10).rev() {
            println!("{i}")
        }
    }
}
