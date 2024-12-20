use gtk::{glib::translate::IntoGlib, pango, prelude::*, TextBuffer, TextIter, TextTag};

use crate::widget::journal::palette::Palette;

pub struct UnitInfoWriter {
    buf: TextBuffer,
    iter: TextIter,
    is_dark: bool,
}

const TAG_NAME_HYPER_LINK: &str = "hyperlink";
const TAG_NAME_ACTIVE: &str = "active";
const TAG_NAME_DISABLE: &str = "disable";
const TAG_NAME_GREY: &str = "grey";

impl UnitInfoWriter {
    pub fn new(buf: TextBuffer, iter: TextIter, is_dark: bool) -> Self {
        UnitInfoWriter { buf, iter, is_dark }
    }

    pub fn insert(&mut self, text: &str) {
        self.buf.insert(&mut self.iter, text);
    }

    pub fn new_line(&mut self) {
        self.buf.insert(&mut self.iter, "\n");
    }

    pub fn insert_active(&mut self, text: &str) {
        self.insert_tag(text, Self::create_active_tag, None);
    }

    pub fn insert_disable(&mut self, text: &str) {
        self.insert_tag(text, Self::create_disable_tag, None);
    }

    pub fn insert_grey(&mut self, text: &str) {
        self.insert_tag(text, Self::create_grey_tag,None);
    }

    pub fn hyperlink(&mut self, text: &str, link: &str) {
        self.insert_tag(text, Self::create_hyperlink_tag, Some(link));
    }

    fn create_hyperlink_tag(buf: &TextBuffer, _is_dark: bool) -> Option<TextTag> {
        let tag_op = buf.create_tag(
            None,
            &[
                //  ("foreground", &"blue".to_value()),
                ("underline", &pango::Underline::SingleLine.to_value()),
            ],
        );


        tag_op
    }

    fn create_active_tag(buf: &TextBuffer, is_dark: bool) -> Option<TextTag> {
        let tag_op = buf.tag_table().lookup(TAG_NAME_ACTIVE);
        if tag_op.is_some() {
            return tag_op;
        }

        let color = if is_dark {
            Palette::Green3.get_color()
        } else {
            Palette::Green5.get_color()
        };

        let tag_op = buf.create_tag(
            Some(TAG_NAME_ACTIVE),
            &[
                ("foreground", &color.to_value()),
                ("weight", &pango::Weight::Bold.into_glib().to_value()),
            ],
        );

        tag_op
    }

    fn create_disable_tag(buf: &TextBuffer, is_dark: bool) -> Option<TextTag> {
        let tag_op = buf.tag_table().lookup(TAG_NAME_DISABLE);
        if tag_op.is_some() {
            return tag_op;
        }

        let color = if is_dark {
            Palette::Yellow3.get_color()
        } else {
            Palette::Yellow3.get_color()
        };

        let tag_op = buf.create_tag(
            Some(TAG_NAME_DISABLE),
            &[
                ("foreground", &color.to_value()),
                ("weight", &pango::Weight::Bold.into_glib().to_value()),
            ],
        );
        tag_op
    }

    fn create_grey_tag(buf: &TextBuffer, is_dark: bool) -> Option<TextTag> {
        let tag_op = buf.tag_table().lookup(TAG_NAME_GREY);
        if tag_op.is_some() {
            return tag_op;
        }

        let color = if is_dark {
            Palette::Dark1.get_color()
        } else {
            Palette::Dark2.get_color()
        };

        let tag_op = buf.create_tag(Some(TAG_NAME_GREY), &[("foreground", &color.to_value())]);
        tag_op
    }

    fn insert_tag(
        &mut self,
        text: &str,
        create_tag: impl Fn(&TextBuffer, bool) -> Option<TextTag>, link : Option<&str>
    ) {
        let start_offset = self.iter.offset();
        self.buf.insert(&mut self.iter, text);

        let tag_op = create_tag(&self.buf, self.is_dark);

        if let Some(tag) = tag_op {

            if let Some(link) = link {
                //tag.set_data("link", link);
            }
    


            let start_iter = self.buf.iter_at_offset(start_offset);
            self.buf.apply_tag(&tag, &start_iter, &self.iter);
        }
    }
}
