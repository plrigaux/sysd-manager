use std::{borrow::Cow, sync::LazyLock};

use glib::Quark;
use gtk::{glib::translate::IntoGlib, pango, prelude::*};
use log::debug;

use super::palette::{blue, green, grey, red, yellow};
use crate::{
    systemd::{self, enums::ActiveState, generate_file_uri},
    systemd_gui::is_dark,
};
use base::enums::UnitDBusLevel;

pub struct UnitInfoWriter {
    pub buffer: gtk::TextBuffer,
    pub text_iterator: gtk::TextIter,
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
pub static TAG_DATA_LINK: LazyLock<Quark> = LazyLock::new(|| Quark::from_str("link"));

/* const TAG_NAME_YELLOW_DARK: &str = "yellow_dark";
const TAG_NAME_YELLOW: &str = "yellow";
 */
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
    pub fn new(buf: gtk::TextBuffer, iter: gtk::TextIter) -> Self {
        Self {
            buffer: buf,
            text_iterator: iter,
        }
    }

    pub fn insert(&mut self, text: &str) {
        self.buffer.insert(&mut self.text_iterator, text);
    }

    pub fn insertln(&mut self, text: &str) {
        self.buffer.insert(&mut self.text_iterator, text);
        self.buffer.insert(&mut self.text_iterator, "\n");
    }

    pub fn newline(&mut self) {
        self.buffer.insert(&mut self.text_iterator, "\n");
    }

    pub fn insert_active(&mut self, text: &str) {
        self.insert_tag(text, Self::create_active_tag, None, HyperLinkType::None);
    }

    pub fn insert_red(&mut self, text: &str) {
        self.insert_tag(text, Self::create_red_tag, None, HyperLinkType::None);
    }

    /*     pub fn insert_yellow(&mut self, text: &str) {
        self.insert_tag(text, Self::create_yellow_tag, None, HyperLinkType::None);
    } */

    pub fn insert_disable(&mut self, text: &str) {
        self.insert_tag(text, Self::create_disable_tag, None, HyperLinkType::None);
    }

    pub fn insert_grey(&mut self, text: &str) {
        self.insert_tag(text, Self::create_grey_tag, None, HyperLinkType::None);
    }

    /*     pub fn insert_bold(&mut self, text: &str) {
        self.insert_tag(text, Self::create_bold_tag, None, HyperLinkType::None);
    } */

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

    fn create_hyperlink_tag(buf: &gtk::TextBuffer) -> Option<gtk::TextTag> {
        buf.create_tag(
            None,
            &[
                //  (PROP_FOREGROUND, &"blue".to_value()),
                (PROP_UNDERLINE, &pango::Underline::SingleLine.to_value()),
            ],
        )
    }

    fn create_active_tag(buf: &gtk::TextBuffer) -> Option<gtk::TextTag> {
        let (color, name) = if is_dark() {
            (green(), TAG_NAME_ACTIVE_DARK)
        } else {
            (green(), TAG_NAME_ACTIVE)
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

    fn create_red_tag(buf: &gtk::TextBuffer) -> Option<gtk::TextTag> {
        let (color, name) = if is_dark() {
            (red(), TAG_NAME_RED_DARK)
        } else {
            (red(), TAG_NAME_RED)
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

    fn create_disable_tag(buf: &gtk::TextBuffer) -> Option<gtk::TextTag> {
        let (color, name) = if is_dark() {
            (yellow(), TAG_NAME_DISABLE_DARK)
        } else {
            (yellow(), TAG_NAME_DISABLE)
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

    /*     fn create_bold_tag(buf: &gtk::TextBuffer, _is_dark: bool) -> Option<gtk::TextTag> {
        const NAME: &str = "BOLD_SIMPLE";

        let tag_op = buf.tag_table().lookup(NAME);
        if tag_op.is_some() {
            return tag_op;
        }

        buf.create_tag(
            Some(NAME),
            &[(PROP_WEIGHT, &pango::Weight::Bold.into_glib().to_value())],
        )
    } */

    fn create_grey_tag(buf: &gtk::TextBuffer) -> Option<gtk::TextTag> {
        let (color, name) = if is_dark() {
            (grey(), TAG_NAME_GREY_DARK)
        } else {
            (grey(), TAG_NAME_GREY)
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

    fn create_status_tag(buf: &gtk::TextBuffer) -> Option<gtk::TextTag> {
        let (color, name) = if is_dark() {
            (blue(), TAG_NAME_STATUS_DARK)
        } else {
            (blue(), TAG_NAME_STATUS)
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

    fn insert_tag(
        &mut self,
        text: &str,
        create_tag: impl Fn(&gtk::TextBuffer) -> Option<gtk::TextTag>,
        link: Option<&str>,
        type_: HyperLinkType,
    ) {
        let start_offset = self.text_iterator.offset();
        self.buffer.insert(&mut self.text_iterator, text);

        let tag = create_tag(&self.buffer);

        if let Some(tag) = tag {
            if let Some(link) = link {
                let link = type_.ensure_link_type(link);

                let link_value = link.to_value();
                debug!("text {text} link {link_value:?}");
                unsafe {
                    tag.set_qdata(*TAG_DATA_LINK, link_value);
                }
            }

            let start_iter = self.buffer.iter_at_offset(start_offset);
            self.buffer
                .apply_tag(&tag, &start_iter, &self.text_iterator);
        }
    }

    pub fn char_count(&self) -> i32 {
        self.buffer.char_count()
    }
}
