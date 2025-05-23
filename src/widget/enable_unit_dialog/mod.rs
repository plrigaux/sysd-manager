mod imp;

use gtk::{
    glib::{self},
    subclass::prelude::ObjectSubclassIsExt,
};

use super::{InterPanelMessage, app_window::AppWindow, unit_control_panel::UnitControlPanel};

// ANCHOR: mod
glib::wrapper! {
    pub struct EnableUnitDialog(ObjectSubclass<imp::EnableUnitDialogImp>)
    @extends adw::Window, gtk::Window, gtk::Widget,
    @implements gtk::Accessible,  gtk::Buildable,  gtk::ConstraintTarget,
    gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl EnableUnitDialog {
    pub fn new(
        is_dark: bool,
        app_window: Option<&AppWindow>,
        unit_control: &UnitControlPanel,
    ) -> Self {
        let obj: EnableUnitDialog = glib::Object::new();
        let imp = obj.imp();
        imp.set_app_window(app_window, unit_control);

        imp.set_inter_message(&InterPanelMessage::IsDark(is_dark));

        obj
    }
}
