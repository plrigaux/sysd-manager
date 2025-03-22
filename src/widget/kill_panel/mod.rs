mod imp;
use gtk::{
    glib::{self},
    subclass::prelude::ObjectSubclassIsExt,
};

use crate::systemd::data::UnitInfo;

use super::{InterPanelMessage, unit_control_panel::side_control_panel::SideControlPanel};

// ANCHOR: mod
glib::wrapper! {
    pub struct KillPanel(ObjectSubclass<imp::KillPanelImp>)
        @extends adw::Window, gtk::Window, gtk::Widget,
        @implements gtk::Accessible,  gtk::Buildable,  gtk::ConstraintTarget,
        gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl KillPanel {
    pub fn new_kill_window(
        unit: Option<&UnitInfo>,
        is_dark: bool,
        parent: &SideControlPanel,
    ) -> Self {
        KillPanel::new_window(unit, is_dark, false, parent)
    }

    pub fn new_signal_window(
        unit: Option<&UnitInfo>,
        is_dark: bool,
        parent: &SideControlPanel,
    ) -> Self {
        KillPanel::new_window(unit, is_dark, true, parent)
    }

    fn new_window(
        unit: Option<&UnitInfo>,
        is_dark: bool,
        is_signal: bool,
        parent: &SideControlPanel,
    ) -> KillPanel {
        let obj: KillPanel = glib::Object::new();
        let imp = obj.imp();
        imp.set_unit(unit);
        imp.set_inter_message(&InterPanelMessage::IsDark(is_dark));
        imp.set_is_signal(is_signal);
        imp.set_parent(parent);

        obj
    }

    pub fn set_inter_message(&self, action: &InterPanelMessage) {
        self.imp().set_inter_message(action);
    }
}
