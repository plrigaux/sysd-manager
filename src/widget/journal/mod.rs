use crate::systemd::data::UnitInfo;

//mod colorise;
mod imp;
pub mod more_colors;
pub mod palette;

use gtk::{glib, subclass::prelude::ObjectSubclassIsExt};

// ANCHOR: mod
glib::wrapper! {
    pub struct JournalPanel(ObjectSubclass<imp::JournalPanelImp>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl JournalPanel {
    pub fn new() -> Self {
        // Create new window
        let obj: JournalPanel = glib::Object::new();

        obj
    }

    pub fn display_journal(&self, unit: &UnitInfo) {
        self.imp().display_journal(unit);
    }

    pub fn set_dark(&self, is_dark: bool) {
        self.imp().set_dark(is_dark)
    }

    fn set_boot_id_style(&self) {
        self.imp().set_boot_id_style();
    }
}

