mod imp;
use gtk::glib::{self};

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
}

impl Default for UnitPropertiesSelection {
    fn default() -> Self {
        Self::new()
    }
}
