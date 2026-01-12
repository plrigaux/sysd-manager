use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
};

use gettextrs::pgettext;
use glib::WeakRef;
use gtk::{glib, prelude::*, subclass::prelude::*};
use regex::Regex;
use tracing::{debug, info, warn};

use crate::{format2, systemd_gui::is_dark, upgrade};

use super::TextSearchBar;

const SEARCH_HIGHLIGHT: &str = "search_highlight";
const SEARCH_HIGHLIGHT_SELECTED: &str = "search_highlight_selected";

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/text_find.ui")]
pub struct TextSearchBarImp {
    #[template_child]
    search_entry: TemplateChild<gtk::SearchEntry>,

    #[template_child]
    case_sensitive_toggle_button: TemplateChild<gtk::ToggleButton>,

    #[template_child]
    match_whole_word: TemplateChild<gtk::ToggleButton>,

    #[template_child]
    regex_toggle_button: TemplateChild<gtk::ToggleButton>,

    #[template_child]
    previous_match_button: TemplateChild<gtk::Button>,

    #[template_child]
    next_match_button: TemplateChild<gtk::Button>,

    #[template_child]
    search_result_label: TemplateChild<gtk::Label>,

    text_view: WeakRef<gtk::TextView>,

    iter_select: Cell<Option<(gtk::TextIter, gtk::TextIter)>>,

    finds: RefCell<BTreeMap<i32, i32>>,

    regex: RefCell<Option<Regex>>,
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
    fn on_previous_match_clicked(&self, _button: &gtk::Button) {
        self.previous_match_clicked();
    }

    #[template_callback]
    fn on_next_match_clicked(&self, _button: &gtk::Button) {
        self.next_match_clicked();
    }
}

impl TextSearchBarImp {
    fn previous_match_clicked(&self) {
        let text_view = upgrade!(self.text_view);
        let text_view = text_view;
        let buff = text_view.buffer();
        let tag_table = buff.tag_table();

        let Some(tag) = tag_table.lookup(SEARCH_HIGHLIGHT) else {
            warn!("No tag search highlight");
            return;
        };

        let mut end_iter = self.get_iter(&text_view, &buff, false);

        if !end_iter.backward_to_tag_toggle(Some(&tag)) {
            info!("iter can't find tag highlight begin");
            end_iter = buff.end_iter();
            if !end_iter.backward_to_tag_toggle(Some(&tag)) {
                warn!("iter can't find tag highlight begin from end");
                return;
            }
        }

        // start_iter is now at the beginning of a tagged range
        let mut start_iter = end_iter;
        // Move end_iter forward to the next toggle (end of the range)
        if !start_iter.backward_to_tag_toggle(Some(&tag)) {
            warn!("iter can't find tag highlight end");
            return;
        }

        self.apply_hl_tag(text_view, buff, tag_table, start_iter, end_iter);
    }

    fn apply_hl_tag(
        &self,
        text_view: gtk::TextView,
        buff: gtk::TextBuffer,
        tag_table: gtk::TextTagTable,
        mut start_iter: gtk::TextIter,
        end_iter: gtk::TextIter,
    ) {
        let tag_select = if let Some(tag_select) = tag_table.lookup(SEARCH_HIGHLIGHT_SELECTED) {
            // Remove previous highlights
            let start = buff.start_iter();
            let end = buff.end_iter();
            buff.remove_tag(&tag_select, &start, &end);

            tag_select
        } else {
            info!("is_dark {}", is_dark());
            let color = if is_dark() { "#f7d742" } else { "#e5d255" };
            let tag_select = gtk::TextTag::builder()
                .name(SEARCH_HIGHLIGHT_SELECTED)
                .background(color)
                //.priority(10)
                .build();

            if is_dark() {
                tag_select.set_foreground(Some("#000000"));
            }

            tag_table.add(&tag_select);
            tag_select
        };

        buff.apply_tag(&tag_select, &start_iter, &end_iter);
        text_view.scroll_to_iter(&mut start_iter, 0.2, false, 0.0, 0.0);
        self.iter_select.set(Some((start_iter, end_iter)));

        let finds = self.finds.borrow();

        let idx = finds.get(&start_iter.offset()).unwrap_or(&-1);
        //find count
        let search_result = format2!(&pgettext("text_find", "{} of {}"), idx, finds.len());
        self.search_result_label.set_label(&search_result);
    }

