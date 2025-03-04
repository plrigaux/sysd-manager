mod imp;

use gtk::{
    glib::{self},
    subclass::prelude::ObjectSubclassIsExt,
};

use crate::systemd::data::UnitInfo;

use super::InterPanelAction;

// ANCHOR: mod
glib::wrapper! {
    pub struct CleanDialog(ObjectSubclass<imp::CleanDialogImp>)
    @extends adw::Window, gtk::Window, gtk::Widget,
    @implements gtk::Accessible,  gtk::Buildable,  gtk::ConstraintTarget,
    gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl CleanDialog {
    pub fn new(unit: Option<&UnitInfo>, is_dark: bool) -> Self {
        let obj: CleanDialog = glib::Object::new();
        let imp = obj.imp();
        imp.set_unit(unit);
        imp.set_inter_action(&InterPanelAction::IsDark(is_dark));

        obj
    }

    /*     pub fn set_unit(&self, unit: Option<&UnitInfo>) {
        self.imp().set_unit(unit);
    }

    pub fn set_inter_action(&self, action: &InterPanelAction) {
        self.imp().set_inter_action(action);
    } */
}
