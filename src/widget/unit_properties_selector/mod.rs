mod data;
mod imp;
use gtk::glib::{self};

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
}

impl Default for UnitPropertiesSelectorDialog {
    fn default() -> Self {
        Self::new()
    }
}
