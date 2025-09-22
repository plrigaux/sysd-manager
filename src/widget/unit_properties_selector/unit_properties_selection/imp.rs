use std::cell::OnceCell;

use gio::glib::{Object, object::Cast};
use gtk::{
    SignalListItemFactory,
    glib::{self},
    prelude::*,
    subclass::prelude::*,
};
use log::{error, info, warn};

use crate::{
    systemd::{UnitProperty, enums::UnitType},
    widget::{unit_list::UnitListPanel, unit_properties_selector::data::PropertiesSelectorObject},
};

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
            let Some(item) = item.and_downcast_ref::<PropertiesSelectorObject>() else {
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

        list_store.append(&new_property_object);
    }

    pub(super) fn set_unit_list_panel(&self, unit_list_panel: &UnitListPanel) {
        self.unit_list_panel
            .set(unit_list_panel.clone())
            .expect("Assigned only once");
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

        let store = gio::ListStore::new::<PropertiesSelectorObject>();

        self.list_store.set(store.clone()).expect("Only once");

        let selection_model = gtk::NoSelection::new(Some(store.clone()));

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

        let button_fac = gtk::SignalListItemFactory::new();
        button_fac.connect_setup(|_f, o| {
            let item = o.downcast_ref::<gtk::ListItem>().unwrap();
            let label: gtk::Button = gtk::Button::builder().label("X").build();
            item.set_child(Some(&label));
        });

        let list_store = store.clone();
        button_fac.connect_bind(move |_fac, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();

            let widget = item.child();

            let button = widget.and_downcast_ref::<gtk::Button>().unwrap();

            let list_store = list_store.clone();
            let item = item.clone();
            button.connect_clicked(move |_b| {
                list_store.remove(item.position());
            });
        });

        let button_column = gtk::ColumnViewColumn::new(None, Some(button_fac));

        self.properties_selection.append_column(&button_column);
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
