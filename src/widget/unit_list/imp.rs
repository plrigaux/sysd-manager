use std::{
    cell::{OnceCell, RefMut},
    rc::Rc,
};

use gtk::{
    gio::{self},
    glib::{self, BoxedAnyObject, Object},
    prelude::*,
    subclass::{
        box_::BoxImpl,
        prelude::*,
        widget::{
            CompositeTemplateCallbacksClass, CompositeTemplateClass,
            CompositeTemplateInitializingExt, WidgetImpl,
        },
    },
    SearchBar, TemplateChild,
};

use log::{debug, error, info, warn};

use crate::{
    systemd::{
        self,
        data::UnitInfo,
        enums::{ActiveState, EnablementStatus, UnitType},
    },
    widget::{app_window::AppWindow, menu_button::ExMenuButton},
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

    search_entry: OnceCell<gtk::SearchEntry>,
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
        let entry = item.item().and_downcast::<UnitInfo>().unwrap();
        (child, entry)
    }};
}

macro_rules! factory_bind {
    ($item_obj:expr, $func:ident) => {{
        let (child, entry) = factory_bind_pre!($item_obj);
        let v = entry.$func();
        child.set_text(Some(&v));
        (child, entry)
    }};
}

macro_rules! create_column_filter {
    ($func:ident) => {{
        let col_sorter = gtk::CustomSorter::new(move |obj1, obj2| {
            let Some(unit1) = obj1.downcast_ref::<UnitInfo>() else {
                panic!("some wrong downcast_ref {:?}", obj1);
            };

            let Some(unit2) = obj2.downcast_ref::<UnitInfo>() else {
                panic!("some wrong downcast_ref {:?}", obj2);
            };

            unit1.$func().cmp(&unit2.$func()).into()
        });
        col_sorter
    }};
}

macro_rules! column_view_column_set_sorter {
    ($list_item:expr, $col_idx:expr, $sort_func:ident) => {
        let item = $list_item.item($col_idx);

        let item_out = item.expect("Expect item x to be not None");
        let downcast_ref = item_out
            .downcast_ref::<gtk::ColumnViewColumn>()
            .expect("item.downcast_ref::<gtk::ColumnViewColumn>()");

        let sorter = create_column_filter!($sort_func);
        downcast_ref.set_sorter(Some(&sorter));
    };
}

#[gtk::template_callbacks]
impl UnitListPanelImp {
    #[template_callback]
    fn col_unit_name_factory_setup(_fac: &gtk::SignalListItemFactory, item_obj: &Object) {
        factory_setup!(item_obj);
    }

    #[template_callback]
    fn col_unit_name_factory_bind(_fac: &gtk::SignalListItemFactory, item_obj: &Object) {
        factory_bind!(item_obj, display_name);
    }

    #[template_callback]
    fn col_type_factory_setup(_fac: &gtk::SignalListItemFactory, item_obj: &Object) {
        factory_setup!(item_obj);
    }

    #[template_callback]
    fn col_type_factory_bind(_fac: &gtk::SignalListItemFactory, item_obj: &Object) {
        factory_bind!(item_obj, unit_type);
    }

    #[template_callback]
    fn col_enable_status_factory_setup(_fac: &gtk::SignalListItemFactory, item_obj: &Object) {
        factory_setup!(item_obj);
    }

    #[template_callback]
    fn col_enable_status_factory_bind(_fac: &gtk::SignalListItemFactory, item_obj: &Object) {
        let (child, entry) = factory_bind_pre!(item_obj);

        let status_code: EnablementStatus = entry.enable_status().into();

        child.set_text(Some(status_code.to_str()));

        entry
            .bind_property("enable_status", &child, "text")
            .transform_to(|_, status: u32| {
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
        let entry = item.item().and_downcast::<UnitInfo>().unwrap();
        let icon_name = &entry.active_state_icon();
        child.set_icon_name(icon_name.as_deref());
        entry
            .bind_property("active_state_icon", &child, "icon-name")
            .build();
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

    pub(super) fn register_selection_change(&self, app_window: &AppWindow) {
        /*      error!("register_selection_change");
        if let Err(_result) = self.app_window.set(app_window.clone()) {
            warn!("One cell error! It was full.")
        }; */

        let app_window = app_window.clone();

        self.single_selection
            .connect_selected_item_notify(move |single_selection| {
                info!("connect_selected_notify ");
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

                app_window.selection_change(&unit);
            }); // FOR THE SEARCH
    }

    pub fn search_bar(&self) -> gtk::SearchBar {
        self.search_bar.clone()
    }

    pub(super) fn fill_store(&self) {
        let unit_files: Vec<UnitInfo> = match systemd::list_units_description_and_state() {
            Ok(map) => map.into_values().collect(),
            Err(_e) => vec![],
        };

        self.list_store.remove_all();

        for value in unit_files {
            self.list_store.append(&value);
        }
        info!(
            "Unit list refreshed! list size {}",
            self.list_store.n_items()
        )
    }

    pub(super) fn button_search_toggled(&self, toggle_button_is_active: bool) {
        self.search_bar.set_search_mode(toggle_button_is_active);

        if toggle_button_is_active {
            let se = self.search_entry.get().unwrap();

            se.grab_focus();
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

impl ObjectImpl for UnitListPanelImp {
    fn constructed(&self) {
        self.parent_constructed();

        let list_model: gio::ListModel = self.units_browser.columns();

        column_view_column_set_sorter!(list_model, 0, primary);
        column_view_column_set_sorter!(list_model, 1, unit_type);
        column_view_column_set_sorter!(list_model, 2, enable_status);
        column_view_column_set_sorter!(list_model, 3, active_state);

        let sorter = self.units_browser.sorter();

        self.unit_list_sort_list_model.set_sorter(sorter.as_ref());

        self.fill_store();

        let search_entry = fill_search_bar(&self.search_bar, &self.filter_list_model);

        let _ = self.search_entry.set(search_entry);
    }
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

    let search_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(5)
        .build();

    for unit_type in UnitType::iter().filter(|x| match *x {
        UnitType::Unknown(_) => false,
        _ => true,
    }) {
        filter_button_unit_type.add_item(unit_type.to_str());
    }

    for status in EnablementStatus::iter().filter(|x| match *x {
        EnablementStatus::Unknown => false,
        //EnablementStatus::Unasigned => false,
        _ => true,
    }) {
        filter_button_status.add_item(status.to_str());
    }

    for status in ActiveState::iter().filter(|x| match *x {
        ActiveState::Unknown => false,
        //EnablementStatus::Unasigned => false,
        _ => true,
    }) {
        filter_button_active.add_item(status.label());
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

            let custom_filter = gtk::CustomFilter::new(move |object| {
                let Some(unit) = object.downcast_ref::<UnitInfo>() else {
                    error!("some wrong downcast_ref {:?}", object);
                    return false;
                };

                let text = entry1.text();

                let unit_type = unit.unit_type();
                let enable_status: EnablementStatus = unit.enable_status().into();
                let active_state: ActiveState = unit.active_state().into();

                filter_button_unit_type.contains_value(&Some(unit_type))
                    && filter_button_status.contains_value(&Some(enable_status.to_str().to_owned()))
                    && if text.is_empty() {
                        true
                    } else {
                        unit.display_name().contains(text.as_str())
                    }
                    && filter_button_active.contains_value(&Some(active_state.to_string()))
            });

            custom_filter
        };

        filter_button_unit_type.set_filter(custom_filter.clone());
        filter_button_status.set_filter(custom_filter.clone());
        filter_button_active.set_filter(custom_filter.clone());

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
