mod imp;

use super::unit_control_panel::UnitControlPanel;
use gtk::{
    glib::{self},
    subclass::prelude::ObjectSubclassIsExt,
};

glib::wrapper! {
    pub struct SystemdManagerDialog(ObjectSubclass<imp::SystemdManagerDialogImp>)
    @extends adw::Window, gtk::Window, gtk::Widget,
    @implements gtk::Accessible,  gtk::Buildable,  gtk::ConstraintTarget,
    gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl SystemdManagerDialog {
    pub fn new() -> Self {
        let obj: SystemdManagerDialog = glib::Object::new();
        // let imp = obj.imp();

        obj
    }
}
