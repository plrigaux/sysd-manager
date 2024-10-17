use crate::systemd::{self, data::UnitInfo};

use gtk::prelude::*;
use more_colors::TermColor;

use super::preferences::data::PREFERENCES;
mod colorise;
pub mod more_colors;

/// Updates the associated journal `TextView` with the contents of the unit's journal log.
pub fn update_journal(journal: &gtk::TextView, unit: &UnitInfo) {
    let in_color = PREFERENCES.journal_colors();

    let text = match systemd::get_unit_journal(unit, in_color) {
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
    if in_color {
        let mut start_iter = buf.start_iter();
        let journal_color: TermColor = journal.color().into();
        let text = colorise::convert_to_mackup(&text, &journal_color);
        buf.insert_markup(&mut start_iter, &text);
    } else {
        buf.set_text(&text);
    }
}
