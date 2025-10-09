mod imp;
mod row;
mod save;

use crate::widget::{
    unit_list::UnitListPanel, unit_properties_selector::data_browser::PropertyBrowseItem,
};
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

    pub fn add_new_property(&self, new_property_object: PropertyBrowseItem) {
        self.imp().add_new_property(new_property_object);
    }

    pub fn set_unit_list(&self, unit_list_panel: &UnitListPanel) {
        self.imp().set_unit_list_panel(unit_list_panel);
    }

    pub fn list_store(&self) -> Option<&gio::ListStore> {
        self.imp().get_list_store()
    }

    pub fn move_up(&self, position: u32) {
        self.imp().move_up(position)
    }

    pub fn move_down(&self, position: u32) {
        self.imp().move_down(position)
    }

    pub fn delete(&self, position: u32) {
        self.imp().delete(position)
    }
}

impl Default for UnitPropertiesSelection {
    fn default() -> Self {
        Self::new()
    }
}
