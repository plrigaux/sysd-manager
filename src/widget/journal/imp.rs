enum JournalAnswers {
    //Tokens(Vec<colorise::Token>, String),
    //Text(String),
    Markup(String),
    Events(Vec<JournalEvent>),
}

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
    systemd::{self, data::UnitInfo, journal_data::JournalEvent, BootFilter},
    widget::preferences::data::PREFERENCES,
};

use super::{more_colors::TermColor, palette::Palette};

const PANEL_EMPTY: &str = "empty";
const PANEL_JOURNAL: &str = "journal";
const PANEL_SPINNER: &str = "spinner";

const ASCD: &str = "view-sort-ascending";
const DESC: &str = "view-sort-descending";

const CLASS_SUCCESS: &str = "success";
//const CLASS_ACCENT: &str = "accent";
const CLASS_WARNING: &str = "warning";
const CLASS_ERROR: &str = "error";

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
    journal_toggle_sort_button: TemplateChild<gtk::Button>,

    #[template_child]
    list_sort_model: TemplateChild<gtk::SortListModel>,

    #[template_child]
    journal_boot_current_button: TemplateChild<gtk::Button>,

    #[template_child]
    journal_boot_all_button: TemplateChild<gtk::Button>,

    #[template_child]
    journal_boot_id_entry: TemplateChild<adw::EntryRow>,

    #[template_child]
    list_store: TemplateChild<gio::ListStore>,

    unit: RefCell<Option<UnitInfo>>,

    is_dark: Cell<bool>,

    boot_filter: RefCell<BootFilter>,
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

        self.update_journal();
    }

    pub(crate) fn display_journal(&self, unit: &UnitInfo) {
        let _old = self.unit.replace(Some(unit.clone()));

        self.update_journal()
    }

    /// Updates the associated journal `TextView` with the contents of the unit's journal log.
    fn update_journal(&self) {
        //let journal_text: gtk::TextView = self.journal_text.clone();

        let binding = self.unit.borrow();
        let Some(unit_ref) = binding.as_ref() else {
            warn!("No unit file");
            return;
        };

        let unit = unit_ref.clone();
        let journal_refresh_button = self.journal_refresh_button.clone();
        let oldest_first = false;
        let journal_max_events = PREFERENCES.journal_max_events();
        let panel_stack = self.panel_stack.clone();
        let store = self.list_store.clone();
        let boot_filter = self.boot_filter.borrow().clone();

        glib::spawn_future_local(async move {
            let in_color = PREFERENCES.journal_colors();
            panel_stack.set_visible_child_name(PANEL_SPINNER);
            journal_refresh_button.set_sensitive(false);
            let journal_answer = gio::spawn_blocking(move || {
                match systemd::get_unit_journal(
                    &unit,
                    in_color,
                    oldest_first,
                    journal_max_events,
                    boot_filter,
                ) {
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

                    for journal_event in events.drain(..) {
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

        let priority = entry.priority();

        if priority == 6 || !PREFERENCES.journal_colors() {
            let construct = format!("{}{}", entry.prefix(), entry.message());
            event_display.set_text(&construct);
        } else {
            let mut construct = entry.prefix();
            let start = construct.len() as u32;
            construct.push_str(&entry.message());

            let is_dark = self.is_dark.get();
            let attributes = get_attrlist(priority, is_dark, start);
            event_display.set_text(&construct);
            event_display.set_attributes(Some(&attributes));
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
    fn boot_id_text_change(&self, entry: &adw::EntryRow) {
        let text = entry.text();
        info!("boot id entry_changed {}", text);
    }

    #[template_callback]
    fn journal_menu_popover_closed(&self) {
        info!("journal_menu_popover_closed");

        let boot_filter_op = if self.journal_boot_all_button.has_css_class(CLASS_SUCCESS) {
            Some(BootFilter::All)
        } else if self.journal_boot_id_entry.has_css_class(CLASS_SUCCESS) {
            let boot_id = self.journal_boot_id_entry.text();
            Some(BootFilter::Id(boot_id.to_string()))
        } else if self
            .journal_boot_current_button
            .has_css_class(CLASS_SUCCESS)
        {
            Some(BootFilter::Current)
        } else {
            None
        };

        if let Some(boot_filter) = boot_filter_op {
            let replaced = self.boot_filter.replace(boot_filter.clone());

            if replaced != boot_filter {
                //filter updated
                self.update_journal();
            }
        }
    }

    #[template_callback]
    fn journal_menu_popover_activate_default(&self) {
        info!("journal_menu_popover_activate_default");
    }

    #[template_callback]
    fn journal_menu_popover_show(&self) {
        info!("journal_menu_popover_show");

        self.clear_boot_id();

        let boot_filter_ref: &BootFilter = &self.boot_filter.borrow();

        match boot_filter_ref {
            BootFilter::Current => self
                .journal_boot_current_button
                .add_css_class(CLASS_SUCCESS),
            BootFilter::All => self.journal_boot_all_button.add_css_class(CLASS_SUCCESS),
            BootFilter::Id(boot_id) => {
                self.journal_boot_id_entry.set_text(boot_id);
                self.journal_boot_id_entry.add_css_class(CLASS_SUCCESS);
            }
        }
    }

    #[template_callback]
    fn journal_boot_all_button_clicked(&self) {
        info!("journal_boot_all_button_clicked");
        self.clear_boot_id();
        self.journal_boot_all_button.add_css_class(CLASS_SUCCESS);
    }

    #[template_callback]
    fn journal_boot_current_button_clicked(&self) {
        info!("journal_boot_current_button_clicked");
        self.clear_boot_id();
        self.journal_boot_current_button
            .add_css_class(CLASS_SUCCESS);
    }

    fn clear_boot_id(&self) {
        for css_class in [CLASS_WARNING, CLASS_ERROR, CLASS_SUCCESS] {
            self.journal_boot_id_entry.remove_css_class(css_class);
            self.journal_boot_all_button.remove_css_class(css_class);
            self.journal_boot_current_button.remove_css_class(css_class);
        }
    }

    #[template_callback]
    fn journal_boot_id_entry_change(&self) {
        self.set_boot_id_style();
    }

    pub(super) fn set_boot_id_style(&self) {
        let boot_id_text: glib::GString = self.journal_boot_id_entry.text();

        match validate_boot_id(&boot_id_text) {
            BootIdValidation::Fail => self.journal_boot_id_entry.add_css_class(CLASS_ERROR),
            BootIdValidation::Partial => {
                self.journal_boot_id_entry.remove_css_class(CLASS_WARNING);
                self.journal_boot_id_entry.remove_css_class(CLASS_ERROR);
                self.journal_boot_id_entry.remove_css_class(CLASS_SUCCESS);
            }
            BootIdValidation::Valid => {
                self.clear_boot_id();
                self.journal_boot_id_entry.add_css_class(CLASS_SUCCESS);
            }
            BootIdValidation::Over => {
                self.journal_boot_id_entry.remove_css_class(CLASS_SUCCESS);
                self.journal_boot_id_entry.remove_css_class(CLASS_ERROR);
                self.journal_boot_id_entry.add_css_class(CLASS_WARNING)
            }
        };
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
        let t = self.list_store.item_type();

        warn!("Type {:?}", t);

        create_sorter_ascd!(self);

        let event_controller_focus = gtk::EventControllerFocus::new();
        {
            let obj = self.obj().clone();
            event_controller_focus.connect_enter(move |_controller_focus| {
                info!("connect_enter");
                obj.set_boot_id_style();
            });
        }

        self.journal_boot_id_entry
            .add_controller(event_controller_focus);
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

#[derive(Debug, PartialEq, Eq)]
enum BootIdValidation {
    Fail,
    Partial,
    Valid,
    Over,
}

fn validate_boot_id(boot_id: &str) -> BootIdValidation {
    for c in boot_id.chars() {
        if c.is_ascii_digit() {
            continue;
        } else if matches!(c, 'a'..='f') {
            continue;
        } else {
            return BootIdValidation::Fail;
        }
    }

    match boot_id.len() {
        0..32 => BootIdValidation::Partial,
        32 => BootIdValidation::Valid,
        _ => BootIdValidation::Over,
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_boot_regex() {
        let tests = vec![
            ("abc1234", BootIdValidation::Partial),
            ("abc-1234", BootIdValidation::Fail),
            ("0123456789", BootIdValidation::Partial),
            ("abcdef", BootIdValidation::Partial),
            ("abcdefg", BootIdValidation::Fail),
            ("75505929b5c443a09ace6787429c3383", BootIdValidation::Valid),
            ("75505929b5c443a09ace6787429c338300", BootIdValidation::Over),
        ];

        for (boot_id, answer) in tests {
            let res = validate_boot_id(boot_id);

            assert_eq!(res, answer, "boot_id {}", boot_id);
        }
    }
}