use glib::WeakRef;
use gtk::{glib, prelude::*, subclass::prelude::*};
use regex::Regex;
use tracing::{debug, warn};

use crate::upgrade;

use super::TextSearchBar;

const SEARCH_HIGHLIGHT: &str = "search_highlight";

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/text_find.ui")]
pub struct TextSearchBarImp {
    #[template_child]
    search_entry: TemplateChild<gtk::SearchEntry>,

    #[template_child]
    case_sensitive_toggle_button: TemplateChild<gtk::ToggleButton>,

    #[template_child]
    regex_toggle_button: TemplateChild<gtk::ToggleButton>,

    #[template_child]
    previous_match_button: TemplateChild<gtk::Button>,

    #[template_child]
    next_match_button: TemplateChild<gtk::Button>,

    #[template_child]
    search_result_label: TemplateChild<gtk::Label>,

    text_view: WeakRef<gtk::TextView>,
}

#[gtk::template_callbacks]
impl TextSearchBarImp {
    #[template_callback]
    fn search_entry_changed(&self, search_entry: &gtk::SearchEntry) {
        let entry_text: glib::GString = search_entry.text();

        debug!("Search text changed: {}", entry_text);

        self.highlight_text();
    }

    #[template_callback]
    fn on_case_sensitive_toggled(&self, _toggle_button: &gtk::ToggleButton) {
        self.highlight_text();
    }
    #[template_callback]
    fn on_regex_toggled(&self, _toggle_button: &gtk::ToggleButton) {
        self.highlight_text();
    }

    #[template_callback]
    fn on_previous_match_clicked(&self, _button: &gtk::Button) {}

    #[template_callback]
    fn on_next_match_clicked(&self, _button: &gtk::Button) {
        /*  if let Some(text_view) = self.text_view.upgrade() {
            //  text_view.search_next();
        } */
    }
}

impl TextSearchBarImp {
    pub(crate) fn set_text_view(&self, text_view: &gtk::TextView) {
        self.text_view.set(Some(text_view));
    }

    pub(crate) fn grab_focus_on_search_entry(&self) {
        self.search_entry.grab_focus();
    }

    fn highlight_text(&self) {
        let entry_text = self.search_entry.text();
        let text_view = upgrade!(self.text_view);

        let buff = text_view.buffer();

        let start = buff.start_iter();
        let end = buff.end_iter();

        let tag_table = buff.tag_table();

        let tag = if let Some(tag) = tag_table.lookup(SEARCH_HIGHLIGHT) {
            // Remove previous highlights
            buff.remove_tag(&tag, &start, &end);
            tag
        } else {
            let tag = gtk::TextTag::builder()
                .name(SEARCH_HIGHLIGHT)
                .background("yellow")
                .build();

            tag_table.add(&tag);
            tag
        };

        if entry_text.is_empty() {
            return;
        }

        let text = buff.text(&start, &end, true);

        let regex = if self.case_sensitive_toggle_button.is_active() {
            entry_text.to_string()
        } else {
            format!("(?i){}", entry_text)
        };

        let re = match Regex::new(&regex) {
            Ok(re) => re,
            Err(err) => {
                warn!("Invalid regex: {}", err);
                return;
            }
        };

        //start.forward_search(str, flags, limit)
        let mut char_start: i32 = 0;
        let mut byte_start = 0;

        let mut match_num = 0;
        for re_match in re.find_iter(&text) {
            let match_start = re_match.start();
            char_start += text[byte_start..match_start].chars().count() as i32;
            let re_match_end = re_match.end();
            let char_end = char_start + text[match_start..re_match_end].chars().count() as i32;

            let match_start = buff.iter_at_offset(char_start);
            let match_end = buff.iter_at_offset(char_end);

            buff.apply_tag(&tag, &match_start, &match_end);

            byte_start = re_match_end;
            char_start = char_end;
            match_num += 1;
        }

        let hints = format!("0 of {match_num}");

        self.search_result_label.set_label(&hints);

        let sensitive = match_num > 0;

        self.previous_match_button.set_sensitive(sensitive);
        self.next_match_button.set_sensitive(sensitive);
    }
}

#[glib::object_subclass]
impl ObjectSubclass for TextSearchBarImp {
    const NAME: &'static str = "TextFind";
    type Type = TextSearchBar;
    type ParentType = gtk::Box;

    fn class_init(klass: &mut Self::Class) {
        // The layout manager determines how child widgets are laid out.
        klass.bind_template();
        klass.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for TextSearchBarImp {
    fn constructed(&self) {
        self.parent_constructed();
    }
}

impl WidgetImpl for TextSearchBarImp {}
impl BoxImpl for TextSearchBarImp {}
