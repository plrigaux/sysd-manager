use std::{
    cell::{Cell, OnceCell, RefCell, RefMut},
    rc::Rc,
};

use gtk::{
    ffi::GTK_INVALID_LIST_POSITION,
    gio::{self},
    glib::{self, BoxedAnyObject, Object},
    pango::{AttrColor, AttrInt, AttrList, Weight},
    prelude::*,
    subclass::{
        box_::BoxImpl,
        prelude::*,
        widget::{
            CompositeTemplateCallbacksClass, CompositeTemplateClass,
            CompositeTemplateInitializingExt, WidgetImpl,
        },
    },
    ListScrollFlags, SearchBar, TemplateChild,
};

use log::{debug, error, info, warn};

use crate::{
    icon_name,
    systemd::runtime,
    utils::palette::{grey, red, yellow, Palette},
    widget::InterPanelAction,
};
use crate::{
    systemd::{
        self,
        data::UnitInfo,
        enums::{ActiveState, EnablementStatus, UnitType},
    },
    widget::{
        app_window::AppWindow,
        menu_button::{ExMenuButton, OnClose},
    },
};
use strum::IntoEnumIterator;

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/unit_list_panel.ui")]
pub struct UnitListPanelImp {
    #[template_child]
    list_store: TemplateChild<gio::ListStore>,

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
    panel_stack: TemplateChild<gtk::Stack>,

    #[template_child]
    scrolled_window: TemplateChild<gtk::ScrolledWindow>,

    search_entry: OnceCell<gtk::SearchEntry>,

    refresh_unit_list_button: OnceCell<gtk::Button>,

    unit: RefCell<Option<UnitInfo>>,

    pub force_selected_index: Cell<Option<u32>>,

    highlight_yellow: RefCell<AttrList>,
    highlight_red: RefCell<AttrList>,
    grey: RefCell<AttrList>,
}

macro_rules! factory_setup {
    ($item_obj:expr) => {{
        let item = $item_obj
            .downcast_ref::<gtk::ListItem>()
            .expect("item.downcast_ref::<gtk::ListItem>()");
        let row = gtk::Inscription::builder().xalign(0.0).build();
        item.set_child(Some(&row));
    }};
}

macro_rules! downcast_list_item {
    ($item_obj:expr) => {{
        let item = $item_obj
            .downcast_ref::<gtk::ListItem>()
            .expect("item.downcast_ref::<gtk::ListItem>()");
        item
    }};
}

macro_rules! factory_bind_pre {
    ($item_obj:expr) => {{
        let item = downcast_list_item!($item_obj);
        let child = item.child().and_downcast::<gtk::Inscription>().unwrap();
        let unit = item.item().and_downcast::<UnitInfo>().unwrap();
        (child, unit)
    }};
}

macro_rules! factory_bind {
    ($item_obj:expr, $func:ident) => {{
        let (child, unit) = factory_bind_pre!($item_obj);
        let v = unit.$func();
        child.set_text(Some(&v));
        (child, unit)
    }};
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
                .downcast_ref::<UnitInfo>()
                .expect("Needs to be UnitInfo");
            let unit2 = obj2
                .downcast_ref::<UnitInfo>()
                .expect("Needs to be UnitInfo");

            compare_units!(unit1, unit2, $($func),+)
        })
    }};
}

macro_rules! column_view_column_set_sorter {
    ($list_item:expr, $col_idx:expr, $($func:ident),+) => {
        let item_out = $list_item
            .item($col_idx)
            .expect("Expect item x to be not None");

        let column_view_column = item_out
            .downcast_ref::<gtk::ColumnViewColumn>()
            .expect("item.downcast_ref::<gtk::ColumnViewColumn>()");

        let sorter = create_column_filter!($($func),+);
        column_view_column.set_sorter(Some(&sorter));
    };
}

#[gtk::template_callbacks]
impl UnitListPanelImp {
    #[template_callback]
    fn col_unit_name_factory_setup(_fac: &gtk::SignalListItemFactory, item_obj: &Object) {
        factory_setup!(item_obj);
    }

