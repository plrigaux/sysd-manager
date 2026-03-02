mod column_factories;
#[macro_use]
mod construct;
pub mod pop_menu;

use std::{
    borrow::Cow,
    cell::{Cell, OnceCell, Ref, RefCell, RefMut},
    collections::HashMap,
    hash::Hasher,
    rc::Rc,
    sync::OnceLock,
    time::Duration,
};

use crate::{
    consts::{
        ACTION_INCLUDE_UNIT_FILES, ACTION_UNIT_LIST_FILTER, ACTION_UNIT_LIST_FILTER_CLEAR,
        ALL_FILTER_KEY, COL_ACTIVE, FILTER_MARK, PATH_PATH_COL, SYSD_SOCKET_LISTEN,
    },
    systemd::{
        data::UnitInfo,
        enums::{LoadState, UnitType},
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
            COL_ID_UNIT, CustomPropertyId, UnitCuratedList, UnitListPanel,
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
            imp::construct::construct_column_view,
            search_controls::UnitListSearchControls,
        },
        unit_properties_selector::{
            data_selection::UnitPropertySelection,
            save::{self},
        },
    },
};
use base::enums::UnitDBusLevel;
use flagset::FlagSet;
use glib::WeakRef;
use gtk::{
    Adjustment, TemplateChild,
    gio::{self, glib::VariantTy},
    glib::{self, Properties},
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
use std::hash::Hash;
use systemd::{
    ListUnitResponse, UnitProperties, UnitPropertiesFlags, data::UnitPropertySetter,
    enums::UnitFileStatus, socket_unit::SocketUnitInfo,
};
use tokio::task::AbortHandle;
use tracing::{debug, error, info, warn};

static SOCKET_LISTEN_QUARK: OnceLock<glib::Quark> = OnceLock::new();
static PATH_PATHS_QUARK: OnceLock<glib::Quark> = OnceLock::new();

const UNIT_LIST_VIEW_PAGE: &str = "unit_list";
const RESTRICTIVE_FILTER_VIEW_PAGE: &str = "restrictive_filter";

#[derive(Debug, Clone)]
struct UnitKey {
    level: UnitDBusLevel,
    primary: String,
    update_properties: Cell<UnitProperties>,
}

impl UnitKey {
    fn new(unit: &UnitInfo) -> Self {
        Self::new_string(unit.dbus_level(), unit.primary())
    }

    fn new_string(level: UnitDBusLevel, primary: String) -> Self {
        let f = FlagSet::<UnitPropertiesFlags>::empty();
        Self::new_string_flags(level, primary, f)
    }

    fn new_string_flags(
        level: UnitDBusLevel,
        primary: String,
        flags: impl Into<FlagSet<UnitPropertiesFlags>>,
    ) -> Self {
        UnitKey {
            level,
            primary,
            update_properties: Cell::new(UnitProperties(flags.into())),
        }
    }

    fn intersec(&self, flags: impl Into<FlagSet<UnitPropertiesFlags>>) {
        let f = self.update_properties.get().0 & flags;
        self.update_properties.set(UnitProperties(f));
    }
}

impl PartialEq for UnitKey {
    fn eq(&self, other: &UnitKey) -> bool {
        self.level == other.level && self.primary == other.primary
    }
}

impl Eq for UnitKey {}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct UnitKeyRef<'a> {
    level: UnitDBusLevel,
    primary: &'a str,
}

trait UnitKeyInterface {
    fn as_key_ref(&self) -> UnitKeyRef<'_>;
}

impl UnitKeyInterface for UnitKey {
    fn as_key_ref(&self) -> UnitKeyRef<'_> {
        UnitKeyRef {
            level: self.level,
            primary: self.primary.as_str(),
        }
    }
}

impl<'a> UnitKeyInterface for UnitKeyRef<'a> {
    fn as_key_ref(&self) -> UnitKeyRef<'_> {
        *self
    }
}

impl<'a> PartialEq for dyn UnitKeyInterface + 'a {
    fn eq(&self, other: &Self) -> bool {
        self.as_key_ref() == other.as_key_ref()
    }
}