    fn next_match_clicked(&self) {
        let text_view = upgrade!(self.text_view);
        let buff = text_view.buffer();
        let tag_table = buff.tag_table();

        let Some(tag) = tag_table.lookup(SEARCH_HIGHLIGHT) else {
            warn!("No tag search highlight");
            return;
        };

        let mut start_iter = self.get_iter(&text_view, &buff, true);

        if !start_iter.forward_to_tag_toggle(Some(&tag)) {
            debug!("iter can't find tag highlight begin");
            start_iter = buff.start_iter();
            let found = start_iter.forward_to_tag_toggle(Some(&tag));
            if !found {
                warn!("iter can't find tag highlight begin from start");
                return;
            }
        }

        // start_iter is now at the beginning of a tagged range
        let mut end_iter = start_iter;
        // Move end_iter forward to the next toggle (end of the range)
        if !end_iter.forward_to_tag_toggle(Some(&tag)) {
            warn!("iter can't find tag highlight end");
            return;
        }

        self.apply_hl_tag(text_view, buff, tag_table, start_iter, end_iter);
    }

    fn get_iter(
        &self,
        text_view: &gtk::TextView,
        buff: &gtk::TextBuffer,
        is_next: bool,
    ) -> gtk::TextIter {
        if let Some((start_iter, end_iter)) = self.iter_select.get() {
            if is_next { end_iter } else { start_iter }
        } else {
            let cursor_pos = buff.cursor_position();
            let cursor_visible = text_view.is_cursor_visible();
            debug!("cur pos {cursor_pos} vis {cursor_visible}");

            let mut start_iter = buff.start_iter();
            //let fcp = start_iter.forward_cursor_position();

            start_iter.forward_chars(cursor_pos);
            if !start_iter.forward_cursor_position() {
                start_iter = buff.start_iter();
            }

            start_iter
        }
    }

    pub(crate) fn set_text_view(&self, text_view: &gtk::TextView) {
        self.text_view.set(Some(text_view));
    }

    pub(crate) fn grab_focus_on_search_entry(&self) {
        self.search_entry.grab_focus();
    }

    pub(super) fn highlight_text(&self) {
        let entry_text = self.search_entry.text();

        if entry_text.is_empty() {
            self.clear_index();
            return;
        }

        let text_view = upgrade!(self.text_view);

        let buff = text_view.buffer();

        let start = buff.start_iter();
        let end = buff.end_iter();

        let tag_table = buff.tag_table();

        let tag = if let Some(tag) = tag_table.lookup(SEARCH_HIGHLIGHT) {
            // Remove previous highlights
            buff.remove_tag(&tag, &start, &end);

            if let Some(tag_hl) = tag_table.lookup(SEARCH_HIGHLIGHT_SELECTED) {
                buff.remove_tag(&tag_hl, &start, &end);
            }

            tag
        } else {
            Self::new_tag(&tag_table)
        };

        self.set_regex(&entry_text);
        self.apply_tag(&buff, start, end, &tag);
    }
    fn new_tag(tag_table: &gtk::TextTagTable) -> gtk::TextTag {
        let color = if is_dark() { "#8a7826" } else { "#f8e45c" };

        let tag = gtk::TextTag::builder()
            .name(SEARCH_HIGHLIGHT)
            .background(color)
            .build();

        tag_table.add(&tag);
        tag
    }