    #[template_callback]
    fn col_unit_name_factory_bind(&self, item_obj: &Object, _fac: &gtk::SignalListItemFactory) {
        factory_bind!(item_obj, display_name);

        let (child, unit) = factory_bind_pre!(item_obj);
        let v = unit.display_name();
        child.set_text(Some(&v));

        let bus = match unit.dbus_level() {
            systemd::enums::UnitDBusLevel::System => "on system bus",
            systemd::enums::UnitDBusLevel::UserSession => "on user bus",
        };

        child.set_tooltip_text(Some(bus));

        if unit.active_state().is_inactive() {
            let attribute_list = self.grey.borrow();
            child.set_attributes(Some(&attribute_list));
        } else {
            child.set_attributes(None);
        }
    }

    #[template_callback]
    fn col_type_factory_setup(_fac: &gtk::SignalListItemFactory, item_obj: &Object) {
        factory_setup!(item_obj);
    }

    #[template_callback]
    fn col_type_factory_bind(&self, item_obj: &Object, _fac: &gtk::SignalListItemFactory) {
        let (child, unit) = factory_bind!(item_obj, unit_type);

        if unit.active_state().is_inactive() {
            let attribute_list = self.grey.borrow();
            child.set_attributes(Some(&attribute_list));
        } else {
            child.set_attributes(None);
        }
    }

    #[template_callback]
    fn col_enable_status_factory_setup(_fac: &gtk::SignalListItemFactory, item_obj: &Object) {
        factory_setup!(item_obj);
    }

    #[template_callback]
    fn col_enable_status_factory_bind(_fac: &gtk::SignalListItemFactory, item_obj: &Object) {
        let (child, entry) = factory_bind_pre!(item_obj);

        let status_code: EnablementStatus = entry.enable_status().into();

        child.set_text(Some(status_code.as_str()));
        child.set_tooltip_markup(Some(status_code.tooltip_info()));

        entry
            .bind_property("enable_status", &child, "text")
            .transform_to(|_, status: u8| {
                let estatus: EnablementStatus = status.into();
                let str = estatus.to_string();
                Some(str)
            })
            .build();
    }

    #[template_callback]
    fn col_active_status_factory_setup(_fac: &gtk::SignalListItemFactory, item_obj: &Object) {
        let item = downcast_list_item!(item_obj);
        let image = gtk::Image::new();
        item.set_child(Some(&image));
    }

    #[template_callback]
    fn col_active_status_factory_bind(_fac: &gtk::SignalListItemFactory, item_obj: &Object) {
        let item = downcast_list_item!(item_obj);
        let child = item.child().and_downcast::<gtk::Image>().unwrap();
        let unit = item.item().and_downcast::<UnitInfo>().unwrap();
        let state = &unit.active_state();

        let icon_name = state.icon_name();
        child.set_icon_name(icon_name);
        child.set_tooltip_text(Some(state.as_str()));
        unit.bind_property("active_state_num", &child, "icon-name")
            .transform_to(|_, state: u8| {
                let state: ActiveState = state.into();
                icon_name!(state)
            })
            .build();
    }

    #[template_callback]
    fn col_load_factory_setup(_fac: &gtk::SignalListItemFactory, item_obj: &Object) {
        factory_setup!(item_obj);
    }

    #[template_callback]
    fn col_load_factory_bind(&self, item_obj: &Object, _factory: &gtk::SignalListItemFactory) {
        let (child, unit) = factory_bind_pre!(item_obj);

        let ls = unit.load_state();
        child.set_text(Some(&ls));
        unit.bind_property("load_state", &child, "text").build();

        if ls.starts_with('n')
        //"not-found"
        {
            let attribute_list = self.highlight_yellow.borrow();
            child.set_attributes(Some(&attribute_list));
        } else if ls.starts_with('b') || ls.starts_with('e') || ls.starts_with('m')
        // "bad-setting", "error", "masked"
        {
            let attribute_list = self.highlight_red.borrow();
            child.set_attributes(Some(&attribute_list));
        } else {
            child.set_attributes(None);
        }

        //let (child, unit) = factory_bind!(item_obj, load_state);
    }

    #[template_callback]
    fn col_sub_factory_setup(_fac: &gtk::SignalListItemFactory, item_obj: &Object) {
        factory_setup!(item_obj);
    }

    #[template_callback]
    fn col_sub_factory_bind(&self, item_obj: &Object, _fac: &gtk::SignalListItemFactory) {
        let (child, unit) = factory_bind!(item_obj, sub_state);
        unit.bind_property("sub_state", &child, "text").build();

        let state = &unit.active_state();
        if state.is_inactive() {
            let attribute_list = self.grey.borrow();
            child.set_attributes(Some(&attribute_list));
        } else {
            child.set_attributes(None);
        }
    }

