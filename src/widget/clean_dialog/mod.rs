mod imp;

use gtk::{
    glib::{self},
    subclass::prelude::ObjectSubclassIsExt,
};

use crate::systemd::{data::UnitInfo, enums::CleanOption};

use super::{InterPanelMessage, app_window::AppWindow, unit_control_panel::UnitControlPanel};

// ANCHOR: mod
glib::wrapper! {
    pub struct CleanUnitDialog(ObjectSubclass<imp::CleanDialogImp>)
    @extends adw::Window, gtk::Window, gtk::Widget,
    @implements gtk::Accessible,  gtk::Buildable,  gtk::ConstraintTarget,
    gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl CleanUnitDialog {
    pub fn new(
        unit: Option<&UnitInfo>,
        is_dark: bool,
        app_window: Option<&AppWindow>,
        unit_control: &UnitControlPanel,
    ) -> Self {
        let obj: CleanUnitDialog = glib::Object::new();
        let imp = obj.imp();
        imp.set_app_window(app_window, unit_control);

        imp.set_unit(unit);
        imp.set_inter_message(&InterPanelMessage::IsDark(is_dark));

        obj
    }

    fn clean_option_selected(&self, clean_option: &CleanOption, is_active: bool) {
        self.imp().clean_option_selected(clean_option, is_active)
    }
}
