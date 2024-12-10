enum JournalAnswers {
    //Tokens(Vec<colorise::Token>, String),
    //Text(String),
    Markup(String),
    Events(Vec<JournalEventRaw>),
}

use crate::gtk::glib::translate::IntoGlib;
use chrono::{Local, TimeZone};
use gtk::{
    gdk, gio, glib, pango,
    prelude::*,
    subclass::{
        box_::BoxImpl,
        prelude::*,
        widget::{
            CompositeTemplateCallbacksClass, CompositeTemplateClass,
            CompositeTemplateInitializingExt, WidgetClassExt, WidgetImpl,
        },
    },
    TemplateChild, TextTag,
};
use std::{
    cell::{Cell, RefCell},
    sync::LazyLock,
};

use log::{debug, info, warn};

use crate::{
    systemd::{self, data::UnitInfo, JournalEventRaw},
    widget::preferences::data::PREFERENCES,
};

use super::{more_colors::TermColor, palette::Palette, rowitem::JournalEvent};

const PANEL_EMPTY: &str = "empty";
const PANEL_JOURNAL: &str = "journal";
const PANEL_SPINNER: &str = "spinner";

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/journal_panel.ui")]
pub struct JournalPanelImp {
    #[template_child]
    journal_refresh_button: TemplateChild<gtk::Button>,

    #[template_child]
    journal_events: TemplateChild<gtk::ListView>,

    #[template_child]
    panel_stack: TemplateChild<gtk::Stack>,

    #[template_child]
    scrolled_window: TemplateChild<gtk::ScrolledWindow>,

    unit: RefCell<Option<UnitInfo>>,

    #[template_child]
    list_store: TemplateChild<gio::ListStore>,

    is_dark: Cell<bool>,
}

#[gtk::template_callbacks]
impl JournalPanelImp {
    #[template_callback]
    fn refresh_journal_clicked(&self, button: &gtk::Button) {
        debug!("button {:?}", button);

        let binding = self.unit.borrow();
        let Some(unit) = binding.as_ref() else {
            warn!("no unit file");
            return;
        };

        self.update_journal(&unit)
    }

    pub(crate) fn display_journal(&self, unit: &UnitInfo) {
        let _old = self.unit.replace(Some(unit.clone()));

        self.update_journal(&unit)
    }

    /// Updates the associated journal `TextView` with the contents of the unit's journal log.
    fn update_journal(&self, unit: &UnitInfo) {
        //let journal_text: gtk::TextView = self.journal_text.clone();
        let unit = unit.clone();
        let journal_refresh_button = self.journal_refresh_button.clone();
        let oldest_first = false;
        let journal_max_events = PREFERENCES.journal_max_events();
        let panel_stack = self.panel_stack.clone();
        // let scrolled_window = self.scrolled_window.clone();
        //let journal_color: TermColor = journal_text.color().into();

        let store = self.list_store.clone();
        let journal_events = self.journal_events.clone();

        glib::spawn_future_local(async move {
            let in_color = PREFERENCES.journal_colors();
            panel_stack.set_visible_child_name(PANEL_SPINNER);
            journal_refresh_button.set_sensitive(false);
            let journal_answer = gio::spawn_blocking(move || {
                match systemd::get_unit_journal(&unit, in_color, oldest_first, journal_max_events) {
                    Ok(journal_output) => JournalAnswers::Events(journal_output),
                    Err(error) => {
                        let text = match error.gui_description() {
                            Some(s) => s.clone(),
                            None => String::from(""),
                        };
                        JournalAnswers::Markup(text)
                    }
                }
            })
            .await
            .expect("Task needs to finish successfully.");

            let panel = match journal_answer {
                JournalAnswers::Events(mut events) => {
                    let size = events.len();
                    info!("Number of event {}", size);

                    store.remove_all();

                    for je in events.drain(..) {
                        let journal_event = JournalEvent::new(je);
                        store.append(&journal_event);
                    }

                    journal_events.show();

                    if size == 0 {
                        PANEL_EMPTY
                    } else {
                        PANEL_JOURNAL
                    }
                }
                JournalAnswers::Markup(_markup_text) => {
                    warn!("Journal error");
                    PANEL_EMPTY
                }
            };

            journal_refresh_button.set_sensitive(true);

            panel_stack.set_visible_child_name(panel);
        });
    }

