use std::cell::{OnceCell, RefCell};

use adw::subclass::window::AdwWindowImpl;
use gio::glib::{Object, Variant};
use gtk::{
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

use crate::{
    systemd::{self, runtime},
    systemd_gui::new_settings,
    widget::{
        unit_list::UnitListPanel,
        unit_properties_selector::{
            data::PropertiesSelectorObject, unit_properties_selection::UnitPropertiesSelection,
        },
    },
};

use super::UnitPropertiesSelectorDialog;

const WINDOW_SIZE: &str = "unit-property-window-size";
const PANED_SEPARATOR_POSITION: &str = "unit-property-paned-separator-position";

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

    #[template_child]
    unit_properties_selection: TemplateChild<UnitPropertiesSelection>,

    #[template_child]
    paned: TemplateChild<gtk::Paned>,

    last_filter_string: RefCell<String>,

    custom_filter: OnceCell<gtk::CustomFilter>,

    tree_list_model: OnceCell<gtk::TreeListModel>,
}

#[gtk::template_callbacks]
impl UnitPropertiesSelectorDialogImp {
    #[template_callback]
    fn search_entry_changed(&self, search_entry: &gtk::SearchEntry) {
        let text = search_entry.text();

        let mut last_filter = self.last_filter_string.borrow_mut();

        let text_is_empty = text.is_empty();
        if !text_is_empty {
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

        debug!("Search text. Current \"{text}\" Prev \"{last_filter}\"");
        last_filter.replace_range(.., text.as_str());

        if let Some(custom_filter) = self.custom_filter.get() {
            custom_filter.changed(change_type);
        }
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
        warn!("tree_list_model {}", nb_item);
        for i in 0..nb_item {
            if let Some(row) = tree_list_model.child_row(i) {
                warn!("set_expanded {}", i);
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

            let Some(tree_list_row) = object.downcast_ref::<gtk::TreeListRow>() else {
                error!("some wrong downcast_ref {object:?}");
                return false;
            };

            info!("Depth {} ", tree_list_row.depth());

            if let Some(children) = tree_list_row.children() {
                let item = tree_list_row.item();
                if let Some(prop_selector) = item.and_downcast_ref::<PropertiesSelectorObject>() {
                    info!(
                        "Child model {} {}",
                        children.n_items(),
                        prop_selector.interface()
                    );
                } else {
                    error!("some wrong downcast_ref {object:?}");
                };

                return true;
            }

            let item = tree_list_row.item();
            let Some(prop_selector) = item.and_downcast_ref::<PropertiesSelectorObject>() else {
                error!("some wrong downcast_ref {object:?}");
                return false;
            };

            info!(
                "Inter {:?} Prop {:?}",
                prop_selector.interface(),
                prop_selector.unit_property()
            );

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

    pub(super) fn set_unit_list(&self, unit_list_panel: &UnitListPanel) {
        self.unit_properties_selection
            .set_unit_list(unit_list_panel);

        let Some(tree_list_model) = self.tree_list_model.get() else {
            warn!("Not None");
            return;
        };

        let interface_name = "Basic Columns";
        let default = PropertiesSelectorObject::new_interface(interface_name.to_owned());
        // list_store.append(&default);

        for default_column in unit_list_panel.default_columns() {
            let Some(property_name) = default_column.title() else {
                warn!("Column with no title");
                continue;
            };

            let new_property_object =
                PropertiesSelectorObject::from_column(property_name.to_string());
            default.add_child(new_property_object);
        }

        let model = tree_list_model.model();
        let store = model.downcast_ref::<gio::ListStore>().unwrap();
        store.append(&default);
    }

    fn load_window_size(&self) {
        // Get the window state from `settings`
        let settings = new_settings();

        let size = settings.value(WINDOW_SIZE);

        let (mut width, mut height) = size.get::<(i32, i32)>().unwrap();

        let mut separator_position = settings.int(PANED_SEPARATOR_POSITION);

        info!(
            "Window settings: width {width}, height {height},  panes position {separator_position}"
        );

        let obj = self.obj();
        let (def_width, def_height) = obj.default_size();

        if width < 0 {
            width = def_width;
            if width < 0 {
                width = 1280;
            }
        }

        if height < 0 {
            height = def_height;
            if height < 0 {
                height = 720;
            }
        }

        // Set the size of the window
        obj.set_default_size(width, height);

        if separator_position < 0 {
            separator_position = width / 2;
        }

        self.paned.set_position(separator_position);
    }

    pub fn save_window_context(&self) -> Result<(), glib::BoolError> {
        // Get the size of the window

        let obj = self.obj();
        let size = obj.default_size();

        let settings = new_settings();

        let value: Variant = size.into();
        settings.set_value(WINDOW_SIZE, &value)?;

        let separator_position = self.paned.position();
        settings.set_int(PANED_SEPARATOR_POSITION, separator_position)?;

        Ok(())
    }
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for UnitPropertiesSelectorDialogImp {
    const NAME: &'static str = "UnitPropertiesSelectorDialog";
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

        self.load_window_size();

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
        /*         warn!("incremental {}", filtering_model.is_incremental());
        filtering_model.set_incremental(true); */

        let selection_model = gtk::SingleSelection::builder()
            .model(&filtering_model)
            .can_unselect(true)
            .autoselect(false)
            .build();

        self.properties_selector.set_model(Some(&selection_model));

        let factory_interface = gtk::SignalListItemFactory::new();

        factory_interface.connect_setup(|_fac, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();

            let label = gtk::Label::builder().xalign(0.0).build();
            //let label = gtk::Inscription::builder().build();
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

            let tree_list_row = item.item().unwrap().downcast::<gtk::TreeListRow>().unwrap();

            expander.set_list_row(Some(&tree_list_row));

            let property_object = tree_list_row
                .item()
                .and_downcast::<PropertiesSelectorObject>()
                .unwrap();

            let interface = property_object.interface();
            if let Some(unit_type) = interface.split('.').next_back() {
                label.set_text(unit_type);
            } else {
                label.set_text("");
            }
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

        let unit_properties_selection = self.unit_properties_selection.clone();
        selection_model.connect_selected_item_notify(move |single_selection| {
            debug!(
                "connect_selected_notify idx {}",
                single_selection.selected()
            );
            let Some(object) = single_selection.selected_item() else {
                warn!("No object selected");
                return;
            };

            let tree_list_row = object.downcast::<gtk::TreeListRow>().unwrap();

            let property_object = tree_list_row
                .item()
                .and_downcast::<PropertiesSelectorObject>()
                .unwrap();

            if property_object.unit_property().is_empty() {
                single_selection.set_selected(gtk::INVALID_LIST_POSITION);
                warn!("Cant select interface  {property_object:?}");
                return;
            }

            info!("Select {property_object:?}");

            let interface = tree_list_row
                .parent()
                .expect("has a parent")
                .item()
                .and_downcast::<PropertiesSelectorObject>()
                .unwrap();

            let new_property_object =
                PropertiesSelectorObject::from_parent(interface, property_object);

            unit_properties_selection.add_new_property(new_property_object);
        });

        glib::spawn_future_local(async move {
            let (sender, receiver) = tokio::sync::oneshot::channel();

            runtime().spawn(async move {
                match systemd::fetch_unit_interface_properties().await {
                    Ok(map) => sender.send(map).expect("The channel needs to be open."),
                    Err(err) => error!("Fetch unir properties {err:?}"),
                }
            });

            let unit_properties_map = receiver
                .await
                .map_err(|e| error!("Receiver {e:?}"))
                .expect("Tokio receiver works");

            for (inteface, mut properties) in unit_properties_map
                .into_iter()
                .filter(|(k, _)| k.starts_with("org.freedesktop.systemd1"))
            {
                let obj = PropertiesSelectorObject::new_interface(inteface);
                properties.sort();
                for property in properties {
                    let prop_object = PropertiesSelectorObject::from(property);
                    obj.add_child(prop_object);
                }

                store.append(&obj);
            }
        });
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

    let tree_list_row = item.item().unwrap().downcast::<gtk::TreeListRow>().unwrap();
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

impl WindowImpl for UnitPropertiesSelectorDialogImp {
    // Save window state right before the window will be closed
    fn close_request(&self) -> glib::Propagation {
        // Save window size
        debug!("Close window");
        if let Err(_err) = self.save_window_context() {
            error!("Failed to save window state");
        }

        self.parent_close_request();
        // Allow to invoke other event handlers
        glib::Propagation::Proceed
    }
}

impl AdwWindowImpl for UnitPropertiesSelectorDialogImp {}

#[cfg(test)]
mod test {
    #[test]
    fn test_last() {
        let var = "org.freedesktop.systemd1.some_stuff";

        let token = var.split('.').next_back();

        assert_eq!(token, Some("some_stuff"))
    }
}
