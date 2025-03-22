use gtk::{
    TemplateChild, gio, glib,
    prelude::*,
    subclass::{
        box_::BoxImpl,
        prelude::*,
        widget::{
            CompositeTemplateCallbacksClass, CompositeTemplateClass,
            CompositeTemplateInitializingExt, WidgetClassExt, WidgetImpl,
        },
    },
};

use std::cell::{Cell, RefCell};

use log::{debug, error, info, warn};

use crate::{
    systemd::{
        self, BootFilter,
        data::UnitInfo,
        journal::BOOT_IDX,
        journal_data::{EventRange, JournalEvent, JournalEventChunk},
    },
    utils::{font_management::set_text_view_font, writer::UnitInfoWriter},
    widget::{InterPanelMessage, preferences::data::PREFERENCES},
};

const PANEL_EMPTY: &str = "empty";
const PANEL_JOURNAL: &str = "journal";
const PANEL_SPINNER: &str = "spinner";

const ASCD: &str = "view-sort-ascending";
const DESC: &str = "view-sort-descending";

const CLASS_SUCCESS: &str = "success";
//const CLASS_ACCENT: &str = "accent";
const CLASS_WARNING: &str = "warning";
const CLASS_ERROR: &str = "error";

#[derive(Default, glib::Properties, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/journal_panel.ui")]
#[properties(wrapper_type = super::JournalPanel)]
pub struct JournalPanelImp {
    #[template_child]
    journal_refresh_button: TemplateChild<gtk::Button>,

    #[template_child]
    panel_stack: TemplateChild<gtk::Stack>,

    #[template_child]
    scrolled_window: TemplateChild<gtk::ScrolledWindow>,

    #[template_child]
    journal_toggle_sort_button: TemplateChild<gtk::Button>,

    #[template_child]
    journal_boot_current_button: TemplateChild<gtk::Button>,

    #[template_child]
    journal_boot_all_button: TemplateChild<gtk::Button>,

    #[template_child]
    journal_boot_id_entry: TemplateChild<adw::EntryRow>,

    visible_on_page: Cell<bool>,

    //unit_journal_loaded: Cell<bool>,

    //list_store: RefCell<Option<gio::ListStore>>,
    #[property(get, set=Self::set_unit, nullable)]
    unit: RefCell<Option<UnitInfo>>,

    is_dark: Cell<bool>,

    boot_filter: RefCell<BootFilter>,

    from_time: Cell<Option<u64>>,

    most_recent_time: Cell<Option<u64>>,

    journal_text_view: RefCell<gtk::TextView>,

    older_to_recent: Cell<bool>,
}

#[gtk::template_callbacks]
impl JournalPanelImp {
    #[template_callback]
    fn refresh_journal_clicked(&self, _button: &gtk::Button) {
        info!("journal refresh button click");
        self.new_text_view();
        self.update_journal(EventGrabbing::Default);
    }

