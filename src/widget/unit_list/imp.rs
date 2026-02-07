mod column_factories;
#[macro_use]
mod construct;
pub mod pop_menu;

use std::{
    cell::{Cell, OnceCell, Ref, RefCell, RefMut},
    collections::{HashMap, HashSet},
    rc::Rc,
    time::Duration,
};

use crate::{
    consts::{ACTION_UNIT_LIST_FILTER, ACTION_UNIT_LIST_FILTER_CLEAR, ALL_FILTER_KEY, FILTER_MARK},
    systemd::{
        self, SystemdUnitFile,
        data::{UnitInfo, convert_to_string},
        enums::{LoadState, UnitType},
        errors::SystemdErrors,
    },
    systemd_gui, upgrade,
    widget::{
        InterPanelMessage,
        app_window::AppWindow,
        preferences::data::{
            DbusLevel, KEY_PREF_UNIT_LIST_DISPLAY_COLORS, KEY_PREF_UNIT_LIST_DISPLAY_SUMMARY,
            PREFERENCES,
        },
        unit_list::{
            COL_ID_UNIT, CustomPropertyId, UnitListView,
            filter::{
                UnitListFilterWindow, custom_bool, custom_num, custom_str, filter_active_state,
                filter_bus_level, filter_enable_status, filter_load_state, filter_preset,
                filter_sub_state, filter_unit_description, filter_unit_name, filter_unit_type,
                unit_prop_filter::{
                    FilterBool, FilterElement, FilterNum, FilterText, UnitPropertyAssessor,
                    UnitPropertyFilter, UnitPropertyFilterType,
                },
            },
            get_clean_col_title,
            imp::construct::{default_column_definition_list, generate_loaded_units_columns},
            search_controls::UnitListSearchControls,
        },
        unit_properties_selector::{
            data_selection::UnitPropertySelection,
            save::{self},
        },
    },
};
use base::enums::UnitDBusLevel;
use glib::WeakRef;
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
use log::{debug, error, info, warn};
use strum::IntoEnumIterator;
use systemd::CompleteUnitParams;
use zvariant::{OwnedValue, Value};

const PREF_UNIT_LIST_VIEW: &str = "pref-unit-list-view";
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

type UnitPropertyFiltersContainer =
    RefCell<HashMap<String, Rc<RefCell<Box<dyn UnitPropertyFilter>>>>>;
type AppliedUnitPropertyFilters = OnceCell<Rc<RefCell<Vec<Box<dyn UnitPropertyAssessor>>>>>;

#[derive(Default, gtk::CompositeTemplate, Properties)]
#[template(resource = "/io/github/plrigaux/sysd-manager/unit_list_panel.ui")]
#[properties(wrapper_type = super::UnitListPanel)]
pub struct UnitListPanelImp {
    list_store: OnceCell<gio::ListStore>,

    units_map: Rc<RefCell<HashMap<UnitKey, UnitInfo>>>,

    unit_list_sort_list_model: RefCell<gtk::SortListModel>,

    units_browser: OnceCell<gtk::ColumnView>,

    single_selection: OnceCell<gtk::SingleSelection>,

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

    refresh_unit_list_button: WeakRef<gtk::Button>,

    unit: RefCell<Option<UnitInfo>>,

    pub force_selected_index: Cell<Option<u32>>,

    #[property(name = "display-color", get, set)]
    pub display_color: Cell<bool>,

    pub unit_property_filters: UnitPropertyFiltersContainer,

    pub applied_unit_property_filters: AppliedUnitPropertyFilters,

    app_window: OnceCell<AppWindow>,

    current_column_view_column_definition_list: RefCell<Vec<UnitPropertySelection>>,

    default_column_view_column_definition_list: OnceCell<Vec<UnitPropertySelection>>,

    #[property(get, set, default)]
    selected_list_view: Cell<UnitListView>,
}

macro_rules! update_search_entry {
    ($self:expr, $id:expr, $update_widget:expr, $text:expr) => {{
        if $update_widget && $id == COL_ID_UNIT {
            $self.search_entry_set_text($text);
        }
    }};
}

macro_rules! single_selection {
    ($self:expr) => {
        $self.single_selection.get().unwrap()
    };
}

