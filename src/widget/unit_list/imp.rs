
use std::cell::{Cell, RefCell};

use gtk::{
    glib,
    gio,
    prelude::*,
    subclass::{
        box_::BoxImpl,
        prelude::*,
        widget::{
            CompositeTemplateCallbacksClass, CompositeTemplateClass,
            CompositeTemplateInitializingExt, WidgetClassExt, WidgetImpl,
        },
    },
    TemplateChild,
};

use log::{info, warn};

use crate::{
    systemd::{self, data::UnitInfo},
    widget::{button_icon::ButtonIcon, preferences::data::PREFERENCES},
};

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/unit_list_panel.ui")]
pub struct UnitListPanelImp {
    #[template_child]
    list_store : TemplateChild<gio::ListStore>,
}

#[gtk::template_callbacks]
impl UnitListPanelImp {

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