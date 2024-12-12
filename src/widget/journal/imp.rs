enum JournalAnswers {
    //Tokens(Vec<colorise::Token>, String),
    //Text(String),
    Markup(String),
    Events(Vec<JournalEventRaw>),
}

use chrono::{Local, TimeZone};
use gtk::{
    gio, glib,
    pango::{self},
    prelude::*,
    subclass::{
        box_::BoxImpl,
        prelude::*,
        widget::{
            CompositeTemplateCallbacksClass, CompositeTemplateClass,
            CompositeTemplateInitializingExt, WidgetClassExt, WidgetImpl,
        },
    },
    TemplateChild,
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

        let event_display = gtk::Label::builder()
            .height_request(10) //to force min heigt
            .selectable(true)
            .xalign(0.0)
            .single_line_mode(false)
            .css_classes(["unit_info"])
            .build();

        item.set_child(Some(&event_display));
    }

    #[template_callback]
    fn event_list_bind(&self, item_obj: &glib::Object) {
        let item = item_obj
            .downcast_ref::<gtk::ListItem>()
            .expect("Needs to be ListItem");

        let event_display = item
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
  
        event_display.set_text(&construct);
        if priority == 6 || !PREFERENCES.journal_colors() {
            let construct = format!("{} {} {}", priority, prefix, entry.message());
            event_display.set_text(&construct);
        } else {
     
            let mut construct = format!("{} {} ", priority, prefix);
            let start = construct.len() as u32;
            construct.push_str(&entry.message());
         
            let is_dark = self.is_dark.get();
            let attributes = get_attrlist(priority, is_dark, start);
            event_display.set_text(&construct);
            event_display.set_attributes(Some(
                &attributes));
        }
    }

    #[template_callback]
    fn event_list_unbind(&self, item_obj: &glib::Object) {
        // Get `TaskRow` from `ListItem`
        let event_display = item_obj
            .downcast_ref::<gtk::ListItem>()
            .expect("Needs to be ListItem")
            .child()
            .and_downcast::<gtk::Label>()
            .expect("The child has to be a `gtk::Label`.");

        event_display.set_text("");
        event_display.set_attributes(None);
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

static RED: LazyLock<(u16, u16, u16)> = LazyLock::new(|| {
    let color: TermColor = Palette::Red3.into();
    let rgba = color.get_rgb_u16();
    rgba
});

static RED_DARK: LazyLock<(u16, u16, u16)> = LazyLock::new(|| {
    let color: TermColor = Palette::Custom("#ef4b4b").into();
    let rgba = color.get_rgb_u16();
    rgba
});

static YELLOW: LazyLock<(u16, u16, u16)> = LazyLock::new(|| {
    let color: TermColor = Palette::Yellow5.into();
    let rgba = color.get_rgb_u16();
    rgba
});

static YELLOW_DARK: LazyLock<(u16, u16, u16)> = LazyLock::new(|| {
    let color: TermColor = Palette::Custom("#e5e540").into();
    let rgba = color.get_rgb_u16();
    rgba
});

macro_rules! set_attr_color {
    ($attrlist:expr, $r:expr, $g:expr, $b:expr, $start:expr) => {{
        let mut attr_color = pango::AttrColor::new_foreground($r, $g, $b);
        attr_color.set_start_index($start);

        $attrlist.insert(attr_color);
    }};
}

macro_rules! set_attr_bold {
    ($attr_list:expr,  $start:expr) => {{
        let mut font_description = pango::FontDescription::new();
        font_description.set_weight(pango::Weight::Bold);

        let mut attr_font_description = pango::AttrFontDesc::new(&font_description);
        attr_font_description.set_start_index($start);

        $attr_list.insert(attr_font_description);
    }};
}

macro_rules! set_attr_color_bold {
    ($attr_list:expr, $r:expr, $g:expr, $b:expr, $start:expr) => {{
        set_attr_color!($attr_list, $r, $g, $b, $start);
        set_attr_bold!($attr_list, $start);
    }};
}

/// When outputting to a tty, lines are colored according to priority:
///        lines of level ERROR and higher  3-1
///                  are colored red; lines of level
///                  WARNING are colored yellow; 4
///                  lines of level NOTICE are highlighted; 5
///                  lines of level INFO are displayed normally; lines of level  6
///                  DEBUG are colored grey.
///
fn get_attrlist(priority: u8, is_dark: bool, start: u32) -> pango::AttrList {
    let attr_list = pango::AttrList::new();
    match priority {
        0..=3 => {
            let color = if is_dark { &RED_DARK } else { &RED };

            set_attr_color_bold!(attr_list, color.0, color.1, color.2, start);
        }
        4 => {
            let color = if is_dark { &YELLOW_DARK } else { &YELLOW };

            set_attr_color_bold!(attr_list, color.0, color.1, color.2, start);
        }
        5 => {
            set_attr_bold!(attr_list, start);
        }
        7 => {
            set_attr_color!(attr_list, 0x8888, 0x8888, 0x8888, start);
        }

        _ => {
            warn!("Priority {priority} not handeled")
        }
    };
    attr_list
}