impl<'a> Eq for dyn UnitKeyInterface + 'a {}

impl<'a> Hash for dyn UnitKeyInterface + 'a {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_key_ref().hash(state);
    }
}

impl Hash for UnitKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_key_ref().hash(state);
    }
}

impl<'a> std::borrow::Borrow<dyn UnitKeyInterface + 'a> for UnitKey {
    fn borrow(&self) -> &(dyn UnitKeyInterface + 'a) {
        self
    }
}

// impl<'a> AsKeyRef for UnitKeyRef<'a> {
//     fn as_key_ref(&self) -> KeyRef<'_> {
//         match self {
//             &Key::String(ref s) => KeyRef::String(s.as_str()),
//             &Key::Bytes(ref b) => KeyRef::Bytes(&*b),
//         }
//     }
// }
impl<'a> UnitKeyRef<'a> {
    fn new(level: UnitDBusLevel, primary: &'a str) -> Self {
        UnitKeyRef { level, primary }
    }

    fn key_owned(self, flags: impl Into<FlagSet<UnitPropertiesFlags>>) -> UnitKey {
        UnitKey::new_string_flags(self.level, self.primary.to_owned(), flags)
    }
}

impl<'a> PartialEq<UnitKey> for UnitKeyRef<'a> {
    fn eq(&self, other: &UnitKey) -> bool {
        self.level == other.level && self.primary == other.primary
    }
}

