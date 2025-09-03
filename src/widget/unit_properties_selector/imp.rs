use adw::subclass::window::AdwWindowImpl;
use gio::glib::Object;
use gtk::{
    TreeListRow,
    glib::{self},
    prelude::*,
    subclass::{
        prelude::*,
        widget::{
            CompositeTemplateCallbacksClass, CompositeTemplateClass,
            CompositeTemplateInitializingExt, WidgetImpl,
        },
    },
};
use log::{error, info, warn};

use crate::{systemd, widget::unit_properties_selector::data::PropertiesSelectorObject};

use super::UnitPropertiesSelectorDialog;

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/unit_properties_selector.ui")]
pub struct UnitPropertiesSelectorDialogImp {
    #[template_child]
    properties_selector: TemplateChild<gtk::ColumnView>,

    #[template_child]
    interface_column: TemplateChild<gtk::ColumnViewColumn>,

    #[template_child]
    property_column: TemplateChild<gtk::ColumnViewColumn>,

    #[template_child]
    signature_column: TemplateChild<gtk::ColumnViewColumn>,
}

#[gtk::template_callbacks]
impl UnitPropertiesSelectorDialogImp {}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for UnitPropertiesSelectorDialogImp {
    const NAME: &'static str = "UNIT_PROPERTIES_SELECTOR_DIALOG";
    type Type = UnitPropertiesSelectorDialog;
    type ParentType = adw::Window;

    fn class_init(klass: &mut Self::Class) {
        // The layout manager determines how child widgets are laid out.
        klass.bind_template();
        klass.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for UnitPropertiesSelectorDialogImp {
    fn constructed(&self) {
        self.parent_constructed();

        let store = gio::ListStore::new::<PropertiesSelectorObject>();

        let model = gtk::TreeListModel::new(store.clone(), false, false, add_tree_node);
        let selection_model = gtk::SingleSelection::new(Some(model));

        self.properties_selector.set_model(Some(&selection_model));

        let map = match systemd::fetch_unit_properties() {
            Ok(map) => map,
            Err(err) => {
                error!("{err:?}");
                return;
            }
        };

        let factory_interface = gtk::SignalListItemFactory::new();

        factory_interface.connect_setup(|_fac, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();

            let label = gtk::Inscription::builder().width_request(300).build();
            let expander = gtk::TreeExpander::new();
            expander.set_child(Some(&label));
            item.set_child(Some(&expander));
        });

        factory_interface.connect_bind(|_fac, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();

            let expander = item
                .child()
                .unwrap()
                .downcast::<gtk::TreeExpander>()
                .unwrap();

            let label = expander
                .child()
                .unwrap()
                .downcast::<gtk::Inscription>()
                .unwrap();

            let tree_list_row = item.item().unwrap().downcast::<TreeListRow>().unwrap();

            expander.set_list_row(Some(&tree_list_row));

            let property_object = tree_list_row
                .item()
                .and_downcast::<PropertiesSelectorObject>()
                .unwrap();

            label.set_text(property_object.interface().as_deref());
        });
        self.interface_column.set_factory(Some(&factory_interface));

        let factory_property = gtk::SignalListItemFactory::new();

        fn setup(_fac: &gtk::SignalListItemFactory, item: &Object) {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let label = gtk::Inscription::builder().build();
            item.set_child(Some(&label));
        }

        factory_property.connect_setup(setup);

        factory_property.connect_bind(|_fac, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();

            let widget = item.child();

            let label = widget.and_downcast_ref::<gtk::Inscription>().unwrap();

            let tree_list_row = item.item().unwrap().downcast::<TreeListRow>().unwrap();
            let property_object = tree_list_row
                .item()
                .and_downcast::<PropertiesSelectorObject>()
                .unwrap();

            label.set_text(property_object.unit_property().as_deref());
        });

        self.property_column.set_factory(Some(&factory_property));

        let signature_factory = gtk::SignalListItemFactory::new();

        signature_factory.connect_setup(setup);

        signature_factory.connect_bind(|_fac, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();

            let widget = item.child();

            let label = widget.and_downcast_ref::<gtk::Inscription>().unwrap();

            let tree_list_row = item.item().unwrap().downcast::<TreeListRow>().unwrap();
            let property_object = tree_list_row
                .item()
                .and_downcast::<PropertiesSelectorObject>()
                .unwrap();

            label.set_text(property_object.signature().as_deref());
        });

        self.signature_column.set_factory(Some(&signature_factory));

        info!("MAP {:?}", map.keys());

        for (inteface, props) in map.iter() {
            let obj = PropertiesSelectorObject::default();
            obj.set_interface(inteface.as_str());

            for (property, signature) in props {
                let prop_object = PropertiesSelectorObject::ps(property, signature);
                obj.add_child(prop_object);
            }

            store.append(&obj);
        }
    }
}

fn add_tree_node(object: &Object) -> Option<gio::ListModel> {
    let Some(prop_selector) = object.downcast_ref::<PropertiesSelectorObject>() else {
        warn!("object type: {:?} {object:?}", object.type_());
        return None;
    };

    let store = gio::ListStore::new::<PropertiesSelectorObject>();

    let children = prop_selector.children();

    if children.is_empty() {
        return None;
    }

    for child in children.iter() {
        store.append(child)
    }
    Some(store.into())
}

impl WidgetImpl for UnitPropertiesSelectorDialogImp {}
impl WindowImpl for UnitPropertiesSelectorDialogImp {}
impl AdwWindowImpl for UnitPropertiesSelectorDialogImp {}
