use std::cell::OnceCell;

use gio::glib::{Object, object::Cast};
use gtk::{
    SignalListItemFactory,
    glib::{self},
    prelude::*,
    subclass::prelude::*,
};
use log::warn;

use crate::widget::unit_properties_selector::data::PropertiesSelectorObject;

use super::UnitPropertiesSelection;

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/unit_properties_selection.ui")]
pub struct UnitPropertiesSelectionImp {
    #[template_child]
    properties_selection: TemplateChild<gtk::ColumnView>,

    #[template_child]
    interface_column: TemplateChild<gtk::ColumnViewColumn>,

    #[template_child]
    property_column: TemplateChild<gtk::ColumnViewColumn>,

    #[template_child]
    signature_column: TemplateChild<gtk::ColumnViewColumn>,

    #[template_child]
    access_column: TemplateChild<gtk::ColumnViewColumn>,

    list_store: OnceCell<gio::ListStore>,
}

impl UnitPropertiesSelectionImp {
    pub fn add_new_property(&self, new_property_object: PropertiesSelectorObject) {
        let Some(list_store) = self.list_store.get() else {
            warn!("Not None");
            return;
        };

        list_store.append(&new_property_object);
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
        //klass.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for UnitPropertiesSelectionImp {
    fn constructed(&self) {
        self.parent_constructed();

        let store = gio::ListStore::new::<PropertiesSelectorObject>();

        self.list_store.set(store.clone()).expect("Only once");

        let selection_model = gtk::NoSelection::new(Some(store));

        self.properties_selection.set_model(Some(&selection_model));

        let factory_interface = gtk::SignalListItemFactory::new();
        factory_interface.connect_setup(setup);
        factory_interface.connect_bind(bind_interface);

        self.interface_column.set_factory(Some(&factory_interface));

        let factory_property = gtk::SignalListItemFactory::new();
        factory_property.connect_setup(setup);
        factory_property.connect_bind(|_fac, item| {
            bind(item, PropertiesSelectorObject::unit_property);
        });
        self.property_column.set_factory(Some(&factory_property));

        let signature_factory = gtk::SignalListItemFactory::new();
        signature_factory.connect_setup(setup);
        signature_factory.connect_bind(|_fac, item| {
            bind(item, PropertiesSelectorObject::signature);
        });

        self.signature_column.set_factory(Some(&signature_factory));
        let access_factory = gtk::SignalListItemFactory::new();
        access_factory.connect_setup(setup);
        access_factory.connect_bind(|_fac, item| {
            bind(item, PropertiesSelectorObject::access);
        });
        self.access_column.set_factory(Some(&access_factory));
    }
}

fn setup(_fac: &SignalListItemFactory, item: &Object) {
    let item = item.downcast_ref::<gtk::ListItem>().unwrap();
    let label = gtk::Inscription::builder().xalign(0.0).build();
    item.set_child(Some(&label));
}

fn bind(item: &Object, func: fn(&PropertiesSelectorObject) -> String) {
    let item = item.downcast_ref::<gtk::ListItem>().unwrap();

    let widget = item.child();

    let label = widget.and_downcast_ref::<gtk::Inscription>().unwrap();

    let property_object = item
        .item()
        .unwrap()
        .downcast::<PropertiesSelectorObject>()
        .unwrap();

    let value = func(&property_object);
    let value = value.split('.').next_back();

    label.set_text(value)
}

fn bind_interface(_: &gtk::SignalListItemFactory, item: &Object) {
    let item = item.downcast_ref::<gtk::ListItem>().unwrap();

    let widget = item.child();

    let label = widget.and_downcast_ref::<gtk::Inscription>().unwrap();

    let property_object = item
        .item()
        .unwrap()
        .downcast::<PropertiesSelectorObject>()
        .unwrap();

    let value = property_object.interface();
    label.set_text(Some(&value))
}

impl WidgetImpl for UnitPropertiesSelectionImp {}
impl BoxImpl for UnitPropertiesSelectionImp {}
