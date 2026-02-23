use std::cell::OnceCell;

use gio::glib::object::Cast;
use gtk::{
    ListScrollFlags,
    glib::{self},
    prelude::*,
    subclass::prelude::*,
};
use tracing::{error, info, warn};

use crate::widget::{
    unit_list::{UnitListPanel, UnitCuratedList},
    unit_properties_selector::{
        data_browser::PropertyBrowseItem,
        data_selection::UnitPropertySelection,
        save::{self, save_column_config},
        unit_properties_selection::row::UnitPropertiesSelectionRow,
    },
};

use super::UnitPropertiesSelectionPanel;

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/unit_properties_selection.ui")]
pub struct UnitPropertiesSelectionPanelImp {
    #[template_child]
    properties_selection: TemplateChild<gtk::ListView>,

    #[template_child]
    apply_button: TemplateChild<gtk::Button>,

    #[template_child]
    ok_button: TemplateChild<gtk::Button>,

    #[template_child]
    column_nb: TemplateChild<gtk::Label>,

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

macro_rules! get_unit_list_panel {
    ($self:expr) => {{
        let Some(list_store) = $self.unit_list_panel.get() else {
            error!("No unit list panel");
            return;
        };
        list_store
    }};
}

#[gtk::template_callbacks]
impl UnitPropertiesSelectionPanelImp {
    #[template_callback]
    fn apply_clicked(&self, _button: &gtk::Button) {
        info!("Apply pressed");

        let list_store = get_list_store!(self);

        let mut list: Vec<UnitPropertySelection> = list_store
            .iter::<UnitPropertySelection>()
            .filter_map(|result| result.ok())
            .collect();

        save_column_config(None, &mut list, UnitCuratedList::Custom);

        let unit_list_panel = get_unit_list_panel!(self);

        unit_list_panel.set_new_columns(list);
    }

    #[template_callback]
    fn ok_clicked(&self, button: &gtk::Button) {
        info!("Ok pressed");

        self.apply_clicked(button);

        if let Err(boolerror) = button.activate_action("window.close", None) {
            warn!("bool error {boolerror}")
        };
    }

    #[template_callback]
    fn reset_clicked(&self, _button: &gtk::Button) {
        let Some(unit_list_panel) = self.unit_list_panel.get() else {
            error!("unit_list_panel is None");
            return;
        };

        let list_store = get_list_store!(self);

        list_store.remove_all(); //TBSafe

        for unit_property_column in unit_list_panel.default_displayed_columns().iter() {
            let unit_property_column = unit_property_column.copy();
            list_store.append(&unit_property_column);
        }
    }
}

impl UnitPropertiesSelectionPanelImp {
    pub fn add_new_property(&self, new_property_object: PropertyBrowseItem) {
        let new_unit_prop = UnitPropertySelection::from_browser(new_property_object);

        let list_store = get_list_store!(self);
        list_store.append(&new_unit_prop);
        self.properties_selection.scroll_to(
            list_store.n_items() - 1,
            gtk::ListScrollFlags::FOCUS,
            None,
        );
    }

    pub(super) fn set_unit_list_panel(
        &self,
        unit_list_panel: &UnitListPanel,
        column_id: Option<String>,
    ) {
        self.unit_list_panel
            .set(unit_list_panel.clone())
            .expect("Assigned only once");

        let list_store = get_list_store!(self);

        let mut columns = { unit_list_panel.current_columns_mut() };
        save::order_columns(Some(&unit_list_panel.columns()), &mut columns);

        for unit_property_column in columns.iter() {
            let unit_property_column = unit_property_column.copy(); //if no copy the column prop are in sync
            list_store.append(&unit_property_column);
        }

        if let Some(column_id) = column_id {
            for (index, id) in list_store
                .iter::<UnitPropertySelection>()
                .enumerate()
                .filter_map(|r| {
                    if let Ok(item) = r.1
                        && let Some(id) = item.id()
                    {
                        Some((r.0, id))
                    } else {
                        None
                    }
                })
            {
                if column_id == id {
                    self.properties_selection.scroll_to(
                        index as u32,
                        ListScrollFlags::FOCUS | ListScrollFlags::SELECT,
                        None,
                    );
                }
            }
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
impl ObjectSubclass for UnitPropertiesSelectionPanelImp {
    const NAME: &'static str = "UnitPropertiesSelectionPanel";
    type Type = UnitPropertiesSelectionPanel;
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

impl ObjectImpl for UnitPropertiesSelectionPanelImp {
    fn constructed(&self) {
        self.parent_constructed();

        let list_store = gio::ListStore::new::<UnitPropertySelection>();

        self.list_store.set(list_store.clone()).expect("Only once");

        //let selection_model = gtk::NoSelection::new(Some(store.clone()));

        let selection_model = gtk::SingleSelection::builder()
            .can_unselect(true)
            .model(&list_store.clone())
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

        factory.connect_unbind(move |_factory, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();

            let child = item
                .child()
                .and_downcast::<UnitPropertiesSelectionRow>()
                .unwrap();

            child.unbind();
        });

        self.properties_selection.set_factory(Some(&factory));

        list_store
            .bind_property::<gtk::Button>("n-items", self.apply_button.as_ref(), "sensitive")
            .transform_to(|_bond, nitems: u32| Some(nitems > 0))
            .build();

        list_store
            .bind_property::<gtk::Button>("n-items", self.ok_button.as_ref(), "sensitive")
            .transform_to(|_bond, nitems: u32| Some(nitems > 0))
            .build();

        list_store
            .bind_property::<gtk::Label>("n-items", self.column_nb.as_ref(), "label")
            .transform_to(|_bond, nitems: u32| Some(nitems.to_string()))
            .build();
    }
}

impl WidgetImpl for UnitPropertiesSelectionPanelImp {}
impl BoxImpl for UnitPropertiesSelectionPanelImp {}
