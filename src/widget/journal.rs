use crate::systemd::{self, data::UnitInfo};
use gtk::prelude::*;
mod colorise;
/// Updates the associated journal `TextView` with the contents of the unit's journal log.
pub fn update_journal(journal: &gtk::TextView, unit: &UnitInfo) {
    let text = match systemd::get_unit_journal(unit, true) {
        Ok(journal_output) => journal_output,
        Err(error) => {
            let text = match error.gui_description() {
                Some(s) => s.clone(),
                None => String::from(""),
            };
            text
        }
    };

    let buf = journal.buffer();
    buf.set_text("");
    let mut start_iter = buf.start_iter();

    buf.insert_markup(&mut start_iter, &text);
}
