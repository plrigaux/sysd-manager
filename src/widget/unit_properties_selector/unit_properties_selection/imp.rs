use gtk::{
    glib::{self},
    subclass::prelude::*,
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
}

impl UnitPropertiesSelectionImp {}

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
    }
}

impl WidgetImpl for UnitPropertiesSelectionImp {}
impl BoxImpl for UnitPropertiesSelectionImp {}
