use std::borrow::Cow;

use gtk::{glib::translate::IntoGlib, pango, prelude::*, TextBuffer, TextIter, TextTag};
use log::debug;

use crate::systemd::{
    self,
    enums::{ActiveState, UnitDBusLevel},
    generate_file_uri,
};

use super::palette::{green, grey, red, yellow, Palette};

pub struct UnitInfoWriter {
    buf: TextBuffer,
    iter: TextIter,
    is_dark: bool,
}

//const TAG_NAME_HYPER_LINK: &str = "hyperlink";
const TAG_NAME_ACTIVE: &str = "active";
const TAG_NAME_ACTIVE_DARK: &str = "active_dark";
const TAG_NAME_RED: &str = "red";
const TAG_NAME_RED_DARK: &str = "red_dark";
const TAG_NAME_DISABLE: &str = "disable";
const TAG_NAME_DISABLE_DARK: &str = "disable_dark";
const TAG_NAME_GREY: &str = "grey";
const TAG_NAME_GREY_DARK: &str = "grey_dark";
const TAG_NAME_STATUS: &str = "blue";
const TAG_NAME_STATUS_DARK: &str = "blue_dark";
pub const TAG_DATA_LINK: &str = "link";
const TAG_NAME_YELLOW_DARK: &str = "yellow_dark";
const TAG_NAME_YELLOW: &str = "yellow";

pub const PROP_UNDERLINE: &str = "underline";

pub const SPECIAL_GLYPH_TREE_VERTICAL: &str = "│ ";
pub const SPECIAL_GLYPH_TREE_SPACE: &str = "  ";
pub const SPECIAL_GLYPH_TREE_RIGHT: &str = "└─";
pub const SPECIAL_GLYPH_TREE_BRANCH: &str = "├─";

const PROP_WEIGHT: &str = "weight";
const PROP_FOREGROUND: &str = "foreground";

pub enum HyperLinkType {
    File,
    Unit(UnitDBusLevel),
    Http,
    Man,
    None,
}

impl HyperLinkType {
    fn ensure_link_type<'a>(&self, link: &'a str) -> Cow<'a, str> {
        match self {
            HyperLinkType::File => {
                if link.starts_with("file://") {
                    Cow::from(link)
                } else {
                    Cow::from(generate_file_uri(link))
                }
            }
            HyperLinkType::Unit(level) => {
                if link.starts_with("unit://") {
                    Cow::from(link)
                } else {
                    Cow::from(format!("unit://{}?{}", link, level.short()))
                }
            }
            HyperLinkType::Http => Cow::from(link),
            HyperLinkType::Man => Cow::from(link),
            HyperLinkType::None => Cow::from(link),
        }
    }
}

impl UnitInfoWriter {
    pub fn new(buf: TextBuffer, iter: TextIter, is_dark: bool) -> Self {
        UnitInfoWriter { buf, iter, is_dark }
    }

    pub fn insert(&mut self, text: &str) {
        self.buf.insert(&mut self.iter, text);
    }

    pub fn insertln(&mut self, text: &str) {
        self.buf.insert(&mut self.iter, text);
        self.buf.insert(&mut self.iter, "\n");
    }

    pub fn newline(&mut self) {
        self.buf.insert(&mut self.iter, "\n");
    }

    pub fn insert_active(&mut self, text: &str) {
        self.insert_tag(text, Self::create_active_tag, None, HyperLinkType::None);
    }

    pub fn insert_red(&mut self, text: &str) {
        self.insert_tag(text, Self::create_red_tag, None, HyperLinkType::None);
    }

    pub fn insert_yellow(&mut self, text: &str) {
        self.insert_tag(text, Self::create_yellow_tag, None, HyperLinkType::None);
    }

    pub fn insert_disable(&mut self, text: &str) {
        self.insert_tag(text, Self::create_disable_tag, None, HyperLinkType::None);
    }

    pub fn insert_grey(&mut self, text: &str) {
        self.insert_tag(text, Self::create_grey_tag, None, HyperLinkType::None);
    }

    pub fn insert_bold(&mut self, text: &str) {
        self.insert_tag(text, Self::create_bold_tag, None, HyperLinkType::None);
    }

    pub fn insert_status(&mut self, text: &str) {
        self.insert_tag(text, Self::create_status_tag, None, HyperLinkType::None);
    }

    pub fn hyperlink(&mut self, text: &str, link: &str, type_: HyperLinkType) {
        self.insert_tag(text, Self::create_hyperlink_tag, Some(link), type_);
    }

    pub fn insert_state(&mut self, state: ActiveState) {
        let glyph = state.glyph_str();

        match state {
            systemd::enums::ActiveState::Active
            | systemd::enums::ActiveState::Reloading
            | systemd::enums::ActiveState::Activating
            | systemd::enums::ActiveState::Refreshing => self.insert_active(glyph),

            systemd::enums::ActiveState::Inactive | systemd::enums::ActiveState::Deactivating => {
                self.insert(glyph);
            }
            _ => self.insert_red(glyph),
        }
    }

    fn create_hyperlink_tag(buf: &TextBuffer, _is_dark: bool) -> Option<TextTag> {
        buf.create_tag(
            None,
            &[
                //  (PROP_FOREGROUND, &"blue".to_value()),
                (PROP_UNDERLINE, &pango::Underline::SingleLine.to_value()),
            ],
        )
    }

