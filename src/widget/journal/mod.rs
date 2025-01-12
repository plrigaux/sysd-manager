//mod colorise;
mod imp;
mod journal_row;
pub mod more_colors;
pub mod palette;

use gtk::{glib, subclass::prelude::ObjectSubclassIsExt};

glib::wrapper! {
    pub struct JournalPanel(ObjectSubclass<imp::JournalPanelImp>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl JournalPanel {
    pub fn new() -> Self {
        let obj: JournalPanel = glib::Object::new();
        obj
    }

    fn set_boot_id_style(&self) {
        self.imp().set_boot_id_style();
    }
}

impl Default for JournalPanel {
    fn default() -> Self {
        JournalPanel::new()
    }
}