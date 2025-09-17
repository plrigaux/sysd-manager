mod column_factories;
mod menus;
mod pop_menu;
mod rowdata;

use std::{
    cell::{Cell, OnceCell, RefCell},
    collections::HashMap,
    rc::Rc,
    time::Duration,
};

use gio::glib::VariantTy;
use gtk::{
    TemplateChild,
    gio::{self},
    glib::{self, Object, Properties},
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
use menus::create_col_menu;

use crate::{
    consts::ACTION_UNIT_LIST_FILTER_CLEAR,
    systemd::{
        self, UnitProperty,
        data::UnitInfo,
        enums::{LoadState, UnitDBusLevel},
        runtime,
    },
    systemd_gui,
    widget::{
        InterPanelMessage,
        app_window::AppWindow,
        preferences::data::{
            COL_SHOW_PREFIX, COL_WIDTH_PREFIX, FLAG_SHOW, FLAG_WIDTH,
            KEY_PREF_UNIT_LIST_DISPLAY_COLORS, KEY_PREF_UNIT_LIST_DISPLAY_SUMMARY,
            UNIT_LIST_COLUMNS, UNIT_LIST_COLUMNS_UNIT,
        },
        unit_list::{
            filter::{
                UnitListFilterWindow, filter_active_state, filter_bus_level, filter_enable_status,
                filter_load_state, filter_preset, filter_sub_state, filter_unit_description,
                filter_unit_name, filter_unit_type,
                unit_prop_filter::{
                    FilterElement, FilterText, UnitPropertyAssessor, UnitPropertyFilter,
                },
            },
            imp::rowdata::UnitBinding,
            search_controls::UnitListSearchControls,
        },
    },
};

#[derive(Debug, Eq, PartialEq, Hash)]
struct UnitKey {
    level: UnitDBusLevel,
    primary: String,
}

impl UnitKey {
    fn new(unit: &UnitInfo) -> Self {
        UnitKey {
            level: unit.dbus_level(),
            primary: unit.primary(),
        }
    }
}

type UnitPropertyFiltersContainer = OnceCell<HashMap<u8, Rc<RefCell<Box<dyn UnitPropertyFilter>>>>>;
type AppliedUnitPropertyFilters = OnceCell<Rc<RefCell<Vec<Box<dyn UnitPropertyAssessor>>>>>;

#[derive(Default, gtk::CompositeTemplate, Properties)]
#[template(resource = "/io/github/plrigaux/sysd-manager/unit_list_panel.ui")]
#[properties(wrapper_type = super::UnitListPanel)]
pub struct UnitListPanelImp {
    #[template_child]
    list_store: TemplateChild<gio::ListStore>,

    unit_map: Rc<RefCell<HashMap<UnitKey, UnitInfo>>>,

    #[template_child]
    unit_list_sort_list_model: TemplateChild<gtk::SortListModel>,

    #[template_child]
    units_browser: TemplateChild<gtk::ColumnView>,

    #[template_child]
    single_selection: TemplateChild<gtk::SingleSelection>,

    #[template_child]
    search_bar: TemplateChild<gtk::SearchBar>,

    #[template_child]
    filter_list_model: TemplateChild<gtk::FilterListModel>,

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
}

macro_rules! compare_units {
    ($unit1:expr, $unit2:expr, $func:ident) => {{
        $unit1.$func().cmp(&$unit2.$func()).into()
    }};

    ($unit1:expr, $unit2:expr, $func:ident, $($funcx:ident),+) => {{

        let ordering = $unit1.$func().cmp(&$unit2.$func());
        if ordering != core::cmp::Ordering::Equal {
            return ordering.into();
        }

        compare_units!($unit1, $unit2, $($funcx),+)
    }};
}

macro_rules! create_column_filter {
    ($($func:ident),+) => {{
        gtk::CustomSorter::new(move |obj1, obj2| {
            let unit1 = obj1
                .downcast_ref::<UnitBinding>()
                .expect("Needs to be UnitInfo").unit_ref();
            let unit2 = obj2
                .downcast_ref::<UnitBinding>()
                .expect("Needs to be UnitInfo").unit_ref();

            compare_units!(unit1, unit2, $($func),+)
        })
    }};
}

macro_rules! column_view_column_set_sorter {
    ($map:expr, $col_id:expr, $($func:ident),+) => {{
        let column_view_column = $map.get($col_id)
            .expect(&format!("Column with id {:?} not found!", $col_id));
        let sorter = create_column_filter!($($func),+);
        column_view_column.set_sorter(Some(&sorter));
        let column_menu = create_col_menu($col_id);
        column_view_column.set_header_menu(Some(&column_menu));
    }};
}

macro_rules! update_search_entry {
    ($self:expr, $id:expr, $update_widget:expr, $text:expr) => {{
        if $update_widget && $id == UNIT_LIST_COLUMNS_UNIT {
            $self.search_entry_set_text($text);
        }
    }};
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
        let settings = systemd_gui::new_settings();

        self.app_window
            .set(app_window.clone())
            .expect("app_window set once");

        let app_window_clone = app_window.clone();
        let unit_list = self.obj().clone();

        self.single_selection
            .connect_selected_item_notify(move |single_selection| {
                info!(
                    "connect_selected_notify idx {}",
                    single_selection.selected()
                );
                let Some(object) = single_selection.selected_item() else {
                    warn!("No object selected");
                    return;
                };

                let unit_binding = match object.downcast::<UnitBinding>() {
                    Ok(unit) => unit,
                    Err(val) => {
                        error!("Object.downcast::<UnitInfo> Error: {val:?}");
                        return;
                    }
                };

                let unit = unit_binding.unit();
                info!("Selection changed, new unit {}", unit.primary());

                unit_list.imp().set_unit_internal(&unit);
                app_window_clone.selection_change(Some(&unit));
            }); // FOR THE SEARCH

        self.refresh_unit_list_button
            .set(refresh_unit_list_button.clone())
            .expect("refresh_unit_list_button was already set!");

        self.fill_store();

        let col_map = self.generate_column_map();

        for (_, key, _, flags) in &*UNIT_LIST_COLUMNS {
            let Some(column_view_column) = col_map.get(*key) else {
                warn!("Can't bind setting key {key} to column {key}");
                continue;
            };

            if flags & FLAG_SHOW != 0 {
                let setting_key = format!("{COL_SHOW_PREFIX}{key}");
                let action = settings.create_action(&setting_key);
                app_window.add_action(&action);

                settings
                    .bind(&setting_key, column_view_column, "visible")
                    .build();
            }

            if flags & FLAG_WIDTH != 0 {
                let setting_key = format!("{COL_WIDTH_PREFIX}{key}");
                settings
                    .bind(&setting_key, column_view_column, "fixed-width")
                    .build();
            }
        }

        let action_entry = {
            let settings = settings.clone();
            gio::ActionEntry::builder("hide_unit_col")
                .activate(move |_application: &AppWindow, _b, target_value| {
                    if let Some(value) = target_value {
                        let key = value.get::<String>().expect("variant always be String");
                        if let Err(error) = settings.set_boolean(&key, false) {
                            warn!("Setting error, key {key}, {error:?}");
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
                //     .parameter_type(Some(VariantTy::STRING))
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
        let list_model: gio::ListModel = self.units_browser.columns();

        let mut col_map = HashMap::new();
        for col_idx in 0..list_model.n_items() {
            let item_out = list_model
                .item(col_idx)
                .expect("Expect item x to be not None");

            let column_view_column = item_out
                .downcast_ref::<gtk::ColumnViewColumn>()
                .expect("item.downcast_ref::<gtk::ColumnViewColumn>()");

            let id = column_view_column.id();

            if let Some(id) = id {
                col_map.insert(id, column_view_column.clone());
            } else {
                warn!("Column {col_idx} has no id.")
            }
        }
        col_map
    }

    pub(super) fn fill_store(&self) {
        let list_store = self.list_store.clone();
        let unit_map = self.unit_map.clone();
        let panel_stack = self.panel_stack.clone();
        let single_selection = self.single_selection.clone();
        let unit_list = self.obj().clone();
        let units_browser = self.units_browser.clone();

        let refresh_unit_list_button = self
            .refresh_unit_list_button
            .get()
            .expect("Supposed to be set")
            .clone();

        // let sender_c = sender.clone();

        //Rem sorting before adding lot of items for performance reasons
        self.unit_list_sort_list_model
            .set_sorter(None::<&gtk::Sorter>);

        glib::spawn_future_local(async move {
            refresh_unit_list_button.set_sensitive(false);
            panel_stack.set_visible_child_name("spinner");
            let (sender, receiver) = tokio::sync::oneshot::channel();

            runtime().spawn(async move {
                let response = systemd::list_units_description_and_state_async().await;

                sender
                    .send(response)
                    .expect("The channel needs to be open.");
            });

            let (unit_desc, unit_from_files) = match receiver.await.expect("Tokio receiver works") {
                Ok(unit_files) => unit_files,
                Err(error) => {
                    warn!("Fail fetch unit list {error:?}");
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
                    list_store.append(&UnitBinding::new(&unit));
                    unit_map1.insert(UnitKey::new(&unit), unit.clone());
                    all_units.insert(unit.primary(), unit);
                }
            }

            for (_key, unit) in unit_desc.into_iter() {
                list_store.append(&UnitBinding::new(&unit));
                unit_map1.insert(UnitKey::new(&unit), unit.clone());
                all_units.insert(unit.primary(), unit);
            }

            // The sort function needs to be the same of the  first column sorter
            let sort_func = |o1: &Object, o2: &Object| {
                let u1 = o1
                    .downcast_ref::<UnitBinding>()
                    .expect("Needs to be UnitInfo")
                    .unit_ref();
                let u2 = o2
                    .downcast_ref::<UnitBinding>()
                    .expect("Needs to be UnitInfo")
                    .unit_ref();

                compare_units!(u1, u2, primary, dbus_level)
            };

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
                    let unit_binding = object
                        .downcast_ref::<UnitBinding>()
                        .expect("Needs to be UnitBinding");

                    unit_binding.unit_ref().primary().eq(&selected_unit_name)
                }) {
                    info!(
                        "Force selection to index {index:?} to select unit {selected_unit_name:?}"
                    );
                    single_selection.select_item(index, true);
                    //unit_list.set_force_to_select(index);
                    force_selected_index = index;
                }
            }

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
                let (sender, mut receiver) = tokio::sync::mpsc::channel(100);
                {
                    let all_units = all_units.clone();

                    runtime().spawn(async move {
                        const BATCH_SIZE: usize = 5;
                        let mut batch = Vec::with_capacity(BATCH_SIZE);
                        for (idx, unit) in (1..).zip(all_units.values()) {
                            batch.push((unit.primary(), unit.dbus_level(), unit.object_path()));

                            if idx % BATCH_SIZE == 0 {
                                call_complete_unit(&sender, &batch).await;

                                batch.clear();
                            }
                        }

                        call_complete_unit(&sender, &batch).await;
                    });
                }

                while let Some(updates) = receiver.recv().await {
                    for update in updates {
                        let Some(unit) = all_units.get(&update.primary) else {
                            continue;
                        };

                        unit.update_from_unit_info(update);
                    }
                }
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
            self.list_store.n_items(),
            self.filter_list_model.n_items(),
            self.unit_list_sort_list_model.n_items()
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

        if let Some(unit2) = self.unit_map.borrow().get(&UnitKey::new(unit)) {
            self.unit.replace(Some(unit2.clone()));
        } else {
            self.add_one_unit(unit);
        }

        //Don't select and focus if filter out
        if let Some(filter) = self.filter_list_model.filter() {
            let unit_binding = UnitBinding::new(unit);
            if !filter.match_(&unit_binding) {
                //Unselect
                self.single_selection
                    .set_selected(gtk::INVALID_LIST_POSITION);
                info!("Unit {unit_name} no Match");
                return Some(unit.clone());
            }
        }

        let finding = self.list_store.find_with_equal_func(|object| {
            let Some(unit_item) = object.downcast_ref::<UnitBinding>() else {
                error!("item.downcast_ref::<UnitBinding>()");
                return false;
            };

            unit_name == unit_item.primary()
        });

        if let Some(row) = finding {
            info!("Scroll to row {row}");

            self.units_browser.scroll_to(
                row, // to centerish on the selected unit
                None,
                gtk::ListScrollFlags::FOCUS | gtk::ListScrollFlags::SELECT,
                None,
            );
        }

        Some(unit.clone())
    }

    fn add_one_unit(&self, unit: &UnitInfo) {
        self.list_store.append(&UnitBinding::new(unit));
        let mut unit_map = self.unit_map.borrow_mut();
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
        let sorter = self.units_browser.sorter();

        self.unit_list_sort_list_model.set_sorter(sorter.as_ref());

        let item_out = self
            .units_browser
            .columns()
            .item(0)
            .expect("Expect item x to be not None");

        //Sort on first column
        let c1 = item_out
            .downcast_ref::<gtk::ColumnViewColumn>()
            .expect("item.downcast_ref::<gtk::ColumnViewColumn>()");

        self.units_browser
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
            if let Some(filter) = self.filter_list_model.filter() {
                filter.changed(change_type);
            } else {
                let custom_filter = self.create_custom_filter();
                self.filter_list_model.set_filter(Some(&custom_filter));
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

        self.filter_list_model.set_filter(None::<&gtk::Filter>); //FIXME this workaround prevent core dump

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
            let unit = if let Some(unit_binding) = object.downcast_ref::<UnitBinding>() {
                unit_binding.unit_ref()
            } else {
                error!("some wrong downcast_ref to UnitBinding  {object:?}");
                return false;
            };

            for asserror in applied_assessors.borrow().iter() {
                if !asserror.filter_unit(&unit) {
                    return false;
                }
            }
            true
        })
    }

    pub(super) fn set_new_columns(&self, list: Vec<UnitProperty>) {
        for prop in &list {
            let factory = column_factories::get_custom_factoy(prop);
            let column = gtk::ColumnViewColumn::new(Some(&prop.name), Some(factory));
            self.units_browser.append_column(&column);
        }

        for prop in list {}

        let list_store = self.list_store.clone();
        glib::spawn_future_local(async move {
            //let (sender, receiver) = tokio::sync::oneshot::channel();
            // let (sender, mut receiver) = tokio::sync::mpsc::channel(100);
            {
                for position in 0..list_store.n_items() {
                    let Some(obj) = list_store.item(position).and_downcast_ref::<UnitBinding>()
                    else {
                        warn!("No item found a position {position}");
                        continue;
                    };
                }

                runtime().spawn(async move {
                    /*   const BATCH_SIZE: usize = 5;
                    let mut batch = Vec::with_capacity(BATCH_SIZE);
                    for (idx, unit) in (1..).zip(all_units.values()) {
                        batch.push((unit.primary(), unit.dbus_level(), unit.object_path()));

                        if idx % BATCH_SIZE == 0 {
                            call_complete_unit(&sender, &batch).await;

                            batch.clear();
                        }
                    }

                    call_complete_unit(&sender, &batch).await; */
                });
            }
            /*
            while let Some(updates) = receiver.recv().await {
                 for update in updates {
                    let Some(unit) = all_units.get(&update.primary) else {
                        continue;
                    };

                    unit.update_from_unit_info(update);
                } */
        });
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
        let column_view_column_map = self.generate_column_map();
        column_factories::setup_factories(&unit_list, &column_view_column_map);

        settings.connect_changed(
            Some(KEY_PREF_UNIT_LIST_DISPLAY_COLORS),
            move |_settings, _key| {
                let display_color = unit_list.display_color();
                info!("change preference setting \"display color\" to {display_color}");
                let column_view_column_map = unit_list.imp().generate_column_map();
                column_factories::setup_factories(&unit_list, &column_view_column_map);
            },
        );

        column_view_column_set_sorter!(column_view_column_map, "unit", primary, dbus_level);
        column_view_column_set_sorter!(column_view_column_map, "type", unit_type);
        column_view_column_set_sorter!(column_view_column_map, "bus", unit_type);
        column_view_column_set_sorter!(column_view_column_map, "state", enable_status);
        column_view_column_set_sorter!(column_view_column_map, "preset", preset);
        column_view_column_set_sorter!(column_view_column_map, "load", load_state);
        column_view_column_set_sorter!(column_view_column_map, "active", active_state);
        column_view_column_set_sorter!(column_view_column_map, "sub", sub_state);
        column_view_column_set_sorter!(column_view_column_map, "description", description);

        let mut filter_assessors: HashMap<u8, Rc<RefCell<Box<dyn UnitPropertyFilter>>>> =
            HashMap::with_capacity(UNIT_LIST_COLUMNS.len());

        let unit_list_panel: glib::BorrowedObject<'_, crate::widget::unit_list::UnitListPanel> =
            self.obj();
        for (_, key, num_id, _) in &*UNIT_LIST_COLUMNS {
            let filter: Option<Box<dyn UnitPropertyFilter>> = match *key {
                "unit" => Some(Box::new(FilterText::new(
                    *num_id,
                    filter_unit_name,
                    &unit_list_panel,
                ))),
                "bus" => Some(Box::new(FilterElement::new(
                    *num_id,
                    filter_bus_level,
                    &unit_list_panel,
                ))),
                "type" => Some(Box::new(FilterElement::new(
                    *num_id,
                    filter_unit_type,
                    &unit_list_panel,
                ))),
                "state" => Some(Box::new(FilterElement::new(
                    *num_id,
                    filter_enable_status,
                    &unit_list_panel,
                ))),
                "preset" => Some(Box::new(FilterElement::new(
                    *num_id,
                    filter_preset,
                    &unit_list_panel,
                ))),
                "load" => Some(Box::new(FilterElement::new(
                    *num_id,
                    filter_load_state,
                    &unit_list_panel,
                ))),
                "active" => Some(Box::new(FilterElement::new(
                    *num_id,
                    filter_active_state,
                    &unit_list_panel,
                ))),
                "sub" => Some(Box::new(FilterElement::new(
                    *num_id,
                    filter_sub_state,
                    &unit_list_panel,
                ))),
                "description" => Some(Box::new(FilterText::new(
                    *num_id,
                    filter_unit_description,
                    &unit_list_panel,
                ))),
                _ => None,
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
        self.filter_list_model.set_filter(Some(&custom_filter));

        self.filter_list_model
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
            let units_browser = self.units_browser.clone();
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

        self.units_browser
            .connect_activate(|_a, row| info!("Unit row position {row}")); //TODO make selection

        pop_menu::setup_popup_menu(&self.units_browser, &self.filter_list_model, &self.obj());
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
    batch: &Vec<(String, systemd::enums::UnitDBusLevel, Option<String>)>,
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