    pub fn green_dark() -> &'static str {
        Palette::Green3.get_color()
    }

    pub fn green_light() -> &'static str {
        Palette::Green3.get_color()
    }

    fn create_active_tag(buf: &TextBuffer, is_dark: bool) -> Option<TextTag> {
        let (color, name) = if is_dark {
            (green(is_dark), TAG_NAME_ACTIVE_DARK)
        } else {
            (green(is_dark), TAG_NAME_ACTIVE)
        };

        let tag_op = buf.tag_table().lookup(name);
        if tag_op.is_some() {
            return tag_op;
        }

        buf.create_tag(
            Some(name),
            &[
                (PROP_FOREGROUND, &color.get_color().to_value()),
                (PROP_WEIGHT, &pango::Weight::Bold.into_glib().to_value()),
            ],
        )
    }

    fn create_yellow_tag(buf: &TextBuffer, is_dark: bool) -> Option<TextTag> {
        let (color, name) = if is_dark {
            (yellow(is_dark), TAG_NAME_YELLOW_DARK)
        } else {
            (yellow(is_dark), TAG_NAME_YELLOW)
        };

        let tag_op = buf.tag_table().lookup(name);
        if tag_op.is_some() {
            return tag_op;
        }

        buf.create_tag(
            Some(name),
            &[
                (PROP_FOREGROUND, &color.get_color().to_value()),
                (PROP_WEIGHT, &pango::Weight::Bold.into_glib().to_value()),
            ],
        )
    }

    /*     pub fn red_dark() -> &'static str {
        Palette::RedErrorDark.get_color()
    }

    pub fn red_light() -> &'static str {
        Palette::Red3.get_color()
    } */

    fn create_red_tag(buf: &TextBuffer, is_dark: bool) -> Option<TextTag> {
        let (color, name) = if is_dark {
            (red(is_dark), TAG_NAME_RED_DARK)
        } else {
            (red(is_dark), TAG_NAME_RED)
        };

        let tag_op = buf.tag_table().lookup(name);
        if tag_op.is_some() {
            return tag_op;
        }

        buf.create_tag(
            Some(name),
            &[
                (PROP_FOREGROUND, &color.get_color().to_value()),
                (PROP_WEIGHT, &pango::Weight::Bold.into_glib().to_value()),
            ],
        )
    }

    fn create_disable_tag(buf: &TextBuffer, is_dark: bool) -> Option<TextTag> {
        let (color, name) = if is_dark {
            (Palette::Yellow3.get_color(), TAG_NAME_DISABLE_DARK)
        } else {
            (Palette::Yellow4.get_color(), TAG_NAME_DISABLE)
        };

        let tag_op = buf.tag_table().lookup(name);
        if tag_op.is_some() {
            return tag_op;
        }

        buf.create_tag(
            Some(name),
            &[
                (PROP_FOREGROUND, &color.to_value()),
                (PROP_WEIGHT, &pango::Weight::Bold.into_glib().to_value()),
            ],
        )
    }

    fn create_bold_tag(buf: &TextBuffer, _is_dark: bool) -> Option<TextTag> {
        const NAME: &str = "BOLD_SIMPLE";

        let tag_op = buf.tag_table().lookup(NAME);
        if tag_op.is_some() {
            return tag_op;
        }

        buf.create_tag(
            Some(NAME),
            &[(PROP_WEIGHT, &pango::Weight::Bold.into_glib().to_value())],
        )
    }

    fn create_grey_tag(buf: &TextBuffer, is_dark: bool) -> Option<TextTag> {
        let (color, name) = if is_dark {
            (grey(is_dark), TAG_NAME_GREY_DARK)
        } else {
            (grey(is_dark), TAG_NAME_GREY)
        };

        let tag_op = buf.tag_table().lookup(name);
        if tag_op.is_some() {
            return tag_op;
        }

        buf.create_tag(
            Some(name),
            &[(PROP_FOREGROUND, &color.get_color().to_value())],
        )
    }

    pub fn blue_dark() -> &'static str {
        Palette::Blue2.get_color()
    }

    pub fn blue_light() -> &'static str {
        Palette::Blue4.get_color()
    }

    fn create_status_tag(buf: &TextBuffer, is_dark: bool) -> Option<TextTag> {
        let (color, name) = if is_dark {
            (Self::blue_dark(), TAG_NAME_STATUS_DARK)
        } else {
            (Self::blue_light(), TAG_NAME_STATUS)
        };

        let tag_op = buf.tag_table().lookup(name);
        if tag_op.is_some() {
            return tag_op;
        }

        buf.create_tag(
            Some(name),
            &[
                (PROP_FOREGROUND, &color.to_value()),
                (PROP_WEIGHT, &pango::Weight::Bold.into_glib().to_value()),
            ],
        )
    }

    fn insert_tag(
        &mut self,
        text: &str,
        create_tag: impl Fn(&TextBuffer, bool) -> Option<TextTag>,
        link: Option<&str>,
        type_: HyperLinkType,
    ) {
        let start_offset = self.iter.offset();
        self.buf.insert(&mut self.iter, text);

        let tag = create_tag(&self.buf, self.is_dark);

        if let Some(tag) = tag {
            if let Some(link) = link {
                let link = type_.ensure_link_type(link);

                let link_value = link.to_value();
                debug!("text {} link {:?}", text, link_value);
                unsafe {
                    tag.set_data(TAG_DATA_LINK, link_value);
                }
            }

            let start_iter = self.buf.iter_at_offset(start_offset);
            self.buf.apply_tag(&tag, &start_iter, &self.iter);
        }
    }

    pub fn char_count(&self) -> i32 {
        self.buf.char_count()
    }
}
