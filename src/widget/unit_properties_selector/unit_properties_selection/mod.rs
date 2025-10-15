mod imp;
mod row;

use crate::widget::{
    unit_list::UnitListPanel, unit_properties_selector::data_browser::PropertyBrowseItem,
};
use gtk::{
    glib::{self},
    subclass::prelude::*,
};
glib::wrapper! {
    pub struct UnitPropertiesSelectionPanel(ObjectSubclass<imp::UnitPropertiesSelectionPanelImp>)
    @extends gtk::Box, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl UnitPropertiesSelectionPanel {
    pub fn new() -> Self {
        let obj: UnitPropertiesSelectionPanel = glib::Object::new();
        obj
    }

    pub fn add_new_property(&self, new_property_object: PropertyBrowseItem) {
        self.imp().add_new_property(new_property_object);
    }

    pub fn set_unit_list(&self, unit_list_panel: &UnitListPanel, column_id: Option<String>) {
        self.imp().set_unit_list_panel(unit_list_panel, column_id);
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

impl Default for UnitPropertiesSelectionPanel {
    fn default() -> Self {
        Self::new()
    }
}