impl<'a> PartialEq<UnitKeyRef<'a>> for UnitKey {
    fn eq(&self, other: &UnitKeyRef<'a>) -> bool {
        self.level == other.level && self.primary == other.primary
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
    unit_files_count_label: TemplateChild<gtk::Label>,

    #[template_child]
    loaded_units_count_label: TemplateChild<gtk::Label>,

    #[template_child]
    unit_filtered_count_label: TemplateChild<gtk::Label>,

    #[property(get, set=Self::set_unit_files_count)]
    unit_files_count: Cell<u32>,

    #[property(get, set=Self::set_loaded_units_count)]
    loaded_units_count: Cell<u32>,

    #[property(get, set=Self::set_unit_filter_count)]
    unit_filtered_count: Cell<u32>,

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
    selected_list_view: Cell<UnitCuratedList>,

    #[property(get, set)]
    include_unit_files: Cell<bool>,

    abort_handles: RefCell<Vec<AbortHandle>>,
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
                    unit_list_panel.imp().fill_store(None);
                })
                .build()
        };

        app_window.add_action_entries([
            action_entry,
            list_filter_action_entry,
            list_filter_action_entry_blank,
            list_filter_clear_action_entry,
            refresh_unit_list,
        ]);

        let settings = systemd_gui::new_settings();

        let action = settings.create_action(UnitCuratedList::base_action());
        app_window.add_action(&action);

        let unit_list_panel = self.obj().clone();
        let unit_list_panel2 = self.obj().clone();
        settings
            .bind::<UnitListPanel>(
                UnitCuratedList::base_action(),
                &self.obj(),
                "selected-list-view",
            )
            .mapping(move |variant, _| {
                let unit_list_view: UnitCuratedList = variant.into();
                unit_list_panel.imp().fill_store(Some(unit_list_view));
                Some(unit_list_view.to_value())
            })
            .set_mapping(move |value, _| {
                let unit_list_view = value
                    .get::<UnitCuratedList>()
                    .inspect_err(|err| warn!("Conv error {:?}", err))
                    .unwrap_or(UnitCuratedList::Defaut);
                unit_list_panel2.imp().fill_store(Some(unit_list_view));
                Some(unit_list_view.id().to_variant())
            })
            .build();

        let action = settings.create_action(ACTION_INCLUDE_UNIT_FILES);
        app_window.add_action(&action);

        settings
            .bind::<UnitListPanel>(
                ACTION_INCLUDE_UNIT_FILES,
                &self.obj(),
                ACTION_INCLUDE_UNIT_FILES,
            )
            .build();
    }

    fn generate_column_list(&self) -> Vec<gtk::ColumnViewColumn> {
        let list_model: gio::ListModel = units_browser!(self).columns();

        let mut col_list = Vec::with_capacity(list_model.n_items() as usize);

        for column_view_column in list_model
            .iter::<gtk::ColumnViewColumn>()
            .filter_map(|item| {
                item.inspect_err(|err| error!("Expect gtk::ColumnViewColumn> {err:?}"))
                    .ok()
            })
        {
            col_list.push(column_view_column);
        }
        col_list
    }

    fn fill_store(&self, new_view: Option<UnitCuratedList>) {
        if let Some(new_view) = new_view {
            self.save_config();

            self.obj().set_selected_list_view(new_view);
        }

        let view = self.selected_list_view.get();

        debug!("fill store {:?}", view);

        let cols = construct_column_view(self.display_color.get(), view);
        self.set_new_columns(cols, false);

        self.fill_browser();
    }

    fn fill_browser(&self) {
        let list_store = self.list_store.get().expect("LIST STORE NOT NONE").clone();
        let main_unit_map_rc: Rc<RefCell<HashMap<UnitKey, UnitInfo>>> = self.units_map.clone();
        let panel_stack = self.panel_stack.clone();
        let single_selection = single_selection!(self).clone();
        let unit_list = self.obj().clone();
        let units_browser = units_browser!(self).clone();
        let view = self.selected_list_view.get();
        let dbus_level = PREFERENCES.dbus_level();
        let refresh_unit_list_button = upgrade!(self.refresh_unit_list_button);

        //Rem sorting before adding lot of items for performance reasons
        self.unit_list_sort_list_model
            .borrow()
            .set_sorter(None::<&gtk::Sorter>);

        self.aborts_handles();

        glib::spawn_future_local(async move {
            refresh_unit_list_button.set_sensitive(false);
            panel_stack.set_visible_child_name("spinner");

            let Ok(retrieved_units) = retrieve_unit_list(dbus_level, view, &unit_list).await else {
                panel_stack.set_visible_child_name("error");
                return;
            };

            let (loaded_count, file_count) = retrieved_units.iter().fold((0, 0), |a, b| {
                let l = b.r_len();
                (a.0 + l.0, a.1 + l.1)
            });

            unit_list.set_loaded_units_count(loaded_count as u32);
            unit_list.set_unit_files_count(file_count as u32);

            let n_items = list_store.n_items();
            list_store.remove_all();

            let total = retrieved_units.iter().fold(0, |acc, i| acc + i.t_len());

            #[allow(clippy::mutable_key_type)]
            let mut all_units: HashMap<UnitKey, UnitInfo> = HashMap::with_capacity(total);

            for (system_unit_file, flags) in retrieved_units.into_iter().map(|s| {
                let f = s.update_flags();
                (s, f)
            }) {
                match system_unit_file {
                    ListUnitResponse::Loaded(level, lunits) => {
                        for loaded_unit in lunits.into_iter() {
                            let key = UnitKeyRef::new(level, &loaded_unit.primary_unit_name);
                            if let Some((key, unit)) =
                                all_units.get_key_value(&key as &dyn UnitKeyInterface)
                            {
                                key.intersec(flags);
                                unit.update_from_loaded_unit(loaded_unit);
                            } else {
                                let key = key.key_owned(flags);
                                let unit = UnitInfo::from_listed_unit(loaded_unit, level);

                                list_store.append(&unit);
                                all_units.insert(key, unit);
                            }
                        }
                    }
                    ListUnitResponse::File(level, items) => {
                        for unit_file in items {
                            let key = UnitKeyRef::new(level, unit_file.unit_primary_name());
                            if let Some((key, unit)) =
                                all_units.get_key_value(&key as &dyn UnitKeyInterface)
                            {
                                key.intersec(flags);
                                unit.update_from_unit_file(unit_file);
                            } else {
                                let key = key.key_owned(flags);
                                let unit = UnitInfo::from_unit_file(unit_file, level);
                                list_store.append(&unit);
                                all_units.insert(key, unit);
                            }
                        }
                    }
                };
            }
            main_unit_map_rc.replace(all_units);

            // The sort function needs to be the same of the  first column sorter
            let sort_func = column_sorter_lambda!(primary, dbus_level);

            list_store.sort(sort_func);

            info!("Unit list refreshed! list size {}", list_store.n_items());

            let mut force_selected_index = gtk::INVALID_LIST_POSITION;

            //Handle unit selection
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

            //Set selection
            unit_list
                .imp()
                .force_selected_index
                .set(Some(force_selected_index));
            refresh_unit_list_button.set_sensitive(true);
            // unit_list.imp().set_sorter();

            //cause no scrollwindow v adjustment
            if n_items > 0 {
                focus_on_row(&unit_list, &units_browser);
            }
            panel_stack.set_visible_child_name(UNIT_LIST_VIEW_PAGE);

            unit_list.imp().fetch_custom_unit_properties();
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

        if LoadState::Loaded == unit.load_state() {
            let count = self.obj().loaded_units_count();
            self.obj().set_loaded_units_count(count + 1)
        }

        if unit.enable_status() != UnitFileStatus::Unknown {
            let count = self.obj().unit_files_count();
            self.obj().set_unit_files_count(count + 1);
        }
    }

    pub fn selected_unit(&self) -> Option<UnitInfo> {
        self.unit.borrow().clone()
    }

    pub fn set_inter_message(&self, _action: &InterPanelMessage) {}

    fn set_sorter(&self) {
        let units_browser = units_browser!(self);
        let sorter = units_browser.sorter();

        self.unit_list_sort_list_model
            .borrow()
            .set_sorter(sorter.as_ref());

        let col_def_list = self.current_column_view_column_definition_list.borrow();

        if let Some((idx, col_def)) = col_def_list
            .iter()
            .enumerate()
            .find(|(_, col_def)| col_def.sort() != save::SortType::Unset)
        {
            let first_col = units_browser.columns().item(idx as u32);

            //Sort on first column
            let idx_column = first_col.and_downcast_ref::<gtk::ColumnViewColumn>();
            if let Some(sort_type) = col_def.sort().into() {
                units_browser.sort_by_column(idx_column, sort_type);
            }
        } else {
            let first_col = units_browser.columns().item(0);

            //Sort on first column
            let first_column = first_col.and_downcast_ref::<gtk::ColumnViewColumn>();

            units_browser.sort_by_column(first_column, gtk::SortType::Ascending);
        }
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
            COL_ACTIVE => Some(Box::new(FilterElement::new(
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

    pub(super) fn set_new_columns(
        &self,
        property_list: Vec<UnitPropertySelection>,
        fetch_custom_props: bool,
    ) {
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

        if fetch_custom_props {
            self.fetch_custom_unit_properties();
        }
    }

    fn fetch_custom_unit_properties(&self) {
        let current_property_list = self.current_column_view_column_definition_list.borrow();

        if current_property_list.is_empty() {
            info!("No extra properties to fetch");
            return;
        }

        info!("!!! Fetching custom unit properties !!!");
        let current_property_list = current_property_list.clone();

        let mut property_list_send = HashMap::with_capacity(current_property_list.len());

        for unit_property_selection in current_property_list.iter() {
            //Add custom factory
            unit_property_selection.fill_property_fetcher(&mut property_list_send)
        }

        let units_browser = units_browser!(self).clone();
        let units_map = self.units_map.clone();
        let display_color = self.display_color.get();
        let unit_list = self.obj().clone();
        let list_store = self.list_store.get().unwrap().clone();

        glib::spawn_future_local(async move {
            let units_list: Vec<_> = units_map
                .borrow()
                .iter()
                // .filter(|unit| is_unit_type || types.contains(&unit.unit_type()))
                .map(|(key, unit)| {
                    (
                        unit.dbus_level(),
                        unit.primary(),
                        unit.object_path(),
                        unit.unit_type(),
                        key.update_properties.get(),
                    )
                })
                .collect();

            let (sender, mut receiver) = tokio::sync::mpsc::channel(100);
            let handle = systemd::runtime().spawn(async move {
                info!("Fetching properties START for {} units", units_list.len());
                for (level, primary_name, object_path, unit_type, update_property_flag) in
                    units_list.into_iter()
                {
                    let mut cleaned_props: Vec<_> = Vec::with_capacity(property_list_send.len());
                    for (unit_type, quark) in property_list_send.iter().filter(|(item, _)| {
                        item.unit_type == UnitType::Unit || item.unit_type == unit_type
                    }) {
                        cleaned_props.push((unit_type.unit_type, &unit_type.property, *quark));
                    }
                    // println!("orig {:?}", property_list_send);
                    // println!("cleaned {:?}", cleaned_props);

                    let properties_setter = systemd::fetch_unit_properties(
                        level,
                        &primary_name,
                        &object_path,
                        update_property_flag,
                        cleaned_props,
                    )
                    .await
                    .inspect_err(|err| debug!("Some Error : {err:?}"))
                    .unwrap_or(vec![]);

                    let result = sender
                        .send((UnitKey::new_string(level, primary_name), properties_setter))
                        .await;
                    if let Err(err) = result {
                        error!("The channel needs to be open. {err:?}");
                        break;
                    }
                }
            });

            unit_list.imp().add_tokio_handle(handle);

            info!("Fetching properties WAIT");
            while let Some((key, property_value_list)) = receiver.recv().await {
                // info!("Got {} properties for {:?}", property_value_list.len(), key);
                let map_ref = units_map.borrow();
                let Some(unit) = map_ref.get(&key) else {
                    warn!("UnitKey not Found: {key:?}");
                    continue;
                };

                for setter in property_value_list {
                    match setter {
                        UnitPropertySetter::FileState(unit_file_status) => {
                            unit.set_enable_status(unit_file_status)
                        }
                        UnitPropertySetter::Description(description) => {
                            unit.set_description(description)
                        }
                        UnitPropertySetter::ActiveState(active_state) => {
                            unit.set_active_state(active_state)
                        }
                        UnitPropertySetter::LoadState(load_state) => {
                            unit.set_load_state(load_state)
                        }
                        UnitPropertySetter::FragmentPath(_) => todo!(),
                        UnitPropertySetter::UnitFilePreset(preset) => unit.set_preset(preset),
                        UnitPropertySetter::SubState(substate) => unit.set_sub_state(substate),
                        UnitPropertySetter::Custom(quark, owned_value) => {
                            // println!("DEBUG Custom prop {:?} {:?}", quark, owned_value);
                            if &quark
                                == SOCKET_LISTEN_QUARK
                                    .get_or_init(|| glib::Quark::from_str(SYSD_SOCKET_LISTEN))
                            {
                                let listens = unit.insert_socket_listen(quark, owned_value);
                                for idx in 1..listens {
                                    //Frankeinstein
                                    let usocket = SocketUnitInfo::from_unit_socket(unit, idx);
                                    list_store.append(&usocket);
                                }
                            } else if &quark
                                == PATH_PATHS_QUARK
                                    .get_or_init(|| glib::Quark::from_str(PATH_PATH_COL))
                            {
                                debug!("Custom value {:?}", owned_value);
                                let _listens = unit.insert_socket_listen(quark, owned_value);
                            } else {
                                unit.insert_unit_property_value(quark, owned_value)
                            }
                        }
                    }
                }
            }

            info!("Fetching properties FINISHED");

            //Force the factory to display data by setting the factory after the value set (no data binding)
            for column in units_browser
                .columns()
                .iter::<gtk::ColumnViewColumn>()
                .filter_map(|item| item.ok())
            {
                let prop_type = current_property_list
                    .iter()
                    .find(|prop_selection| prop_selection.id() == column.id())
                    .and_then(|prop_selection| prop_selection.prop_type());

                construct::set_column_factory_and_sorter(
                    &column,
                    display_color,
                    prop_type.as_deref(),
                );
            }

            unit_list.imp().set_sorter();
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

    fn add_tokio_handle(&self, handle: tokio::task::JoinHandle<()>) {
        let ah = handle.abort_handle();
        self.abort_handles.borrow_mut().push(ah);
    }

    fn aborts_handles(&self) {
        for ah in self.abort_handles.borrow().iter() {
            ah.abort();
        }
    }

    fn set_loaded_units_count(&self, count: u32) {
        let label = if count == 0 {
            Cow::from("")
        } else {
            Cow::from(count.to_string())
        };
        self.loaded_units_count_label.set_label(&label);
        self.loaded_units_count.set(count);
    }

    fn set_unit_files_count(&self, count: u32) {
        let label = if count == 0 {
            Cow::from("")
        } else {
            Cow::from(count.to_string())
        };
        self.unit_files_count_label.set_label(&label);
        self.unit_files_count.set(count);
    }

    fn set_unit_filter_count(&self, count: u32) {
        let label = if count == 0 {
            Cow::from("")
        } else {
            Cow::from(count.to_string())
        };
        self.unit_filtered_count_label.set_label(&label);
        self.unit_filtered_count.set(count);

        if count == 0 && (self.unit_files_count.get() + self.loaded_units_count.get()) != 0 {
            self.panel_stack
                .set_visible_child_name(RESTRICTIVE_FILTER_VIEW_PAGE);
        } else if self.panel_stack.visible_child_name().as_deref()
            == Some(RESTRICTIVE_FILTER_VIEW_PAGE)
        {
            self.panel_stack.set_visible_child_name(UNIT_LIST_VIEW_PAGE);
        }
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
        self.units_browser.get_or_init(|| column_view);
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
            .bind_property::<UnitListPanel>("n-items", &self.obj(), "unit_filtered_count")
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

macro_rules! dbus_call {
    ($int_level:expr, $handles:expr, $module:ident :: $f:ident) => {{
        if matches!($int_level, DbusLevel::System | DbusLevel::SystemAndSession) {
            $handles.push(tokio::spawn($module::$f(UnitDBusLevel::System)));
        }

        if matches!(
            $int_level,
            DbusLevel::UserSession | DbusLevel::SystemAndSession
        ) {
            $handles.push(tokio::spawn($module::$f(UnitDBusLevel::UserSession)));
        }
    }};
}

async fn retrieve_unit_list(
    int_level: DbusLevel,
    view: UnitCuratedList,
    unit_list: &UnitListPanel,
) -> Result<Vec<ListUnitResponse>, bool> {
    let (sender_syst, receiver_syst) = tokio::sync::oneshot::channel();
    let handle = systemd::runtime().spawn(async move {
        let mut handles = Vec::with_capacity(4);

        match view {
            UnitCuratedList::Defaut | UnitCuratedList::Custom | UnitCuratedList::LoadedUnit => {
                dbus_call!(int_level, handles, systemd::list_loaded_units)
            }
            UnitCuratedList::UnitFiles => {}
            UnitCuratedList::Timers => {
                dbus_call!(int_level, handles, systemd::list_loaded_units_timers)
            }
            UnitCuratedList::Sockets => {
                dbus_call!(int_level, handles, systemd::list_loaded_units_sockets)
            }
            UnitCuratedList::Path => {
                dbus_call!(int_level, handles, systemd::list_loaded_units_paths)
            }
            UnitCuratedList::Automount => {
                dbus_call!(int_level, handles, systemd::list_loaded_units_automounts)
            }
        }

        if matches!(
            view,
            UnitCuratedList::Defaut | UnitCuratedList::Custom | UnitCuratedList::UnitFiles
        ) {
            dbus_call!(int_level, handles, systemd::list_unit_files);
        }

        let mut results = Vec::with_capacity(handles.len());
        let mut error = false;
        for handle in handles {
            let Ok(r) = handle
                .await
                .inspect_err(|err| warn!("Unit List Join Error: {err:?}"))
            else {
                error = true;
                continue;
            };

            let Ok(x) = r.inspect_err(|err| warn!("Unit List Call Error: {err:?}")) else {
                error = true;
                continue;
            };
            results.push(x);
        }

        let result = if error { Err(error) } else { Ok(results) };
        sender_syst
            .send(result)
            .expect("The channel needs to be open.");
    });

    unit_list.imp().add_tokio_handle(handle);

    receiver_syst.await.unwrap_or_else(|err| {
        error!("Tokio receiver works, {err:?}");
        Err(true)
    })
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
