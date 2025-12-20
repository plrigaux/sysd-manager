mod imp;

use gtk::{
    glib::{self},
    subclass::prelude::ObjectSubclassIsExt,
};

use super::unit_control_panel::UnitControlPanel;

// ANCHOR: mod
glib::wrapper! {
    pub struct CleanUnitDialog(ObjectSubclass<imp::CleanDialogImp>)
    @extends adw::Window, gtk::Window, gtk::Widget,
    @implements gtk::Accessible,  gtk::Buildable,  gtk::ConstraintTarget,
    gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl CleanUnitDialog {
    pub fn new(unit_control_panel: &UnitControlPanel) -> Self {
        let obj: CleanUnitDialog = glib::Object::new();
        let imp = obj.imp();
        imp.set_unit_control_panel(unit_control_panel);

        obj
    }
}
