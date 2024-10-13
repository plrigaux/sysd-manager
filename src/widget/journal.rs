use crate::systemd::{self, data::UnitInfo};
use gtk::prelude::*;
mod colorise;
mod more_colors;

/// Updates the associated journal `TextView` with the contents of the unit's journal log.
pub fn update_journal(journal: &gtk::TextView, unit: &UnitInfo) {
    let in_color = false;
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
    buf.set_text("");

    let color = journal.color();
    println!("color {:?}", color);

    let css = journal.css_classes();
    println!("css_classes {:?}", css);

    let css_name = journal.css_name();
    println!("{:?}", css_name);



    let sm = adw::StyleManager::default();
    println!("color_scheme {:?}", sm.color_scheme());
    println!("is_dark {:?}", sm.is_dark());

    if in_color {
        let mut start_iter = buf.start_iter();
        let text = colorise::convert_to_mackup(&text);
        buf.insert_markup(&mut start_iter, &text);
    } else {
        buf.set_text(&text);
    }
}
