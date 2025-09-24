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
            data::PropertiesSelectorObject,
            unit_properties_selection::{
                data::UnitPropertySelection, row::UnitPropertiesSelectionRow,
            },
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

#[gtk::template_callbacks]
impl UnitPropertiesSelectionImp {
    #[template_callback]
    fn apply_clicked(&self, _button: &gtk::Button) {
        info!("Apply pressed");

        let Some(list_store) = self.list_store.get() else {
            error!("list_store not set");
            return;
        };

        let n_item = list_store.n_items();
        let mut list = Vec::with_capacity(n_item as usize);
        for i in 0..n_item {
            let item = list_store.item(i);
            let Some(item) = item.and_downcast_ref::<UnitPropertySelection>() else {
                warn!("Bad downcast {:?}", list_store.item(i));
                continue;
            };

            let interface = UnitType::from_intreface(&item.interface());
            let unit_property = UnitProperty::new(
                interface,
                item.unit_property(),
                item.signature(),
                item.access(),
            );

            list.push(unit_property);
        }
        if let Some(unit_list_panel) = self.unit_list_panel.get() {
            unit_list_panel.set_new_columns(list);
        } else {
            error!("No unit list panel");
        }
    }
}

impl UnitPropertiesSelectionImp {
    pub fn add_new_property(&self, new_property_object: PropertiesSelectorObject) {
        let Some(list_store) = self.list_store.get() else {
            warn!("Not None");
            return;
        };

        let new_unit_prop = UnitPropertySelection::from_po(new_property_object);
        list_store.append(&new_unit_prop);
    }

    pub(super) fn set_unit_list_panel(&self, unit_list_panel: &UnitListPanel) {
        self.unit_list_panel
            .set(unit_list_panel.clone())
            .expect("Assigned only once");
    }

    pub(super) fn get_list_store(&self) -> Option<&gio::ListStore> {
        self.list_store.get()
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
