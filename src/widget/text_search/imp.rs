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

        let text = buff.text(&start, &end, true);
        println!("{}", text);

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

        for re_match in re.find_iter(&text) {
            let match_start = buff.iter_at_offset(re_match.start() as i32);
            let match_end = buff.iter_at_offset(re_match.end() as i32);

            buff.apply_tag(&tag, &match_start, &match_end);
        }
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
