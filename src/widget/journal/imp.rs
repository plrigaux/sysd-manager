enum JournalAnswers {
    //Tokens(Vec<colorise::Token>, String),
    //Text(String),
    Markup(String),
    Events(Vec<JournalEventRaw>),
}

use crate::gtk::glib::translate::IntoGlib;
use chrono::{Local, TimeZone};
use gtk::{
    gdk, gio, glib,
    pango::{self, AttrList},
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

const ASCD: &str = "view-sort-ascending";
const DESC: &str = "view-sort-descending";

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

    #[template_child]
    from_last_boot_check_button: TemplateChild<gtk::CheckButton>,

    #[template_child]
    journal_toggle_sort_button: TemplateChild<gtk::Button>,

    #[template_child]
    list_sort_model: TemplateChild<gtk::SortListModel>,

    unit: RefCell<Option<UnitInfo>>,

    #[template_child]
    list_store: TemplateChild<gio::ListStore>,

    is_dark: Cell<bool>,
}

macro_rules! create_sorter_ascd {
    ($self:expr) => {{
        let sorter = gtk::CustomSorter::new(move |obj1, obj2| {
            let unit1 = obj1
                .downcast_ref::<JournalEvent>()
                .expect("Needs to be JournalEvent");
            let unit2 = obj2
                .downcast_ref::<JournalEvent>()
                .expect("Needs to be JournalEvent");

            unit1.timestamp().cmp(&unit2.timestamp()).into()
        });

        $self.list_sort_model.set_sorter(Some(&sorter))
    }};
}

macro_rules! create_sorter_desc {
    ($self:expr) => {{
        let sorter = gtk::CustomSorter::new(move |obj1, obj2| {
            let unit1 = obj1
                .downcast_ref::<JournalEvent>()
                .expect("Needs to be JournalEvent");
            let unit2 = obj2
                .downcast_ref::<JournalEvent>()
                .expect("Needs to be JournalEvent");

            unit2.timestamp().cmp(&unit1.timestamp()).into()
        });

        $self.list_sort_model.set_sorter(Some(&sorter))
    }};
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
        //let scrolled_window = self.scrolled_window.clone();
        let store = self.list_store.clone();
        //let journal_events = self.journal_events.clone();

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

                    //journal_events.vadjustment();

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

        //let adj = gtk::Adjustment::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let text_view = gtk::Label::new(None);
        text_view.set_height_request(16); //to force min heigt
        text_view.set_selectable(true);
        text_view.set_xalign(0.0);
        text_view.set_single_line_mode(false);
        text_view.add_css_class("unit_info");

        //text_view.set_editable(false);
        //text_view.set_cursor_visible(false);
        //text_view.set_monospace(true);
        //.hadjustment(&adj)
        //.can_focus(false)
        //.can_target(false)
        //.build();

        item.set_child(Some(&text_view));
    }

    #[template_callback]
    fn event_list_bind(&self, item_obj: &glib::Object) {
        let item = item_obj
            .downcast_ref::<gtk::ListItem>()
            .expect("Needs to be ListItem");

        let child = item
            .child()
            .and_downcast::<gtk::Label>()
            .expect("The child has to be a `gtk::Label`.");
        let entry = item
            .item()
            .and_downcast::<JournalEvent>()
            .expect("The item has to be an `JournalEvent`.");

        //let text_buffer = child.buffer();

        let local_result = Local.timestamp_millis_opt(entry.timestamp() as i64);

        let prefix = match local_result {
            chrono::offset::LocalResult::Single(l) => l.format("%Y-%m-%d %T").to_string(),
            chrono::offset::LocalResult::Ambiguous(a, _b) => a.format("%Y-%m-%d %T").to_string(),
            chrono::offset::LocalResult::None => "NONE".to_owned(),
        };

        let priority = entry.priority();
        let construct = format!("{} {} {}", priority, prefix, entry.message());

        if priority <= 3 {
            let asdf = pango::AttrList::new();

            let mut a = pango::AttrColor::new_foreground(0xFFFF, 0, 0);
            a.set_start_index(8);

            let mut fd = pango::FontDescription::new();
            fd.set_weight(pango::Weight::Bold);

            asdf.insert(a);
            asdf.insert(pango::AttrFontDesc::new(&fd));

            child.set_attributes(Some(&asdf));
        } else {
            let asdf = pango::AttrList::new();

            let mut a = pango::AttrColor::new_foreground(0, 0, 0xFFFF);
            a.set_start_index(8);
            asdf.insert(a);
            child.set_attributes(Some(&asdf));
        }

        child.set_text(&construct);
        /*  if priority == 6 || !PREFERENCES.journal_colors() {
            let construct = format!("{} {} {}", priority, prefix, entry.message());
            text_buffer.set_text(&construct);
        } else {
            let tag_table = text_buffer.tag_table();
            //text_buffer.set_text("");

            let mut iter = text_buffer.start_iter();
            let construct = format!("{} {} ", priority, prefix);

            text_buffer.insert(&mut iter, &construct);

            let start_offset = iter.offset();
            text_buffer.insert(&mut iter, &entry.message());
            let start_iter = text_buffer.iter_at_offset(start_offset);

            let tag = get_tag(priority, self.is_dark.get());

            tag_table.add(&tag);
            text_buffer.apply_tag(&tag, &start_iter, &iter);
        } */
    }

    #[template_callback]
    fn event_list_unbind(&self, item_obj: &glib::Object) {
        // Get `TaskRow` from `ListItem`
        let task_row = item_obj
            .downcast_ref::<gtk::ListItem>()
            .expect("Needs to be ListItem")
            .child()
            .and_downcast::<gtk::Label>()
            .expect("The child has to be a `gtk::Label`.");

        //task_row.buffer().set_text("");
        task_row.set_text("");
    }

    #[template_callback]
    fn toggle_sort_clicked(&self, button: &gtk::Button) {
        info!("toggle_sort_clicked");

        let child = button.child().and_downcast::<adw::ButtonContent>().unwrap();

        let icon_name = child.icon_name();

        if icon_name == ASCD {
            child.set_icon_name(DESC);

            create_sorter_desc!(self);
        } else {
            //     view-sort-descending
            child.set_icon_name(ASCD);

            create_sorter_ascd!(self);
        }
    }

    #[template_callback]
    fn from_last_boot_toggled(&self, check: &gtk::CheckButton) {
        info!("from_last_boot_toggled {}", check.is_active());
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

        create_sorter_ascd!(self);
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
