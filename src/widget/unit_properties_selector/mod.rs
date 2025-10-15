pub mod data_browser;
pub mod data_selection;
mod imp;
pub mod save;
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
    pub fn new(unit_list_panel: &UnitListPanel, column_id: Option<String>) -> Self {
        let obj: UnitPropertiesSelectorDialog = glib::Object::new();
        obj.set_unit_list(unit_list_panel, column_id);
        obj
    }

    fn set_unit_list(&self, unit_list_panel: &UnitListPanel, column_id: Option<String>) {
        self.imp().set_unit_list(unit_list_panel, column_id);
    }
}