    pub(crate) fn set_dark(&self, is_dark: bool) {
        self.is_dark.set(is_dark);
    }

    #[template_callback]
    fn event_list_setup(&self, item_obj: &glib::Object) {
        let item = item_obj
            .downcast_ref::<gtk::ListItem>()
            .expect("item.downcast_ref::<gtk::ListItem>()");

        let text_view = gtk::TextView::new();
        item.set_child(Some(&text_view));
    }

    #[template_callback]
    fn event_list_bind(&self, item_obj: &glib::Object) {
        let item = item_obj
            .downcast_ref::<gtk::ListItem>()
            .expect("item.downcast_ref::<gtk::ListItem>()");

        let child = item.child().and_downcast::<gtk::TextView>().unwrap();
        let entry = item.item().and_downcast::<JournalEvent>().unwrap();

        let text_buffer = child.buffer();

        let local_result = Local.timestamp_millis_opt(entry.timestamp() as i64);

        let prefix = match local_result {
            chrono::offset::LocalResult::Single(l) => l.format("%Y-%m-%d %T").to_string(),
            chrono::offset::LocalResult::Ambiguous(a, _b) => a.format("%Y-%m-%d %T").to_string(),
            chrono::offset::LocalResult::None => "NONE".to_owned(),
        };

        let priority = entry.priority();

        if priority == 6 {
            let construct = format!("{} {} {}", priority, prefix, entry.message());
            text_buffer.set_text(&construct);
        } else {
            let tag_table = text_buffer.tag_table();

            let mut iter = text_buffer.start_iter();
            let construct = format!("{} {} ", priority, prefix);
            text_buffer.insert(&mut iter, &construct);

            let start_offset = iter.offset();
            text_buffer.insert(&mut iter, &entry.message());
            let start_iter = text_buffer.iter_at_offset(start_offset);

            let tag = get_tag(priority, self.is_dark.get());

            tag_table.add(&tag);
            text_buffer.apply_tag(&tag, &start_iter, &iter);
        }
    }
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for JournalPanelImp {
    const NAME: &'static str = "JournalPanel";
    type Type = super::JournalPanel;
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

impl ObjectImpl for JournalPanelImp {
    fn constructed(&self) {
        self.parent_constructed();
    }
}
impl WidgetImpl for JournalPanelImp {}
impl BoxImpl for JournalPanelImp {}

static RED: LazyLock<gdk::RGBA> = LazyLock::new(|| {
    let color: TermColor = Palette::Red3.into();
    let rgba = color.get_rgba();
    rgba
});

static RED_DARK: LazyLock<gdk::RGBA> = LazyLock::new(|| {
    let color: TermColor = Palette::Custom("#ef4b4b").into();
    let rgba = color.get_rgba();
    rgba
});

static YELLOW: LazyLock<gdk::RGBA> = LazyLock::new(|| {
    let color: TermColor = Palette::Yellow5.into();
    let rgba = color.get_rgba();
    rgba
});

static YELLOW_DARK: LazyLock<gdk::RGBA> = LazyLock::new(|| {
    let color: TermColor = Palette::Custom("#e5e540").into();
    let rgba = color.get_rgba();
    rgba
});

/// When outputting to a tty, lines are colored according to priority:
///        lines of level ERROR and higher  3-1
///                  are colored red; lines of level
///                  WARNING are colored yellow; 4
///                  lines of level NOTICE are highlighted; 5
///                  lines of level INFO are displayed normally; lines of level  6
///                  DEBUG are colored grey.
///
fn get_tag(priority: u8, is_dark: bool) -> TextTag {
    let tags = match priority {
        0..=3 => {
            let color = if is_dark { &RED_DARK } else { &RED };

            gtk::TextTag::builder()
                .weight(pango::Weight::Bold.into_glib())
                .foreground_rgba(color)
                .build()
        }
        4 => {
            let color = if is_dark { &YELLOW_DARK } else { &YELLOW };

            gtk::TextTag::builder()
                .weight(pango::Weight::Bold.into_glib())
                .foreground_rgba(color)
                .build()
        }
        5 => gtk::TextTag::builder()
            .weight(pango::Weight::Bold.into_glib())
            .build(),
        _ => {
            let color = TermColor::VGA(128, 128, 128);
            gtk::TextTag::builder()
                .foreground_rgba(&color.get_rgba())
                .build()
        }
    };
    tags
}
