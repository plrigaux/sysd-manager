use std::cell::OnceCell;

use gtk::{
    gio::{self},
    glib::{self, Object},
    prelude::*,
    subclass::{
        box_::BoxImpl,
        prelude::*,
        widget::{
            CompositeTemplateCallbacksClass, CompositeTemplateClass,
            CompositeTemplateInitializingExt, WidgetImpl,
        },
    },
    TemplateChild,
};

use log::{error, info, warn};

use crate::{
    systemd::{self, data::UnitInfo, enums::EnablementStatus},
    widget::app_window::AppWindow,
};

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

    app_window: OnceCell<AppWindow>,
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

        entry.bind_property("enable_status", &child, "text").build();
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
        child.set_icon_name(Some(&entry.active_state_icon()));
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
        factory_bind!(item_obj, description);
    }

    #[template_callback]
    fn single_selection_selection_changed(&self, position: u32) {
        let Some(object) = self.single_selection.selected_item() else {
            warn!("No object selected, position {position}");
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

        match self.app_window.get() {
            Some(win) => win.selection_change(&unit),
            None => warn!("No selection_change handler"),
        }
    }

    pub(super) fn register_selection_change(&self, app_window: &AppWindow) {
        if let Err(_result) = self.app_window.set(app_window.clone()) {
            warn!("One cell error! It was full.")
        };
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

        fill_store(&self.list_store);

        warn!("UnitListPanelImp constructed");
    }
}
impl WidgetImpl for UnitListPanelImp {}
impl BoxImpl for UnitListPanelImp {}

fn fill_store(store: &gio::ListStore) {
    let unit_files: Vec<UnitInfo> = match systemd::list_units_description_and_state() {
        Ok(map) => map.into_values().collect(),
        Err(_e) => vec![],
    };

    store.remove_all();

    for value in unit_files {
        store.append(&value);
    }
    info!("Unit list refreshed! list size {}", store.n_items())
}