    #[template_callback]
    fn toggle_sort_clicked(&self, button: &gtk::Button) {
        info!("toggle_sort_clicked");

        let child = button.child().and_downcast::<adw::ButtonContent>().unwrap();

        let icon_name = child.icon_name();

        if icon_name == ASCD {
            child.set_icon_name(DESC);
            child.set_label("Descending");
            self.older_to_recent.set(false);
        } else {
            //     view-sort-descending
            child.set_icon_name(ASCD);
            child.set_label("Ascending");
            self.older_to_recent.set(true);
        }

        self.new_text_view();
        self.update_journal(EventGrabbing::Default);
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
                self.update_journal(EventGrabbing::Default);
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

    #[template_callback]
    fn on_journal_hide(&self) {
        error!("journal hide");
    }

    #[template_callback]
    fn on_journal_show(&self) {
        error!("journal show");
    }

    #[template_callback]
    fn on_journal_move_focus(&self) {
        error!("journal on_journal_move_focus");
    }

    #[template_callback]
    fn on_journal_realize(&self) {
        error!("journal realize");
    }

    #[template_callback]
    fn on_journal_unrealize(&self) {
        error!("journal unrealize");
    }

    #[template_callback]
    fn scwin_edge_overshot(&self, position: gtk::PositionType) {
        info!("scwin_edge_overshot {:?}", position);

        self.on_position(position);
    }

    #[template_callback]
    fn scwin_edge_reached(&self, position: gtk::PositionType) {
        info!("scwin_edge_reached {:?}", position);

        self.on_position(position);
    }

    fn on_position(&self, position: gtk::PositionType) {
        match position {
            gtk::PositionType::Bottom => {
                info!("call for new {:?}", position);
                self.update_journal(EventGrabbing::Default)
            }
            gtk::PositionType::Top => {
                if !self.older_to_recent.get() {
                    let grabber = EventGrabbing::Newer;
                    self.update_journal(grabber)
                };
            }
            _ => {}
        }
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
}

impl JournalPanelImp {
    fn set_visible_on_page(&self, value: bool) {
        debug!("set_visible_on_page val {value}");
        self.visible_on_page.set(value);

        self.update_journal(EventGrabbing::Default)
    }

    pub(crate) fn set_unit(&self, unit: Option<&UnitInfo>) {
        let unit = match unit {
            Some(u) => u,
            None => {
                self.unit.replace(None);
                self.new_text_view();
                self.panel_stack.set_visible_child_name(PANEL_EMPTY);
                //self.update_journal(); //to clear the journal
                return;
            }
        };

        let old_unit = self.unit.replace(Some(unit.clone()));

        if unit.primary() != old_unit.map_or(String::new(), |o_unit| o_unit.primary()) {
            self.new_text_view();
        }

        self.update_journal(EventGrabbing::Default)
    }

    /// Updates the associated journal `TextView` with the contents of the unit's journal log.
    fn update_journal(&self, grabbing: EventGrabbing) {
        if !self.visible_on_page.get() {
            return;
        }

        let binding = self.unit.borrow();
        let Some(unit_ref) = binding.as_ref() else {
            info!("No unit file");
            self.panel_stack.set_visible_child_name(PANEL_EMPTY);
            return;
        };

        //self.unit_journal_loaded.set(true); // maybe wait at the full loaded
        let unit = unit_ref.clone();
        let journal_refresh_button = self.journal_refresh_button.clone();
        let oldest_to_recent = self.older_to_recent.get();
        let journal_max_events_batch_size = PREFERENCES.journal_max_events_batch_size();
        let panel_stack = self.panel_stack.clone();
        let boot_filter = self.boot_filter.borrow().clone();
        let from_time = self.from_time.get();
        let most_recent_time = self.most_recent_time.get();
        let journal_panel = self.obj().clone();

        let text_buffer = {
            let text_view = self.journal_text_view.borrow();
            text_view.buffer()
        };

        let is_dark = self.is_dark.get();
        let journal_color = PREFERENCES.journal_colors();

        debug!("Call from time {:?}", from_time);
        debug!("grabbing {:?}", grabbing);

        let range = if grabbing == EventGrabbing::Newer {
            if !oldest_to_recent {
                EventRange {
                    oldest_first: oldest_to_recent,
                    batch_size: 0,
                    begin: None,
                    end: most_recent_time,
                }
            } else {
                EventRange::basic(oldest_to_recent, journal_max_events_batch_size, from_time)
            }
        } else {
            EventRange::basic(oldest_to_recent, journal_max_events_batch_size, from_time)
        };

        debug!("range {:?}", range);

        glib::spawn_future_local(async move {
            panel_stack.set_visible_child_name(PANEL_SPINNER);
            journal_refresh_button.set_sensitive(false);
            let events: JournalEventChunk = gio::spawn_blocking(move || {
                match systemd::get_unit_journal(&unit, boot_filter, range) {
                    Ok(journal_output) => journal_output,
                    Err(error) => {
                        warn!("Journal Events Error {:?}", error);
                        JournalEventChunk::error()
                    }
                }
            })
            .await
            .expect("Task needs to finish successfully.");

            let size = events.len();

            if from_time.is_none() {
                text_buffer.set_text("");
            }

            if !oldest_to_recent {
                if let Some(journal_event) = events.first() {
                    let time = journal_event.timestamp;
                    journal_panel.set_most_recent_time(time)
                }
            }

            let text_iter = if grabbing == EventGrabbing::Newer && !oldest_to_recent {
                text_buffer.start_iter()
            } else {
                text_buffer.end_iter()
            };

            let mut writer = UnitInfoWriter::new(text_buffer, text_iter, is_dark);
            for journal_event in events.iter() {
                fill_journal_event(journal_event, &mut writer, journal_color);
            }

            info!("Finish added {size} journal events!");

            if let Some(journal_event) = events.last() {
                let from_time = journal_event.timestamp;
                journal_panel.set_from_time(Some(from_time));
            }

            let panel = if writer.char_count() <= 0 {
                PANEL_EMPTY
            } else {
                PANEL_JOURNAL
            };

            journal_refresh_button.set_sensitive(true);

            panel_stack.set_visible_child_name(panel);
        });
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

    pub(super) fn refresh_panels(&self) {
        self.update_journal(EventGrabbing::Default)
    }

    pub(super) fn set_inter_message(&self, action: &InterPanelMessage) {
        match *action {
            InterPanelMessage::IsDark(is_dark) => self.set_dark(is_dark),
            InterPanelMessage::FontProvider(old, new) => {
                let text_view = self.journal_text_view.borrow();
                set_text_view_font(old, new, &text_view);
            }
            InterPanelMessage::PanelVisible(visible) => self.set_visible_on_page(visible),
            _ => {}
        }
    }

    pub(super) fn set_from_time(&self, from_time: Option<u64>) {
        info!("From time {:?}", from_time);
        self.from_time.set(from_time);
    }

    pub(super) fn set_oldest(&self, time: u64) {
        let max_time = if let Some(oldest_time) = self.most_recent_time.get() {
            oldest_time.max(time)
        } else {
            time
        };

        self.most_recent_time.set(Some(max_time));
    }

    fn set_dark(&self, is_dark: bool) {
        self.is_dark.set(is_dark);
    }

    fn new_text_view(&self) {
        let tv: gtk::TextView = gtk::TextView::builder().build();
        self.scrolled_window.set_child(Some(&tv));
        self.journal_text_view.replace(tv);
        self.from_time.set(None);
        //self.unit_journal_loaded.set(false);
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

#[glib::derived_properties]
impl ObjectImpl for JournalPanelImp {
    fn constructed(&self) {
        self.parent_constructed();

        self.new_text_view();

        self.older_to_recent.set(true);
    }
}
impl WidgetImpl for JournalPanelImp {}

impl BoxImpl for JournalPanelImp {}

#[derive(Debug, PartialEq, Eq)]
enum BootIdValidation {
    Fail,
    Partial,
    Valid,
    Over,
}

fn validate_boot_id(boot_id: &str) -> BootIdValidation {
    for c in boot_id.chars() {
        if c.is_ascii_digit() || matches!(c, 'a'..='f') {
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

/// When outputting to a tty, lines are colored according to priority:
///        lines of level ERROR and higher  3-1
///                  are colored red; lines of level
///                  WARNING are colored yellow; 4
///                  lines of level NOTICE are highlighted; 5
///                  lines of level INFO are displayed normally; lines of level  6
///                  DEBUG are colored grey.
///
fn fill_journal_event(
    journal_event: &JournalEvent,
    writer: &mut UnitInfoWriter,
    journal_color: bool,
) {
    writer.insert(&journal_event.prefix);

    let priority = if journal_color {
        journal_event.priority
    } else {
        6
    };

    match priority {
        0..=3 => pad_lines(writer, journal_event, UnitInfoWriter::insert_red),
        4 => pad_lines(writer, journal_event, UnitInfoWriter::insert_yellow),
        5 => pad_lines(writer, journal_event, UnitInfoWriter::insert_bold),
        6 => pad_lines(writer, journal_event, UnitInfoWriter::insert),
        7 => pad_lines(writer, journal_event, UnitInfoWriter::insert_grey),
        BOOT_IDX => pad_lines(writer, journal_event, UnitInfoWriter::insert_bold),

        _ => {
            warn!("Priority {priority} not handeled")
        }
    };
    writer.newline();
}

fn pad_lines(
    writer: &mut UnitInfoWriter,
    journal_event: &JournalEvent,
    inserter: impl Fn(&mut UnitInfoWriter, &str),
) {
    let mut lines = journal_event.message.lines();

    if let Some(line) = lines.next() {
        inserter(writer, line);
    }

    let mut space_padding = String::new();
    for line in lines {
        if space_padding.is_empty() {
            let bytes = vec![b' '; journal_event.prefix.len()];
            space_padding = String::from_utf8(bytes).expect("No issues");
        }

        writer.newline();
        writer.insert(&space_padding);
        inserter(writer, line);
    }
}

#[derive(PartialEq, Debug)]
enum EventGrabbing {
    // Older,
    Newer,
    Default,
    //None,
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
