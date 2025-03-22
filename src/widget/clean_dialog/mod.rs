mod imp;

use gtk::{
    glib::{self},
    subclass::prelude::ObjectSubclassIsExt,
};

use crate::systemd::{data::UnitInfo, enums::CleanOption};

use super::{InterPanelMessage, app_window::AppWindow};

// ANCHOR: mod
glib::wrapper! {
    pub struct CleanDialog(ObjectSubclass<imp::CleanDialogImp>)
    @extends adw::Window, gtk::Window, gtk::Widget,
    @implements gtk::Accessible,  gtk::Buildable,  gtk::ConstraintTarget,
    gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl CleanDialog {
    pub fn new(unit: Option<&UnitInfo>, is_dark: bool, app_window: Option<&AppWindow>) -> Self {
        let obj: CleanDialog = glib::Object::new();
        let imp = obj.imp();
        imp.set_app_window(app_window);
        imp.set_unit(unit);
        imp.set_inter_message(&InterPanelMessage::IsDark(is_dark));

        obj
    }

    fn clean_option_selected(&self, clean_option: &CleanOption, is_active: bool) {
        self.imp().clean_option_selected(clean_option, is_active)
    }
}
