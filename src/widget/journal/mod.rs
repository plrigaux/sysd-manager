mod colorise;
mod imp;
pub mod list_boots;
//mod journal_row;
//pub mod more_colors;
//pub mod palette;

use std::cell::RefCell;

use gtk::{glib, subclass::prelude::ObjectSubclassIsExt};

use crate::systemd::journal_data::JournalEventChunk;

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
}

impl Default for JournalPanel {
    fn default() -> Self {
        JournalPanel::new()
    }
}

// global variable to store  the ui and an input channel
// on the main thread only
thread_local!(
    static GLOBAL: RefCell<Option<(JournalPanel, std::sync::mpsc::Receiver<JournalEventChunk>)>> =  const {RefCell::new(None)}
);

pub fn check_for_new_journal_entry() {
    GLOBAL.with(
        |global: &RefCell<Option<(JournalPanel, std::sync::mpsc::Receiver<JournalEventChunk>)>>| {
            if let Some((journal_panel, rx)) = &*global.borrow() {
                match rx.recv() {
                    Ok(journal_events) => {
                        log::info!(
                            "New journal_events info: {:?} len {:?}",
                            journal_events.info(),
                            journal_events.len()
                        );
                        journal_panel.imp().append_journal_event(journal_events);
                    }
                    Err(error) => {
                        log::warn!("Journal recv Error: {error}");
                    }
                }
            }
        },
    );
}
