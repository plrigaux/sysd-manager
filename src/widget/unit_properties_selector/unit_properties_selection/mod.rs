mod imp;
use crate::widget::unit_properties_selector::data::PropertiesSelectorObject;
use gtk::{
    glib::{self},
    subclass::prelude::*,
};
glib::wrapper! {
    pub struct UnitPropertiesSelection(ObjectSubclass<imp::UnitPropertiesSelectionImp>)
    @extends gtk::Box, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl UnitPropertiesSelection {
    pub fn new() -> Self {
        let obj: UnitPropertiesSelection = glib::Object::new();
        obj
    }

    pub fn add_new_property(&self, new_property_object: PropertiesSelectorObject) {
        self.imp().add_new_property(new_property_object);
    }
}

impl Default for UnitPropertiesSelection {
    fn default() -> Self {
        Self::new()
    }
}