    #[template_callback]
    fn col_description_factory_setup(_fac: &gtk::SignalListItemFactory, item_obj: &Object) {
        factory_setup!(item_obj);
    }

    #[template_callback]
    fn col_description_factory_bind(_fac: &gtk::SignalListItemFactory, item_obj: &Object) {
        let (child, unit) = factory_bind!(item_obj, description);
        unit.bind_property("description", &child, "text").build();
    }

    #[template_callback]
    fn sections_changed(&self, position: u32) {
        info!("sections_changed {position}");
    }
}

impl UnitListPanelImp {
    pub(super) fn register_selection_change(
        &self,
        app_window: &AppWindow,
        refresh_unit_list_button: &gtk::Button,
    ) {
        let app_window = app_window.clone();
        let list_widjet = self.obj().clone();

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

                let unit = match object.downcast::<UnitInfo>() {
                    Ok(unit) => unit,
                    Err(val) => {
                        error!("Object.downcast::<UnitInfo> Error: {:?}", val);
                        return;
                    }
                };

                info!("Selection changed, new unit {}", unit.primary());

                list_widjet.set_unit_internal(&unit);
                app_window.selection_change(Some(&unit));
            }); // FOR THE SEARCH

        self.refresh_unit_list_button
            .set(refresh_unit_list_button.clone())
            .expect("refresh_unit_list_button was already set!");

        self.fill_store();
    }

    pub fn search_bar(&self) -> gtk::SearchBar {
        self.search_bar.clone()
    }

    pub(super) fn fill_store(&self) {
        let list_store = self.list_store.clone();
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
                Err(err) => {
                    warn!("Fail fetch list {:?}", err);
                    panel_stack.set_visible_child_name("error");
                    return;
                }
            };

            let n_items = list_store.n_items();
            list_store.remove_all();

            for item in unit_desc.values() {
                list_store.append(item);
            }

            for item in unit_from_files.iter() {
                list_store.append(item);
            }

            // The sort function needs to be the same of the  first column sorter
            let sort_func = |o1: &Object, o2: &Object| {
                let u1 = o1.downcast_ref::<UnitInfo>().expect("Needs to be UnitInfo");
                let u2 = o2.downcast_ref::<UnitInfo>().expect("Needs to be UnitInfo");

                compare_units!(u1, u2, primary, dbus_level)
            };

            list_store.sort(sort_func);

            info!("Unit list refreshed! list size {}", list_store.n_items());

            let mut force_selected_index = GTK_INVALID_LIST_POSITION;

            let selected_unit = unit_list.selected_unit();
            if let Some(selected_unit) = selected_unit {
                let selected_unit_name = selected_unit.primary();

                debug!(
                    "LS items-n {} name {}",
                    list_store.n_items(),
                    selected_unit_name
                );

                if let Some(index) = list_store.find_with_equal_func(|object| {
                    let list_unit = object
                        .downcast_ref::<UnitInfo>()
                        .expect("Needs to be UnitInfo");

                    list_unit.primary().eq(&selected_unit_name)
                }) {
                    info!(
                        "Force selection to index {:?} to select unit {:?}",
                        index, selected_unit_name
                    );
                    single_selection.select_item(index, true);
                    //unit_list.set_force_to_select(index);
                    force_selected_index = index;
                }
            }

            unit_list.set_force_selected_index(Some(force_selected_index));
            refresh_unit_list_button.set_sensitive(true);

            //cause no scrollwindow v adjustment
            if n_items > 0 {
                focus_on_row(&unit_list, &units_browser);
            }
            panel_stack.set_visible_child_name("unit_list");

            glib::spawn_future_local(async move {
                runtime().spawn(async move {
                    let response = systemd::complete_unit_information(unit_from_files).await;
                    if let Err(error) = response {
                        warn!("Complete Unit Information Error: {:?}", error);
                    }
                });
            });
        });
    }

    pub(super) fn button_search_toggled(&self, toggle_button_is_active: bool) {
        self.search_bar.set_search_mode(toggle_button_is_active);

        if toggle_button_is_active {
            let se = self.search_entry.get().unwrap();

            se.grab_focus();
        }
    }

    pub fn set_unit_internal(&self, unit: &UnitInfo) {
        let _ = self.unit.replace(Some(unit.clone()));
    }

    pub fn set_unit(&self, unit: Option<&UnitInfo>) {
        let unit = match unit {
            Some(u) => u,
            None => {
                self.unit.replace(None);
                return;
            }
        };

        let old = self.unit.replace(Some(unit.clone()));
        if let Some(old) = old {
            if old.primary() == unit.primary() {
                info!("List {} == {}", old.primary(), unit.primary());
                return;
            }
        }

        let unit_name = unit.primary();

        info!(
            "Unit List {} list_store {} filter {} sort_model {}",
            unit_name,
            self.list_store.n_items(),
            self.filter_list_model.n_items(),
            self.unit_list_sort_list_model.n_items()
        );

        //Don't  select and focus if filter out
        if let Some(filter) = self.filter_list_model.filter() {
            if !filter.match_(unit) {
                //Unselect
                self.single_selection
                    .set_selected(GTK_INVALID_LIST_POSITION);
                info!("Unit {} no Match", unit_name);
                return;
            }
        }

        let finding = self.list_store.find_with_equal_func(|object| {
            let unit_item = object
                .downcast_ref::<UnitInfo>()
                .expect("item.downcast_ref::<gtk::ListItem>()");

            unit_name == unit_item.primary()
        });

        if let Some(row) = finding {
            info!("Scroll to row {}", row);

            self.units_browser.scroll_to(
                row, // to centerish on the selected unit
                None,
                ListScrollFlags::FOCUS | ListScrollFlags::SELECT,
                None,
            );
        }
    }

    pub fn selected_unit(&self) -> Option<UnitInfo> {
        self.unit.borrow().clone()
    }

    pub fn set_force_selected_index(&self, force_selected_index: Option<u32>) {
        self.force_selected_index.set(force_selected_index)
    }

    pub fn set_inter_action(&self, action: &InterPanelAction) {
        if let InterPanelAction::IsDark(is_dark) = *action {
            let attribute_list = highlight_attrlist(yellow(is_dark));

            self.highlight_yellow.replace(attribute_list);

            let attribute_list = highlight_attrlist(red(is_dark));

            self.highlight_red.replace(attribute_list);

            let attribute_list = AttrList::new();
            let (red, green, blue) = grey(is_dark).get_rgb_u16();
            attribute_list.insert(AttrColor::new_foreground(red, green, blue));

            self.grey.replace(attribute_list);
        }
    }
}

