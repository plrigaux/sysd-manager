use std::cell::OnceCell;

use gio::glib::object::Cast;
use gtk::{
    glib::{self},
    prelude::*,
    subclass::prelude::*,
};
use log::{error, info, warn};

use crate::{
    systemd::{UnitProperty, enums::UnitType},
    widget::{
        unit_list::UnitListPanel,
        unit_properties_selector::{
            data::{PropertiesSelectorObject, UnitPropertySelection},
            unit_properties_selection::row::UnitPropertiesSelectionRow,
        },
    },
};

use super::UnitPropertiesSelection;

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/unit_properties_selection.ui")]
pub struct UnitPropertiesSelectionImp {
    #[template_child]
    properties_selection: TemplateChild<gtk::ListView>,

    list_store: OnceCell<gio::ListStore>,

    selection_model: OnceCell<gtk::SingleSelection>,

    unit_list_panel: OnceCell<UnitListPanel>,
}

macro_rules! get_list_store {
    ($self:expr) => {{
        let Some(list_store) = $self.list_store.get() else {
            error!("list_store not set");
            return;
        };
        list_store
    }};
}

#[gtk::template_callbacks]
impl UnitPropertiesSelectionImp {
    #[template_callback]
    fn apply_clicked(&self, _button: &gtk::Button) {
        info!("Apply pressed");

        let list_store = get_list_store!(self);

        let n_item = list_store.n_items();
        let mut list = Vec::with_capacity(n_item as usize);
        for i in 0..n_item {
            let item = list_store.item(i);
            let Some(unit_property) = item.and_downcast_ref::<UnitPropertySelection>() else {
                warn!("Bad downcast {:?}", list_store.item(i));
                continue;
            };

            list.push(unit_property.clone());
        }

        if let Some(unit_list_panel) = self.unit_list_panel.get() {
            unit_list_panel.set_new_columns(list);
        } else {
            error!("No unit list panel");
        }
    }

    #[template_callback]
    fn reset_clicked(&self, _button: &gtk::Button) {
        let Some(unit_list_panel) = self.unit_list_panel.get() else {
            error!("unit_list_panel is None");
            return;
        };

        let list_store = get_list_store!(self);

        list_store.remove_all();
        for unit_property_column in unit_list_panel.default_displayed_columns().iter() {
            list_store.append(unit_property_column);
        }
    }
}

impl UnitPropertiesSelectionImp {
    pub fn add_new_property(&self, new_property_object: PropertiesSelectorObject) {
        let list_store = get_list_store!(self);

        let new_unit_prop = UnitPropertySelection::from_po(new_property_object);
        list_store.append(&new_unit_prop);
    }

    pub(super) fn set_unit_list_panel(&self, unit_list_panel: &UnitListPanel) {
        self.unit_list_panel
            .set(unit_list_panel.clone())
            .expect("Assigned only once");

        let list_store = get_list_store!(self);
        for unit_property_column in unit_list_panel.current_columns().iter() {
            list_store.append(unit_property_column);
        }
    }

    pub(super) fn get_list_store(&self) -> Option<&gio::ListStore> {
        self.list_store.get()
    }

    pub fn move_up(&self, position: u32) {
        if position == 0 {
            return;
        }

        let list_store = get_list_store!(self);

        if let Some(ref prop_seletion) = list_store.item(position) {
            list_store.remove(position);
            list_store.insert(position - 1, prop_seletion);
        } else {
            warn!("list_store of data None");
        };
    }

    pub fn move_down(&self, position: u32) {
        let list_store = get_list_store!(self);

        if position + 1 >= list_store.n_items() {
            return;
        }

        if let Some(ref prop_seletion) = list_store.item(position) {
            list_store.remove(position);
            list_store.insert(position + 1, prop_seletion);
        } else {
            warn!("list_store of data None");
        };
    }

    pub fn delete(&self, position: u32) {
        let Some(list_store) = self.list_store.get() else {
            warn!("list_store should not be None");
            return;
        };

        list_store.remove(position);
    }
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for UnitPropertiesSelectionImp {
    const NAME: &'static str = "UnitPropertiesSelection";
    type Type = UnitPropertiesSelection;
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

impl ObjectImpl for UnitPropertiesSelectionImp {
    fn constructed(&self) {
        self.parent_constructed();

        let store = gio::ListStore::new::<UnitPropertySelection>();

        self.list_store.set(store.clone()).expect("Only once");

        //let selection_model = gtk::NoSelection::new(Some(store.clone()));

        let selection_model = gtk::SingleSelection::builder()
            .can_unselect(true)
            .model(&store.clone())
            .build();

        self.selection_model
            .set(selection_model.clone())
            .expect("Only once");

        self.properties_selection.set_model(Some(&selection_model));
        let factory = gtk::SignalListItemFactory::new();
        {
            let prop_selection = self.obj().clone();
            factory.connect_setup(move |_factory, item| {
                let item = item.downcast_ref::<gtk::ListItem>().unwrap();
                let row = UnitPropertiesSelectionRow::new(prop_selection.clone());
                item.set_child(Some(&row));
            });
        }

        factory.connect_bind(move |_factory, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let prop_selection = item.item().and_downcast::<UnitPropertySelection>().unwrap();

            let child = item
                .child()
                .and_downcast::<UnitPropertiesSelectionRow>()
                .unwrap();

            child.set_data_selection(&prop_selection, item);
        });

        self.properties_selection.set_factory(Some(&factory));
    }
}

impl WidgetImpl for UnitPropertiesSelectionImp {}
impl BoxImpl for UnitPropertiesSelectionImp {}
