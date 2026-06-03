pub mod app_window;
pub mod clean_dialog;
pub mod control_action_dialog;
pub mod creator;
pub mod grid_cell;
pub mod info_window;
pub mod journal;
pub mod kill_panel;
pub mod menu_button;
pub mod preferences;
pub mod signals_dialog;
pub mod text_search;
pub mod unit_control_panel;
pub mod unit_dependencies_panel;
pub mod unit_file_panel;
pub mod unit_info;
pub mod unit_list;
pub mod unit_properties_selector;

use crate::{
    format2,
    systemd::{BootFilter, data::UnitInfo},
    utils::palette::{blue, green, red},
};
use adw::prelude::AdwDialogExt;
use base::consts::{FAVORITE_ICON_FILLED, FAVORITE_ICON_OUTLINE};
use gettextrs::pgettext;
use glib::object::{Cast, CastNone, IsA};
use gtk::{gdk, pango::FontDescription, prelude::*};
use regex::Regex;
pub(crate) use std::{rc::Rc, sync::OnceLock};
use tracing::debug;

pub enum InterPanelMessage<'a> {
    Font(Option<&'a FontDescription>),
    FontProvider(Option<&'a gtk::CssProvider>, Option<&'a gtk::CssProvider>),
    IsDark(bool),
    PanelVisible(bool),
    NewStyleScheme(Option<&'a str>),
    FileLineNumber(bool),
    UnitChange(Option<&'a UnitInfo>),
    Refresh(Option<&'a UnitInfo>),
    JournalFilterBoot(BootFilter),
    EnableUnit(UnitInfo, Rc<Box<dyn Fn()>>),
    DisableUnit(UnitInfo, Rc<Box<dyn Fn()>>),
    ReenableUnit(UnitInfo, Rc<Box<dyn Fn()>>),
    MaskUnit(&'a gtk::Button, &'a UnitInfo),
    UnMaskUnit(&'a gtk::Button, &'a UnitInfo),
}

pub fn set_favorite_info(is_favorite: bool, unit: &Option<UnitInfo>) -> (&str, String) {
    let (favorite_icon, tooltip) = if is_favorite {
        (
            FAVORITE_ICON_FILLED,
            //tooltip for action button
            pgettext("controls", "Remove {} from Favorites"),
        )
    } else {
        (
            FAVORITE_ICON_OUTLINE,
            //tooltip for action button
            pgettext("controls", "Add {} to Favorites"),
        )
    };

    let unit_txt = format!(
        "<unit>{}</unit>",
        unit.as_ref()
            .map(|u| u.primary())
            .unwrap_or("Unit".to_owned())
    );

    let unit_txt = replace_tags(&unit_txt);
    let tooltip = format2!(tooltip, unit_txt);
    (favorite_icon, tooltip)
}

pub fn toast_regex() -> &'static Regex {
    static TOAST_REGEX: OnceLock<Regex> = OnceLock::new();
    TOAST_REGEX
        .get_or_init(|| Regex::new(r"<(\w+).*?>(.*?)</(\w+?)>").expect("Rexgex compile error"))
}

pub fn replace_tags(message: &str) -> String {
    debug!("{message}");
    let mut out = String::with_capacity(message.len() * 2);
    let re = toast_regex();

    let mut i: usize = 0;
    for capture in re.captures_iter(message) {
        let m = capture.get(0).unwrap();
        out.push_str(&message[i..m.start()]);

        let tag = &capture[1];
        match tag {
            "unit" => {
                tag_unit(&mut out, &capture[2]);
            }

            "red" => {
                out.push_str("<span fgcolor='");
                out.push_str(red().get_color());
                out.push_str("'>");
                out.push_str(&capture[2]);
                out.push_str("</span>");
            }

            "green" => {
                out.push_str("<span fgcolor='");
                out.push_str(green().get_color());
                out.push_str("'>");
                out.push_str(&capture[2]);
                out.push_str("</span>");
            }
            _ => {
                out.push_str(&capture[0]);
            }
        }
        i = m.end();
    }
    out.push_str(&message[i..message.len()]);
    debug!("{out}");
    out
}

fn tag_unit(out: &mut String, unit_name: &str) {
    out.push_str("<span fgcolor='");
    out.push_str(blue().get_color());
    out.push_str("' font_family='monospace' size='larger' weight='bold'>");
    out.push_str(unit_name);
    out.push_str("</span>");
}

pub fn highlight_unit_text(unit_name: &str) -> String {
    let mut out = String::new();
    tag_unit(&mut out, unit_name);
    out
}

pub fn close_window_shortcut(window: &impl IsA<gtk::Widget>) {
    let shortcut_controller = gtk::ShortcutController::new();
    let trigger = gtk::ShortcutTrigger::parse_string("<Ctrl>w");
    let trigger_esc = gtk::ShortcutTrigger::parse_string("Escape");
    let action = gtk::CallbackAction::new(|widget, _| {
        if let Some(window) = widget.downcast_ref::<adw::Window>() {
            window.close();
        } else if let Some(dialog) = widget.downcast_ref::<adw::Dialog>() {
            dialog.close();
        }
        glib::Propagation::Proceed
    });

    let shortcut = gtk::Shortcut::new(trigger, Some(action.clone()));
    shortcut_controller.add_shortcut(shortcut);
    let shortcut = gtk::Shortcut::new(trigger_esc, Some(action));
    shortcut_controller.add_shortcut(shortcut);
    window.add_controller(shortcut_controller);
}

pub fn clear_on_escape() -> gtk::EventControllerKey {
    let event_controller = gtk::EventControllerKey::new();

    event_controller.connect_key_released(|controller, key, _keycode, _state| {
        if key == gdk::Key::Escape
            && let Some(search_entry) = controller.widget().and_downcast_ref::<gtk::Editable>()
        {
            search_entry.set_text("");
        }
    });

    event_controller
}

pub fn clear_on_escape2() -> gtk::EventControllerKey {
    let event_controller = gtk::EventControllerKey::new();

    event_controller.connect_key_released(|controller, key, _keycode, _state| {
        if key == gdk::Key::Escape {
            //TODO
            if let Some(search_entry) = controller.widget().and_downcast_ref::<adw::EntryRow>() {
                search_entry.set_text("");
            }
        }
    });

    event_controller
}

pub fn grab_focus_on_search_entry(search_entry: &gtk::SearchEntry) {
    search_entry.select_region(0, -1);
    search_entry.grab_focus();
}
