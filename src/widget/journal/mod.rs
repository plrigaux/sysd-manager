//mod colorise;
mod imp;
//mod journal_row;
//pub mod more_colors;
//pub mod palette;

use gtk::{glib, subclass::prelude::ObjectSubclassIsExt};

use super::InterPanelMessage;

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

    pub fn refresh_panels(&self) {
        self.imp().refresh_panels();
    }

    pub fn set_inter_message(&self, action: &InterPanelMessage) {
        self.imp().set_inter_message(action);
    }

    fn set_from_time(&self, from_time: Option<u64>) {
        self.imp().set_from_time(from_time);
    }

    fn set_most_recent_time(&self, time: u64) {
        self.imp().set_oldest(time);
    }
}

impl Default for JournalPanel {
    fn default() -> Self {
        JournalPanel::new()
    }
}
