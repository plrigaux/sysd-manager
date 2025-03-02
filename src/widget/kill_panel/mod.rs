mod imp;
use gtk::{
    glib::{self},
    subclass::prelude::ObjectSubclassIsExt,
};

use crate::systemd::data::UnitInfo;

use super::{unit_control_panel::UnitControlPanel, InterPanelAction};

// ANCHOR: mod
glib::wrapper! {
    pub struct KillPanel(ObjectSubclass<imp::KillPanelImp>)
        @extends adw::Window, gtk::Window, gtk::Widget,
        //@implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;

        @implements gtk::Accessible,  gtk::Buildable,  gtk::ConstraintTarget,
        gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl KillPanel {
    pub fn new_kill_window(
        unit: Option<&UnitInfo>,
        is_dark: bool,
        parent: &UnitControlPanel,
    ) -> Self {
        KillPanel::new_window(unit, is_dark, false, parent)
    }

    pub fn new_signal_window(
        unit: Option<&UnitInfo>,
        is_dark: bool,
        parent: &UnitControlPanel,
    ) -> Self {
        KillPanel::new_window(unit, is_dark, true, parent)
    }

    fn new_window(
        unit: Option<&UnitInfo>,
        is_dark: bool,
        is_signal: bool,
        parent: &UnitControlPanel,
    ) -> KillPanel {
        let obj: KillPanel = glib::Object::new();
        obj.set_unit(unit);
        obj.set_inter_action(&InterPanelAction::IsDark(is_dark));
        let imp = obj.imp();
        imp.set_is_signal(is_signal);
        imp.set_parent(parent);
        obj
    }

    pub fn set_unit(&self, unit: Option<&UnitInfo>) {
        self.imp().set_unit(unit);
    }

    pub fn set_inter_action(&self, action: &InterPanelAction) {
        self.imp().set_inter_action(action);
    }

    pub fn register(
        &self,
        side_overlay: &adw::OverlaySplitView,
        toast_overlay: &adw::ToastOverlay,
    ) {
        let obj = self.imp();
        obj.register(side_overlay, toast_overlay);
    }
}