    pub(super) fn new_added_text(
        &self,
        buff: &gtk::TextBuffer,
        start_iter: gtk::TextIter,
        end_iter: gtk::TextIter,
    ) {
        let tag_table = buff.tag_table();

        let tag = if let Some(tag) = tag_table.lookup(SEARCH_HIGHLIGHT) {
            tag
        } else {
            Self::new_tag(&tag_table)
        };
        self.apply_tag(buff, start_iter, end_iter, &tag);
    }

    fn apply_tag(
        &self,
        buff: &gtk::TextBuffer,
        start_iter: gtk::TextIter,
        end_iter: gtk::TextIter,
        tag: &gtk::TextTag,
    ) {
        let text = buff.text(&start_iter, &end_iter, true);

        //start.forward_search(str, flags, limit)
        let mut char_start: i32 = 0;
        let mut byte_start = 0;

        let mut match_num = 0;
        let mut finds = self.finds.borrow_mut();
        finds.clear();
        let borrow = self.regex.borrow();
        let Some(re) = borrow.as_ref() else {
            warn!("Find in text has no pattern");
            return;
        };

        for re_match in re.find_iter(&text) {
            let match_start = re_match.start();
            char_start += text[byte_start..match_start].chars().count() as i32;
            let re_match_end = re_match.end();
            let char_end = char_start + text[match_start..re_match_end].chars().count() as i32;

            let match_start = buff.iter_at_offset(char_start);
            let match_end = buff.iter_at_offset(char_end);

            buff.apply_tag(tag, &match_start, &match_end);

            match_num += 1;
            finds.insert(char_start, match_num);

            byte_start = re_match_end;
            char_start = char_end;
        }

        //find count initial
        let hints = format2!(&pgettext("text_find", "{} of {}"), 0, finds.len());

        self.search_result_label.set_label(&hints);

        self.prev_next_senstivity(match_num);
    }

    fn set_regex(&self, entry_text: &glib::GString) {
        let mut pattern = String::with_capacity(entry_text.len() + 15);
        if !self.case_sensitive_toggle_button.is_active() {
            pattern.push_str("(?i)");
        }

        if self.match_whole_word.is_active() {
            pattern.push_str("\\b");
        }

        if self.regex_toggle_button.is_active() {
            pattern.push_str(entry_text);
        } else {
            for c in entry_text.chars() {
                if matches!(c, '(' | ')' | '\\' | '*' | '[' | ']' | '.') {
                    pattern.push('\\');
                }
                pattern.push(c);
            }
        }

        if self.match_whole_word.is_active() {
            pattern.push_str("\\b");
        }

        let re = match Regex::new(&pattern) {
            Ok(re) => {
                self.search_entry.remove_css_class("error");
                re
            }
            Err(err) => {
                warn!("Invalid regex: {}", err);
                self.prev_next_senstivity(0);
                self.search_entry.add_css_class("error");
                return;
            }
        };

        self.regex.replace(Some(re));
    }

    fn prev_next_senstivity(&self, match_num: i32) {
        let sensitive = match_num > 0;

        self.previous_match_button.set_sensitive(sensitive);
        self.next_match_button.set_sensitive(sensitive);
    }

    fn clear_index(&self) {
        self.iter_select.set(None);
        self.prev_next_senstivity(0);
        self.search_result_label.set_label("");
        self.finds.borrow_mut().clear();
        self.regex.replace(None);
    }

    pub(super) fn clear_tags(&self) {
        self.clear_index();

        let text_view = upgrade!(self.text_view);

        let buff = text_view.buffer();
        let tag_table = buff.tag_table();

        if let Some(tag) = tag_table.lookup(SEARCH_HIGHLIGHT) {
            let start = buff.start_iter();
            let end = buff.end_iter();
            // Remove previous highlights
            buff.remove_tag(&tag, &start, &end);

            if let Some(tag_hl) = tag_table.lookup(SEARCH_HIGHLIGHT_SELECTED) {
                buff.remove_tag(&tag_hl, &start, &end);
            }
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
