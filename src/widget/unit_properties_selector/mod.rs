mod data;
mod imp;
mod unit_properties_selection;

use adw::subclass::prelude::ObjectSubclassIsExt;
use gtk::glib::{self};

use crate::widget::unit_list::UnitListPanel;

glib::wrapper! {
    pub struct UnitPropertiesSelectorDialog(ObjectSubclass<imp::UnitPropertiesSelectorDialogImp>)
    @extends adw::Window, gtk::Window, gtk::Widget,
    @implements gtk::Accessible,  gtk::Buildable,  gtk::ConstraintTarget,
    gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl UnitPropertiesSelectorDialog {
    pub fn new() -> Self {
        let obj: UnitPropertiesSelectorDialog = glib::Object::new();
        obj
    }

    pub fn set_unit_list(&self, unit_list_panel: &UnitListPanel) {
        self.imp().set_unit_list(unit_list_panel);
    }
}

impl Default for UnitPropertiesSelectorDialog {
    fn default() -> Self {
        Self::new()
    }
}