macro_rules! units_browser {
    ($self:expr) => {
        $self.units_browser.get().unwrap()
    };
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
        self.app_window
            .set(app_window.clone())
            .expect("app_window set once");

        let app_window_clone = app_window.clone();
        let unit_list = self.obj().clone();

        single_selection!(self).connect_selected_item_notify(move |single_selection| {
            info!(
                "connect_selected_notify idx {}",
                single_selection.selected()
            );

            let Some(object) = single_selection.selected_item() else {
                warn!("No unit selected");
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
            .set(Some(refresh_unit_list_button));

        self.fill_store();

        let units_browser = units_browser!(self).clone();
        let action_entry = {
            gio::ActionEntry::builder("hide_unit_col")
                .activate(move |_application: &AppWindow, _b, target_value| {
                    if let Some(value) = target_value {
                        let key = Some(value.get::<String>().expect("variant always be String"));

                        let columns_list_model = units_browser.columns();

                        for cur_column in columns_list_model
                            .iter::<gtk::ColumnViewColumn>()
                            .filter_map(|item| item.ok())
                            .filter(|col| col.id().map(|s| s.to_string()) == key)
                        {
                            cur_column.set_visible(false);
                        }
                    }
                })
                .parameter_type(Some(VariantTy::STRING))
                .build()
        };

        let list_filter_action_entry = {
            let unit_list_panel = self.obj().clone();
            gio::ActionEntry::builder(ACTION_UNIT_LIST_FILTER)
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
                .activate(move |_application: &AppWindow, _b, target_value| {
                    if let Some(v) = target_value
                        && let Some(filter_key) = v.get::<String>()
                    {
                        unit_list_panel.imp().clear_filters(&filter_key);
                    }
                })
                .parameter_type(Some(VariantTy::STRING))
                .build()
        };

        let refresh_unit_list = {
            let unit_list_panel = self.obj().clone();
            gio::ActionEntry::builder("refresh_unit_list")
                .activate(move |_application: &AppWindow, _, _| {
                    info!("Action refresh called");
                    unit_list_panel.imp().fill_store();
                })
                .build()
        };

        let mut entries = vec![
            action_entry,
            list_filter_action_entry,
            list_filter_action_entry_blank,
            list_filter_clear_action_entry,
            refresh_unit_list,
        ];

        for unit_list_view in UnitListView::iter() {
            let action_entry = {
                let unit_list_panel = self.obj().clone();
                gio::ActionEntry::builder(UnitListView::base_action())
                    .activate(move |_application: &AppWindow, action, value| {
                        let Some(value) = value else {
                            warn!("{} has no value", UnitListView::base_action());
                            return;
                        };

                        debug!("{} target {value:?}", UnitListView::base_action());
                        let panel = unit_list_panel.imp();

                        action.set_state(value);

                        let view: UnitListView = value.into(); //FIXME Why can't use unit_list_view variable
                        debug!("new {:?}", view);
                        unit_list_panel.set_selected_list_view(view);
                        panel.fill_store();
                    })
                    .state(unit_list_view.id().to_variant())
                    .parameter_type(Some(glib::VariantTy::STRING))
                    .build()
            };
            entries.push(action_entry);
        }

        app_window.add_action_entries(entries);

        let settings = systemd_gui::new_settings();
        let view = settings.string(PREF_UNIT_LIST_VIEW);

        app_window.change_action_state(UnitListView::base_action(), &view.to_variant());
        debug!("VIEW : {}", view);
    }

    fn generate_column_list(&self) -> Vec<gtk::ColumnViewColumn> {
        let list_model: gio::ListModel = units_browser!(self).columns();

        let mut col_list = Vec::with_capacity(list_model.n_items() as usize);

        for column_view_column in list_model
            .iter::<gtk::ColumnViewColumn>()
            .filter_map(|item| match item {
                Ok(item) => Some(item),
                Err(err) => {
                    error!("Expect gtk::ColumnViewColumn> {err:?}");
                    None
                }
            })
        {
            col_list.push(column_view_column);
        }
        col_list
    }

    fn fill_store(&self) {
        let view = self.selected_list_view.get();
        debug!("fill store {:?}", view);
        match view {
            UnitListView::Defaut => {
                let cols = default_column_definition_list(self.display_color.get());
                save::load_column_config(view);
                self.set_new_columns(cols);
                self.fill_store_default()
            }
            UnitListView::ActiveUnit => {
                let cols = generate_loaded_units_columns(self.display_color.get());
                self.set_new_columns(cols);
                self.fill_store_loaded()
            }
            UnitListView::UnitFiles => {}
            UnitListView::Timers => {}
            UnitListView::Sockets => {}
            UnitListView::Custom => {
                warn!("TODO Load custom");
                self.fill_store_default()
            }
        }
    }

    fn fill_store_default(&self) {
        let list_store = self.list_store.get().expect("LIST STORE NOT NONE").clone();
        let main_unit_map_rc = self.units_map.clone();
        let panel_stack = self.panel_stack.clone();
        let single_selection = single_selection!(self).clone();
        let unit_list = self.obj().clone();
        let units_browser = units_browser!(self).clone();

        let refresh_unit_list_button = upgrade!(self.refresh_unit_list_button);

        //Rem sorting before adding lot of items for performance reasons
        self.unit_list_sort_list_model
            .borrow()
            .set_sorter(None::<&gtk::Sorter>);

        let dbus_level = PREFERENCES.dbus_level();

        glib::spawn_future_local(async move {
            refresh_unit_list_button.set_sensitive(false);
            panel_stack.set_visible_child_name("spinner");

            let Ok((loaded_units_map, unit_from_files)) = go_fetch_data(dbus_level)
                .await
                .inspect_err(|err| warn!("Fail fetch unit list {err:?}"))
            else {
                panel_stack.set_visible_child_name("error");
                return;
            };

            unit_list
                .imp()
                .loaded_units_count
                .set_label(&loaded_units_map.len().to_string());
            unit_list
                .imp()
                .unit_files_number
                .set_label(&unit_from_files.len().to_string());

            let n_items = list_store.n_items();
            list_store.remove_all();
            let mut main_unit_map_rc = main_unit_map_rc.borrow_mut();
            main_unit_map_rc.clear();

            let mut all_units =
                HashMap::with_capacity(loaded_units_map.len() + unit_from_files.len());

            for system_unit_file in unit_from_files.into_iter() {
                if let Some(loaded_unit) = loaded_units_map.get(&system_unit_file.full_name) {
                    loaded_unit.update_from_unit_file(system_unit_file);
                } else {
                    let unit = UnitInfo::from_unit_file(system_unit_file);
                    list_store.append(&unit);
                    main_unit_map_rc.insert(UnitKey::new(&unit), unit.clone());
                    all_units.insert(unit.primary(), unit);
                }
            }

            for unit in loaded_units_map.into_values() {
                list_store.append(&unit);
                main_unit_map_rc.insert(UnitKey::new(&unit), unit.clone());
                all_units.insert(unit.primary(), unit);
            }

            // The sort function needs to be the same of the  first column sorter
            let sort_func = construct::column_filter_lambda!(primary, dbus_level);

            list_store.sort(sort_func);

            info!("Unit list refreshed! list size {}", list_store.n_items());

            let mut force_selected_index = gtk::INVALID_LIST_POSITION;

            let selected_unit = unit_list.imp().selected_unit();
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

            //Complete unit information
            glib::spawn_future_local(async move {
                //let (sender, receiver) = tokio::sync::oneshot::channel();

                let (sender, mut receiver) = tokio::sync::mpsc::channel(100);

                let list: Vec<CompleteUnitParams> = all_units
                    .values()
                    .filter(|unit| unit.need_to_be_completed())
                    .map(CompleteUnitParams::new)
                    .collect();

                systemd::runtime().spawn(async move {
                    const BATCH_SIZE: usize = 5;
                    let mut batch = Vec::with_capacity(BATCH_SIZE);
                    for (idx, triple) in (1..).zip(list.into_iter()) {
                        batch.push(triple);

                        if idx % BATCH_SIZE == 0 {
                            call_complete_unit(&sender, &batch).await;
                            batch.clear();
                        }
                    }

                    call_complete_unit(&sender, &batch).await;
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
            //unit_list.imp().fetch_custom_unit_properties();
        });
    }

    fn fill_store_loaded(&self) {
        let list_store = self.list_store.get().expect("LIST STORE NOT NONE").clone();
        let main_unit_map_rc = self.units_map.clone();
        let panel_stack = self.panel_stack.clone();
        let single_selection = single_selection!(self).clone();
        let unit_list_panel = self.obj().clone();
        let units_browser = units_browser!(self).clone();

        let refresh_unit_list_button = upgrade!(self.refresh_unit_list_button);

        //Rem sorting before adding lot of items for performance reasons
        self.unit_list_sort_list_model
            .borrow()
            .set_sorter(None::<&gtk::Sorter>);

        let dbus_level = PREFERENCES.dbus_level();

        glib::spawn_future_local(async move {
            refresh_unit_list_button.set_sensitive(false);
            panel_stack.set_visible_child_name("spinner");

            let loaded_units_map = match go_fetch_data_loaded(dbus_level).await {
                Ok(value) => value,
                Err(err) => {
                    warn!("Fail fetch unit list {err:?}");
                    panel_stack.set_visible_child_name("error");
                    return;
                }
            };

            unit_list_panel
                .imp()
                .loaded_units_count
                .set_label(&loaded_units_map.len().to_string());
            unit_list_panel.imp().unit_files_number.set_label("");

            let n_items = list_store.n_items();
            list_store.remove_all();
            let mut main_unit_map_rc = main_unit_map_rc.borrow_mut();
            main_unit_map_rc.clear();

            for unit in loaded_units_map.into_values() {
                list_store.append(&unit);
                main_unit_map_rc.insert(UnitKey::new(&unit), unit.clone());
            }

            // The sort function needs to be the same of the  first column sorter
            let sort_func = construct::column_filter_lambda!(primary, dbus_level);

            list_store.sort(sort_func);

            info!("Unit list refreshed! list size {}", list_store.n_items());

            let mut force_selected_index = gtk::INVALID_LIST_POSITION;

            let selected_unit = unit_list_panel.imp().selected_unit();
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
            unit_list_panel
                .imp()
                .force_selected_index
                .set(Some(force_selected_index));
            refresh_unit_list_button.set_sensitive(true);
            unit_list_panel.imp().set_sorter();

            //cause no scrollwindow v adjustment
            if n_items > 0 {
                focus_on_row(&unit_list_panel, &units_browser);
            }
            panel_stack.set_visible_child_name("unit_list");

            //unit_list.imp().fetch_custom_unit_properties();
        });
    }

    pub(super) fn button_search_toggled(&self, toggle_button_is_active: bool) {
        self.search_bar.set_search_mode(toggle_button_is_active);

        if toggle_button_is_active {
            let s_controls = self.search_controls.get().unwrap();
            s_controls.grab_focus_on_search_entry();

            let applied_assessors = self
                .applied_unit_property_filters
                .get()
                .expect("applied_assessors not null");

            //TODO report a bug because the adw::ButtonContent doesn't hinerit it's parent
            //sensitivity when hidden
            s_controls.set_filter_is_set(!applied_assessors.borrow().is_empty());
        }
    }

    pub fn set_unit_internal(&self, unit: &UnitInfo) {
        let _ = self.unit.replace(Some(unit.clone()));
    }

    pub fn set_unit(&self, unit: Option<&UnitInfo>) -> Option<UnitInfo> {
        let Some(unit) = unit else {
            self.unit.replace(None);
            return None;
        };

        //FIXME update the data
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
            single_selection!(self).set_selected(gtk::INVALID_LIST_POSITION);
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

            units_browser!(self).scroll_to(
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

    pub fn set_inter_message(&self, _action: &InterPanelMessage) {}

    fn set_sorter(&self) {
        let sorter = units_browser!(self).sorter();

        self.unit_list_sort_list_model
            .borrow()
            .set_sorter(sorter.as_ref());

        let item_out = units_browser!(self)
            .columns()
            .item(0)
            .expect("Expect item x to be not None");

        //Sort on first column
        let first_column = item_out
            .downcast_ref::<gtk::ColumnViewColumn>()
            .expect("item.downcast_ref::<gtk::ColumnViewColumn>()");

        units_browser!(self).sort_by_column(Some(first_column), gtk::SortType::Ascending);
    }

    pub(super) fn filter_assessor_change(
        &self,
        id: &str,
        new_assessor: Option<Box<dyn UnitPropertyAssessor>>,
        change_type: Option<gtk::FilterChange>,
        update_widget: bool,
    ) {
        debug!("Assessor Change {new_assessor:?} Change Type: {change_type:?}");
        let applied_assessors = self
            .applied_unit_property_filters
            .get()
            .expect("applied_assessors not null");

        let add = if let Some(new_assessor) = new_assessor {
            debug!("Add filter id {id}");

            update_search_entry!(self, id, update_widget, new_assessor.text());

            let mut vect = applied_assessors.borrow_mut();

            if let Some(index) = vect.iter().position(|x| x.id() == id) {
                vect[index] = new_assessor;
            } else {
                vect.push(new_assessor);
            }
            true
        } else {
            debug!("Remove filter id {id}");

            applied_assessors.borrow_mut().retain(|x| x.id() != id);

            update_search_entry!(self, id, update_widget, "");
            false
        };

        self.set_filter_column_header_marker(add, id);

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
        search_controls.set_filter_is_set(!applied_assessors.borrow().is_empty());
    }

    fn set_filter_column_header_marker(&self, add: bool, id: &str) {
        if let Some(unit_prop_selection) = self
            .current_column_view_column_definition_list
            .borrow()
            .iter()
            .find(|unit_prop_selection| {
                unit_prop_selection
                    .column()
                    .id()
                    .is_some_and(|cid| cid == id)
            })
        {
            let col = unit_prop_selection.column();

            let title = if let Some(title) = col.title() {
                title.to_string()
            } else {
                "".to_string()
            };

            let new_title = if add {
                if !title.starts_with(FILTER_MARK) {
                    format!("{FILTER_MARK} {title}")
                } else {
                    title
                }
            } else {
                get_clean_col_title(&title)
            };
            col.set_title(Some(&new_title));
        }
    }

    pub(super) fn clear_filters(&self, filter_key: &str) {
        for property_filter in self.unit_property_filters.borrow().values() {
            let mut prop_filter_mut = property_filter.borrow_mut();

            if filter_key == prop_filter_mut.id() || filter_key == ALL_FILTER_KEY {
                prop_filter_mut.clear_n_apply_filter();
            }
        }

        if filter_key == COL_ID_UNIT || filter_key == ALL_FILTER_KEY {
            let search_controls = self.search_controls.get().expect("Not Null");
            search_controls.imp().clear();
        }
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
        let Some(filter) = self.lazy_get_filter_assessor(COL_ID_UNIT, None) else {
            error!("No filter id {COL_ID_UNIT}");
            return;
        };

        let mut filter = filter.borrow_mut();

        let filter = filter
            .as_any_mut()
            .downcast_mut::<FilterText>()
            .expect("downcast to FilterText");

        filter.set_filter_elem(text, update_widget);
    }

    /*     pub(super) fn clear_unit_list_filter_window_dependancy(&self) {
        for property_filter in self.unit_property_filters.borrow().values() {
            property_filter.borrow_mut().clear_widget_dependancy();
        }
    } */

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

    pub(super) fn lazy_get_filter_assessor(
        &self,
        id: &str,
        propperty_type: Option<String>,
    ) -> Option<Rc<RefCell<Box<dyn UnitPropertyFilter>>>> {
        {
            if let Some(filter) = self.unit_property_filters.borrow().get(id) {
                return Some(filter.clone());
            }
        }

        let unit_list_panel: glib::BorrowedObject<'_, crate::widget::unit_list::UnitListPanel> =
            self.obj();

        let filter: Option<Box<dyn UnitPropertyFilter>> = match id {
            COL_ID_UNIT => Some(Box::new(FilterText::new(
                id,
                filter_unit_name,
                &unit_list_panel,
            ))),
            "sysdm-bus" => Some(Box::new(FilterElement::new(
                id,
                filter_bus_level,
                &unit_list_panel,
            ))),
            "sysdm-type" => Some(Box::new(FilterElement::new(
                id,
                filter_unit_type,
                &unit_list_panel,
            ))),
            "sysdm-state" => Some(Box::new(FilterElement::new(
                id,
                filter_enable_status,
                &unit_list_panel,
            ))),
            "sysdm-preset" => Some(Box::new(FilterElement::new(
                id,
                filter_preset,
                &unit_list_panel,
            ))),
            "sysdm-load" => Some(Box::new(FilterElement::new(
                id,
                filter_load_state,
                &unit_list_panel,
            ))),
            "sysdm-active" => Some(Box::new(FilterElement::new(
                id,
                filter_active_state,
                &unit_list_panel,
            ))),
            "sysdm-sub" => Some(Box::new(FilterElement::new(
                id,
                filter_sub_state,
                &unit_list_panel,
            ))),
            "sysdm-description" => Some(Box::new(FilterText::new(
                id,
                filter_unit_description,
                &unit_list_panel,
            ))),
            _ => match propperty_type.as_deref() {
                Some("t") => Some(Box::new(FilterNum::<u64>::new(
                    id,
                    custom_num::<u64>,
                    &unit_list_panel,
                    CustomPropertyId::from_str(id).quark(),
                    UnitPropertyFilterType::NumU64,
                ))),
                Some("s") => Some(Box::new(FilterText::newq(
                    id,
                    custom_str,
                    &unit_list_panel,
                    CustomPropertyId::from_str(id).quark(),
                ))),
                Some("i") => Some(Box::new(FilterNum::<i32>::new(
                    id,
                    custom_num::<i32>,
                    &unit_list_panel,
                    CustomPropertyId::from_str(id).quark(),
                    UnitPropertyFilterType::NumI32,
                ))),
                Some("u") => Some(Box::new(FilterNum::<u32>::new(
                    id,
                    custom_num::<u32>,
                    &unit_list_panel,
                    CustomPropertyId::from_str(id).quark(),
                    UnitPropertyFilterType::NumU32,
                ))),

                Some("b") => Some(Box::new(FilterBool::new(
                    id,
                    custom_bool,
                    &unit_list_panel,
                    CustomPropertyId::from_str(id).quark(),
                ))),
                Some("q") => Some(Box::new(FilterNum::<u16>::new(
                    id,
                    custom_num::<u16>,
                    &unit_list_panel,
                    CustomPropertyId::from_str(id).quark(),
                    UnitPropertyFilterType::NumU16,
                ))),
                Some("x") => Some(Box::new(FilterNum::<i64>::new(
                    id,
                    custom_num::<i64>,
                    &unit_list_panel,
                    CustomPropertyId::from_str(id).quark(),
                    UnitPropertyFilterType::NumI64,
                ))),
                Some(&_) => Some(Box::new(FilterText::newq(
                    id,
                    custom_str,
                    &unit_list_panel,
                    CustomPropertyId::from_str(id).quark(),
                ))),
                None => {
                    error!(
                        "Filtering for key {id:?} not handled yet, data type {propperty_type:?}"
                    );
                    None
                }
            },
        };

        if let Some(filter) = filter {
            let mut unit_property_filters = self.unit_property_filters.borrow_mut();
            let filter = Rc::new(RefCell::new(filter));
            unit_property_filters.insert(id.to_string(), filter.clone());

            Some(filter)
        } else {
            None
        }
    }

    pub(super) fn set_new_columns(&self, property_list: Vec<UnitPropertySelection>) {
        if property_list.is_empty() {
            warn!("Column list empty, Abort");
            return;
        }

        let columns_list_model = units_browser!(self).columns();

        //Get the current column
        let cur_n_items = columns_list_model.n_items();
        let mut current_columns_over = Vec::with_capacity(columns_list_model.n_items() as usize);
        for position in (property_list.len() as u32)..columns_list_model.n_items() {
            let Some(column) = columns_list_model
                .item(position)
                .and_downcast::<gtk::ColumnViewColumn>()
            else {
                warn!("Col None");
                continue;
            };
            current_columns_over.push(column);
        }

        let units_browser = units_browser!(self);
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
                units_browser.append_column(&new_column);
            }
        }

        self.current_column_view_column_definition_list
            .replace(property_list);

        //remove all columns that exceed the new ones
        for column in current_columns_over.iter() {
            units_browser.remove_column(column);
        }

        force_expand_on_the_last_visible_column(&columns_list_model);

        self.fetch_custom_unit_properties();
    }

    fn fetch_custom_unit_properties(&self) {
        info!("!!! Fetching custom unit properties !!!");
        let property_list = self.current_column_view_column_definition_list.borrow();

        if property_list.is_empty() {
            info!("No properties to fetch");
            return;
        }

        let current_property_list = property_list.clone();

        /*        let list_len = property_list.len();
        warn!("Property list size {} ", list_len); */
        let mut property_list_send = Vec::with_capacity(current_property_list.len());
        let mut property_list_keys = Vec::with_capacity(current_property_list.len());
        let mut types = HashSet::with_capacity(16);
        let mut is_unit_type = false;
        for unit_property in current_property_list.iter() {
            if unit_property.is_custom() {
                //Add custom factory
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

        if property_list_send.is_empty() {
            info!("No custom property to fetch");
            return;
        }

        let list_len = property_list_send.len();
        let units_browser = units_browser!(self).clone();
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
            systemd::runtime().spawn(async move {
                info!("Fetching properties START for {} units", units_list.len());
                for (level, primary_name, object_path, unit_type) in units_list {
                    let mut property_value_list = vec![None; list_len];
                    for (index, unit_property) in property_list_send.iter().enumerate() {
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
                            Ok(value) => property_value_list[index] = Some(value),
                            Err(err) => {
                                debug!(
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
                //info!("Got properties for {:?}", property_value_list);
                let map_ref = units_map.borrow();
                let Some(unit) = map_ref.get(&key) else {
                    continue;
                };

                for (index, value) in property_value_list.into_iter().enumerate() {
                    let Some(key) = property_list_keys.get(index) else {
                        error!(
                            "No key for index {index} key len {}",
                            property_list_keys.len()
                        );
                        panic!("Should never fail");
                    };

                    Self::insert_value(*key, value, unit);
                }
            }
            info!("Fetching properties FINISHED");

            //Force the factory to display data
            for column in units_browser
                .columns()
                .iter::<gtk::ColumnViewColumn>()
                .filter_map(|item| item.ok())
            {
                let prop_type = current_property_list.iter().find_map(|prop_selection| {
                    if prop_selection.id() == column.id() {
                        prop_selection.prop_type()
                    } else {
                        None
                    }
                });

                construct::set_column_factory_and_sorter(&column, display_color, prop_type);
            }
        });
    }

    fn insert_value(key: Quark, value: Option<OwnedValue>, unit: &UnitInfo) {
        let Some(value) = value else {
            unsafe { unit.steal_qdata::<OwnedValue>(key) };
            return;
        };

        //let value_ref = &value as &Value;
        match &value as &Value {
            Value::Bool(b) => unsafe { unit.set_qdata(key, *b) },
            Value::U8(i) => unsafe { unit.set_qdata(key, *i) },
            Value::I16(i) => unsafe { unit.set_qdata(key, *i) },
            Value::U16(i) => unsafe { unit.set_qdata(key, *i) },
            Value::I32(i) => unsafe { unit.set_qdata(key, *i) },
            Value::U32(i) => unsafe { unit.set_qdata(key, *i) },
            Value::I64(i) => unsafe { unit.set_qdata(key, *i) },
            Value::U64(i) => unsafe { unit.set_qdata(key, *i) },
            Value::F64(i) => unsafe { unit.set_qdata(key, *i) },
            Value::Str(s) => {
                if s.is_empty() {
                    unsafe { unit.steal_qdata::<String>(key) };
                } else {
                    unsafe { unit.set_qdata(key, s.to_string()) };
                }
            }
            Value::Signature(s) => unsafe { unit.set_qdata(key, s.to_string()) },
            Value::ObjectPath(op) => unsafe { unit.set_qdata(key, op.to_string()) },
            Value::Value(v) => unsafe { unit.set_qdata(key, v.to_string()) },
            Value::Array(a) => {
                if a.is_empty() {
                    unsafe { unit.steal_qdata::<String>(key) };
                } else {
                    let mut d_str = String::from("");

                    let mut it = a.iter().peekable();
                    while let Some(mi) = it.next() {
                        if let Some(v) = convert_to_string(mi) {
                            d_str.push_str(&v);
                        }
                        if it.peek().is_some() {
                            d_str.push_str(", ");
                        }
                    }

                    unsafe { unit.set_qdata(key, d_str) };
                }
            }
            Value::Dict(d) => {
                let mut it = d.iter().peekable();
                if it.peek().is_none() {
                    unsafe { unit.steal_qdata::<String>(key) };
                } else {
                    let mut d_str = String::from("{ ");

                    for (mik, miv) in it {
                        if let Some(k) = convert_to_string(mik) {
                            d_str.push_str(&k);
                        }
                        d_str.push_str(" : ");

                        if let Some(v) = convert_to_string(miv) {
                            d_str.push_str(&v);
                        }
                    }
                    d_str.push_str(" }");

                    unsafe { unit.set_qdata(key, d_str) };
                }
            }
            Value::Structure(stc) => {
                let mut it = stc.fields().iter().peekable();

                if it.peek().is_none() {
                    unsafe { unit.steal_qdata::<String>(key) };
                } else {
                    let v: Vec<String> = it
                        .filter_map(|v| convert_to_string(v))
                        .filter(|s| !s.is_empty())
                        .collect();
                    let d_str = v.join(", ");

                    unsafe { unit.set_qdata(key, d_str) };
                }
            }
            Value::Fd(fd) => unsafe { unit.set_qdata(key, fd.to_string()) },
            //Value::Maybe(maybe) => (maybe.to_string(), false),
        }
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

    pub(super) fn current_columns_mut(&self) -> RefMut<'_, Vec<UnitPropertySelection>> {
        self.current_column_view_column_definition_list.borrow_mut()
    }

    pub(super) fn columns(&self) -> gio::ListModel {
        units_browser!(self).columns()
    }

    pub(super) fn default_displayed_columns(&self) -> &Vec<UnitPropertySelection> {
        self.default_column_view_column_definition_list
            .get_or_init(|| construct::default_column_definition_list(self.display_color.get()))
    }

    pub(super) fn save_config(&self) {
        let view = self.selected_list_view.get();
        save::save_column_config(
            Some(&units_browser!(self).columns()),
            &mut self.current_columns_mut(),
            view,
        );
    }
}

fn force_expand_on_the_last_visible_column(columns_list_model: &gio::ListModel) {
    if let Some(column) = columns_list_model
        .iter::<gtk::ColumnViewColumn>()
        .rev()
        .filter_map(|item| item.ok())
        .next()
    {
        column.set_expand(true);
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

        let unit_list_panel = self.obj().clone();

        let list_store = gio::ListStore::new::<UnitInfo>();
        self.list_store
            .set(list_store.clone())
            .expect("Set only Once");

        settings
            .bind(PREF_UNIT_LIST_VIEW, &unit_list_panel, "selected-list-view")
            .mapping(|variant, _| {
                let unit_list_view: UnitListView = variant.into();
                let value = unit_list_view.to_value();
                Some(value)
            })
            .set_mapping(|value, _| {
                let unit_list_view = value
                    .get::<UnitListView>()
                    .inspect_err(|err| warn!("Conv error {:?}", err))
                    .unwrap_or(UnitListView::Defaut);
                let variant = unit_list_view.id().to_variant();
                Some(variant)
            })
            .build();

        // let view = settings.string(PREF_UNIT_LIST_VIEW);
        // debug!("VIEW1 : {}", view);
        let view = self.selected_list_view.get();
        info!("Selected Browser View : {:?}", view);

        let sort_list_model = gtk::SortListModel::new(Some(list_store), None::<gtk::Sorter>);
        let filter_list_model =
            gtk::FilterListModel::new(Some(sort_list_model.clone()), None::<gtk::Filter>);
        let single_selection = gtk::SingleSelection::builder()
            .model(&filter_list_model)
            .autoselect(false)
            .build();
        let column_view = gtk::ColumnView::new(Some(single_selection.clone()));

        let column_view_column_definition_list =
            construct::construct_column_view(self.display_color.get(), view);

        for unit_property_selection in column_view_column_definition_list.iter() {
            column_view.append_column(&unit_property_selection.column());
        }

        let sorter = column_view.sorter();
        sort_list_model.set_sorter(sorter.as_ref());

        self.scrolled_window.set_child(Some(&column_view));
        println!("ATK");
        self.units_browser.get_or_init(|| column_view);
        // .expect_err("units browser shall be set only once");
        self.single_selection.get_or_init(|| single_selection);
        self.filter_list_model.replace(filter_list_model);
        self.unit_list_sort_list_model.replace(sort_list_model);

        let column_view_column_list = self.generate_column_list();

        self.current_column_view_column_definition_list
            .replace(column_view_column_definition_list);

        let current_column_view_column_definition_list =
            self.current_column_view_column_definition_list.borrow();
        column_factories::setup_factories(
            &unit_list_panel,
            &column_view_column_list,
            &current_column_view_column_definition_list,
        );

        settings.connect_changed(
            Some(KEY_PREF_UNIT_LIST_DISPLAY_COLORS),
            move |_settings, _key| {
                let display_color = unit_list_panel.display_color();
                info!("Change preference setting \"display color\" to {display_color}");
                let column_view_column_list = unit_list_panel.imp().generate_column_list();

                let current_column_view_column_definition_list = unit_list_panel
                    .imp()
                    .current_column_view_column_definition_list
                    .borrow();
                column_factories::setup_factories(
                    &unit_list_panel,
                    &column_view_column_list,
                    &current_column_view_column_definition_list,
                );
            },
        );

        let _ = self
            .applied_unit_property_filters
            .set(Rc::new(RefCell::new(Vec::new())));

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

        let units_browser = units_browser!(self);
        {
            let unit_list = self.obj().clone();
            let units_browser = units_browser.clone();
            self.scrolled_window
                .vadjustment()
                .connect_changed(move |_adjustment| {
                    focus_on_row(&unit_list, &units_browser);
                });
        }

        settings
            .bind(
                KEY_PREF_UNIT_LIST_DISPLAY_SUMMARY,
                &self.summary.get(),
                "visible",
            )
            .build();

        units_browser.connect_activate(|_a, row| info!("Unit row position {row}")); //TODO make selection

        pop_menu::UnitPopMenu::new(units_browser, &self.obj(), &self.filter_list_model.borrow());

        force_expand_on_the_last_visible_column(&units_browser.columns());
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
    batch: &[CompleteUnitParams],
) {
    let updates = systemd::complete_unit_information(batch)
        .await
        .inspect_err(|error| warn!("Complete Unit Information Error: {error:?}"))
        .unwrap_or(vec![]);

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

            systemd::runtime().spawn(async move {
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

            for listed_unit in loaded_unit_user.into_iter() {
                let unit = UnitInfo::from_listed_unit(listed_unit, level_user);
                hmap.insert(unit.primary(), unit);
            }

            for listed_unit in loaded_unit_system.into_iter() {
                let level = if let Some(_old_unit) = hmap.get(&listed_unit.primary_unit_name) {
                    UnitDBusLevel::Both
                } else {
                    level_syst
                };

                let unit = UnitInfo::from_listed_unit(listed_unit, level);
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

            systemd::runtime().spawn(async move {
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

async fn go_fetch_data_loaded(
    int_level: DbusLevel,
) -> Result<HashMap<String, UnitInfo>, SystemdErrors> {
    match int_level {
        DbusLevel::SystemAndSession => {
            let level_syst = UnitDBusLevel::System;
            let level_user = UnitDBusLevel::UserSession;

            let (sender_syst, receiver_syst) = tokio::sync::oneshot::channel();
            let (sender_user, receiver_user) = tokio::sync::oneshot::channel();

            systemd::runtime().spawn(async move {
                let t_syst = tokio::spawn(systemd::list_loaded_units(level_syst));
                let t_user = tokio::spawn(systemd::list_loaded_units(level_user));

                let joined = tokio::join!(t_syst, t_user);

                sender_syst
                    .send(joined.0)
                    .expect("The channel needs to be open.");
                sender_user
                    .send(joined.1)
                    .expect("The channel needs to be open.");
            });

            let loaded_unit_system = receiver_syst.await.expect("Tokio receiver works")??;
            let loaded_unit_user = receiver_user.await.expect("Tokio receiver works")??;

            let mut hmap =
                HashMap::with_capacity(loaded_unit_system.len() + loaded_unit_user.len());

            for listed_unit in loaded_unit_user.into_iter() {
                let unit = UnitInfo::from_listed_unit(listed_unit, level_user);
                hmap.insert(unit.primary(), unit);
            }

            for listed_unit in loaded_unit_system.into_iter() {
                let level = if let Some(_old_unit) = hmap.get(&listed_unit.primary_unit_name) {
                    UnitDBusLevel::Both
                } else {
                    level_syst
                };

                let unit = UnitInfo::from_listed_unit(listed_unit, level);
                hmap.insert(unit.primary(), unit);
            }

            Ok(hmap)
        }

        dlevel => {
            let level: UnitDBusLevel = if dlevel == DbusLevel::System {
                UnitDBusLevel::System
            } else {
                UnitDBusLevel::UserSession
            };

            let (sender, receiver) = tokio::sync::oneshot::channel();

            systemd::runtime().spawn(async move {
                // let response = systemd::list_units_description_and_state_async().await;

                let response = systemd::list_loaded_units(level).await;
                sender
                    .send(response)
                    .expect("The channel needs to be open.");
            });

            let loaded_unit = receiver.await.expect("Tokio receiver works")?;

            let mut hmap = HashMap::with_capacity(loaded_unit.len());
            for listed_unit in loaded_unit.into_iter() {
                let unit = UnitInfo::from_listed_unit(listed_unit, level);
                hmap.insert(unit.primary(), unit);
            }
            Ok(hmap)
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