fn highlight_attrlist(color: Palette<'_>) -> AttrList {
    let attribute_list = AttrList::new();
    attribute_list.insert(AttrInt::new_weight(Weight::Bold));
    let (red, green, blue) = color.get_rgb_u16();
    attribute_list.insert(AttrColor::new_foreground(red, green, blue));
    attribute_list
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

impl ObjectImpl for UnitListPanelImp {
    fn constructed(&self) {
        self.parent_constructed();

        //self.force_selected_index.set(GTK_INVALID_LIST_POSITION);

        let list_model: gio::ListModel = self.units_browser.columns();

        column_view_column_set_sorter!(list_model, 0, primary, dbus_level);
        column_view_column_set_sorter!(list_model, 1, unit_type);
        column_view_column_set_sorter!(list_model, 2, enable_status);
        column_view_column_set_sorter!(list_model, 3, load_state);
        column_view_column_set_sorter!(list_model, 4, active_state);
        column_view_column_set_sorter!(list_model, 5, sub_state);

        let sorter = self.units_browser.sorter();

        self.unit_list_sort_list_model.set_sorter(sorter.as_ref());

        let search_entry = fill_search_bar(&self.search_bar, &self.filter_list_model);

        self.search_entry
            .set(search_entry)
            .expect("Search entry set once");

        {
            let unit_list = self.obj().clone();
            let units_browser = self.units_browser.clone();
            self.scrolled_window
                .vadjustment()
                .connect_changed(move |_adjustment| {
                    focus_on_row(&unit_list, &units_browser);
                });
        }
    }
}

fn focus_on_row(unit_list: &super::UnitListPanel, units_browser: &gtk::ColumnView) {
    let force_selected_index = unit_list.force_selected_index();
    debug!("vadjustment changed");
    unit_list.set_force_selected_index(None);

    let Some(mut force_selected_index) = force_selected_index else {
        return;
    };

    if force_selected_index == GTK_INVALID_LIST_POSITION {
        force_selected_index = 0;
    }
    info!("Focus on selected unit list row (index {force_selected_index})");

    //needs a bit of time to generate the list
    units_browser.scroll_to(
        force_selected_index, // to centerish on the selected unit
        None,
        ListScrollFlags::FOCUS,
        None,
    );
}
impl WidgetImpl for UnitListPanelImp {}
impl BoxImpl for UnitListPanelImp {}

fn fill_search_bar(
    search_bar: &SearchBar,
    filter_list_model: &gtk::FilterListModel,
) -> gtk::SearchEntry {
    let search_entry = gtk::SearchEntry::new();
    search_entry.set_hexpand(true);

    let mut filter_button_unit_type = ExMenuButton::new("Type");
    let mut filter_button_status = ExMenuButton::new("Enablement");
    let mut filter_button_active = ExMenuButton::new("Active");

    filter_button_unit_type.set_tooltip_text(Some("Filter by types"));
    filter_button_status.set_tooltip_text(Some("Filter by enablement status"));
    filter_button_active.set_tooltip_text(Some("Filter by active state"));

    let search_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(5)
        .build();

    for unit_type in UnitType::iter().filter(|x| !matches!(*x, UnitType::Unknown(_))) {
        filter_button_unit_type.add_item(unit_type.to_str());
    }

    for status in EnablementStatus::iter().filter(|x| match *x {
        EnablementStatus::Unknown => false,
        //EnablementStatus::Unasigned => false,
        _ => true,
    }) {
        filter_button_status.add_item(status.as_str());
    }

    for status in ActiveState::iter() {
        filter_button_active.add_item(status.as_str());
    }

    search_box.append(&search_entry);
    search_box.append(&filter_button_unit_type);
    search_box.append(&filter_button_status);
    search_box.append(&filter_button_active);

    search_bar.set_child(Some(&search_box));

    {
        let entry1 = search_entry.clone();

        let custom_filter = {
            let filter_button_unit_type = filter_button_unit_type.clone();
            let filter_button_status = filter_button_status.clone();
            let filter_button_active = filter_button_active.clone();

            gtk::CustomFilter::new(move |object| {
                let Some(unit) = object.downcast_ref::<UnitInfo>() else {
                    error!("some wrong downcast_ref {:?}", object);
                    return false;
                };

                let text = entry1.text();

                let unit_type = unit.unit_type();
                let enable_status: EnablementStatus = unit.enable_status().into();
                let active_state: ActiveState = unit.active_state();

                filter_button_unit_type.contains_value(Some(&unit_type))
                    && filter_button_status.contains_value(Some(enable_status.as_str()))
                    && if text.is_empty() {
                        true
                    } else {
                        unit.display_name().contains(text.as_str())
                    }
                    && filter_button_active.contains_value(Some(active_state.as_str()))
            })
        };

        let on_close = OnClose::new_filter(&custom_filter);
        filter_button_unit_type.set_on_close(on_close);

        let on_close = OnClose::new_filter(&custom_filter);
        filter_button_status.set_on_close(on_close);

        let on_close = OnClose::new_filter(&custom_filter);
        filter_button_active.set_on_close(on_close);

        filter_list_model.set_filter(Some(&custom_filter));

        let last_filter_string = Rc::new(BoxedAnyObject::new(String::new()));

        search_entry.connect_search_changed(move |entry| {
            let text = entry.text();

            debug!("Search text \"{text}\"");

            let mut last_filter: RefMut<String> = last_filter_string.borrow_mut();

            let change_type = if text.is_empty() {
                gtk::FilterChange::LessStrict
            } else if text.len() > last_filter.len() && text.contains(last_filter.as_str()) {
                gtk::FilterChange::MoreStrict
            } else if text.len() < last_filter.len() && last_filter.contains(text.as_str()) {
                gtk::FilterChange::LessStrict
            } else {
                gtk::FilterChange::Different
            };

            debug!("Current \"{}\" Prev \"{}\"", text, last_filter);
            last_filter.replace_range(.., text.as_str());
            custom_filter.changed(change_type);

            //FIXME when the filter become empty the colunm view display nothing until you click on it
            //unit_col_view_scrolled_window.queue_draw(); //TODO investigate the need
        });

        search_entry
    }
}
