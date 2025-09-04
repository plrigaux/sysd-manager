use std::cell::{OnceCell, RefCell};

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
use log::{debug, error, info, warn};

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

    #[template_child]
    access_column: TemplateChild<gtk::ColumnViewColumn>,

    #[template_child]
    search_entry: TemplateChild<gtk::SearchEntry>,

    #[template_child]
    toogle_button: TemplateChild<gtk::ToggleButton>,

    last_filter_string: RefCell<String>,

    custom_filter: OnceCell<gtk::CustomFilter>,

    tree_list_model: OnceCell<gtk::TreeListModel>,
}

#[gtk::template_callbacks]
impl UnitPropertiesSelectorDialogImp {
    #[template_callback]
    fn search_entry_changed(&self, search_entry: &gtk::SearchEntry) {
        let text = search_entry.text();

        debug!("Search text \"{text}\"");

        let mut last_filter = self.last_filter_string.borrow_mut();

        let text_is_empty = text.is_empty();
        if !text_is_empty {
            /*             let nb_item = tree_list_model.model().n_items();

            for (a, b) in tree_list_model.model().into_iter().enumerate() {
                info!("{a} {b:?}");
            } */

            self.toogle_button.set_active(true);
        }

        let change_type = if text_is_empty {
            gtk::FilterChange::LessStrict
        } else if text.len() > last_filter.len() && text.contains(last_filter.as_str()) {
            gtk::FilterChange::MoreStrict
        } else if text.len() < last_filter.len() && last_filter.contains(text.as_str()) {
            gtk::FilterChange::LessStrict
        } else {
            gtk::FilterChange::Different
        };

        debug!("Current \"{text}\" Prev \"{last_filter}\"");
        last_filter.replace_range(.., text.as_str());

        if let Some(custom_filter) = self.custom_filter.get() {
            custom_filter.changed(change_type);
        }

        //self.set_filter_icon()
    }

    #[template_callback]
    fn expand_toggled(&self, toogle_button: &gtk::ToggleButton) {
        info!("expand_toggled {}", toogle_button.is_active());

        self.expand_interfaces(toogle_button.is_active());

        if toogle_button.is_active() {
            toogle_button.set_icon_name("go-down-symbolic");
            toogle_button.set_tooltip_text(Some("Collapse all interfaces"));
        } else {
            toogle_button.set_icon_name("go-next-symbolic");
            toogle_button.set_tooltip_text(Some("Expand all interfaces"));
        }
    }

    fn expand_interfaces(&self, expand: bool) {
        let Some(tree_list_model) = self.tree_list_model.get() else {
            warn!("Can't find tree list model");
            return;
        };

        let nb_item = tree_list_model.model().n_items();

        for i in 0..nb_item {
            if let Some(row) = tree_list_model.row(i) {
                row.set_expanded(expand);
            }
        }
    }

    fn create_filter(&self) -> gtk::CustomFilter {
        let search_entry = self.search_entry.clone();

        gtk::CustomFilter::new(move |object| {
            let text_gs = search_entry.text();
            if text_gs.is_empty() {
                return true;
            }

            let Some(tree_list_row) = object.downcast_ref::<TreeListRow>() else {
                error!("some wrong downcast_ref {object:?}");
                return false;
            };

            if tree_list_row.children().is_some() {
                return true;
            }

            let item = tree_list_row.item();
            let Some(prop_selector) = item.and_downcast_ref::<PropertiesSelectorObject>() else {
                error!("some wrong downcast_ref {object:?}");
                return false;
            };

            let texts = text_gs.as_str();

            //if an upper case --> filter
            if text_gs.chars().any(|c| c.is_ascii_uppercase()) {
                prop_selector.unit_property().contains(texts)
            } else {
                prop_selector
                    .unit_property()
                    .to_ascii_lowercase()
                    .contains(texts)
            }
        })
    }
}

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

        let tree_list_model = gtk::TreeListModel::new(store.clone(), false, false, add_tree_node);

        self.tree_list_model
            .set(tree_list_model.clone())
            .expect("set once only");

        let filter = self.create_filter();

        self.custom_filter
            .set(filter.clone())
            .expect("custom filter set once");

        let filtering_model = gtk::FilterListModel::new(Some(tree_list_model), Some(filter));

        let selection_model = gtk::SingleSelection::new(Some(filtering_model));

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

            let label = gtk::Label::builder().xalign(0.0).build();
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

            let label = expander.child().unwrap().downcast::<gtk::Label>().unwrap();

            let tree_list_row = item.item().unwrap().downcast::<TreeListRow>().unwrap();

            expander.set_list_row(Some(&tree_list_row));

            let property_object = tree_list_row
                .item()
                .and_downcast::<PropertiesSelectorObject>()
                .unwrap();

            label.set_text(&property_object.interface());
        });
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

        for (inteface, mut props) in map
            .into_iter()
            .filter(|(k, _v)| k.starts_with("org.freedesktop.systemd1"))
        {
            let obj = PropertiesSelectorObject::new_interface(inteface);
            props.sort();
            for property in props {
                let prop_object = PropertiesSelectorObject::from(property);
                obj.add_child(prop_object);
            }

            store.append(&obj);
        }
    }
}

fn setup(_fac: &gtk::SignalListItemFactory, item: &Object) {
    let item = item.downcast_ref::<gtk::ListItem>().unwrap();
    let label = gtk::Label::builder().xalign(0.0).build();
    item.set_child(Some(&label));
}

fn bind(item: &Object, func: fn(&PropertiesSelectorObject) -> String) {
    let item = item.downcast_ref::<gtk::ListItem>().unwrap();

    let widget = item.child();

    let label = widget.and_downcast_ref::<gtk::Label>().unwrap();

    let tree_list_row = item.item().unwrap().downcast::<TreeListRow>().unwrap();
    let property_object = tree_list_row
        .item()
        .and_downcast::<PropertiesSelectorObject>()
        .unwrap();

    let value = func(&property_object);
    label.set_text(&value)
}

fn add_tree_node(object: &Object) -> Option<gio::ListModel> {
    let Some(prop_selector) = object.downcast_ref::<PropertiesSelectorObject>() else {
        warn!("object type: {:?} {object:?}", object.type_());
        return None;
    };

    let store = gio::ListStore::new::<PropertiesSelectorObject>();

    let binding = prop_selector.children();
    let children = (*binding).as_ref()?;

    for child in children.iter() {
        store.append(child)
    }
    Some(store.into())
}

impl WidgetImpl for UnitPropertiesSelectorDialogImp {}
impl WindowImpl for UnitPropertiesSelectorDialogImp {}
impl AdwWindowImpl for UnitPropertiesSelectorDialogImp {}
