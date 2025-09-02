use adw::subclass::window::AdwWindowImpl;
use gtk::{
    glib::{self},
    subclass::{
        prelude::*,
        widget::{
            CompositeTemplateCallbacksClass, CompositeTemplateClass,
            CompositeTemplateInitializingExt, WidgetImpl,
        },
    },
};

use super::UnitPropertiesSelectorDialog;

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/unit_properties_selector.ui")]
pub struct UnitPropertiesSelectorDialogImp {}

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
    }
}

impl WidgetImpl for UnitPropertiesSelectorDialogImp {}
impl WindowImpl for UnitPropertiesSelectorDialogImp {}
impl AdwWindowImpl for UnitPropertiesSelectorDialogImp {}
