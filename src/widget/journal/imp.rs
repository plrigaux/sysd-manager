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

use std::{
    cell::{Cell, RefCell},
    thread,
};

use log::{debug, error, info, warn};

use crate::{
    consts::APP_ACTION_LIST_BOOT,
    systemd::{
        self, BootFilter,
        data::UnitInfo,
        journal::BOOT_IDX,
        journal_data::{
            EventRange, JournalEvent, JournalEventChunk, JournalEventChunkInfo, WhatGrab,
        },
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

#[derive(Default, Clone, Copy, Debug)]
enum JournalDisplayOrder {
    /// Bottom oldests -  Top most recent events  
    Ascending, // 1 2 3 4

    /// Bottom newest events -  Top oldests events
    #[default]
    Descending, // 4 3 2 1
}

impl JournalDisplayOrder {
    pub fn label_icon(&self) -> (&str, &str) {
        match self {
            JournalDisplayOrder::Ascending => ("Ascending", ASCD),
            JournalDisplayOrder::Descending => ("Descending", DESC),
        }
    }
}

#[derive(Default, glib::Properties, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/journal_panel.ui")]
#[properties(wrapper_type = super::JournalPanel)]
pub struct JournalPanelImp {
    #[template_child]
    journal_refresh_button: TemplateChild<gtk::Button>,

    #[template_child]
    panel_stack: TemplateChild<adw::ViewStack>,

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

    #[template_child]
    continuous_switch: TemplateChild<gtk::Switch>,

    visible_on_page: Cell<bool>,

    //unit_journal_loaded: Cell<bool>,

    //list_store: RefCell<Option<gio::ListStore>>,
    #[property(get, set=Self::set_unit, nullable)]
    unit: RefCell<Option<UnitInfo>>,

    is_dark: Cell<bool>,

    boot_filter: RefCell<BootFilter>,

    time_old_new: Cell<Option<(u64, u64)>>,

    journal_text_view: RefCell<gtk::TextView>,

    //old_to_recent_order: Cell<bool>,
    display_order: Cell<JournalDisplayOrder>,
    cancel_continuous_sender: RefCell<Option<std::sync::mpsc::Sender<()>>>,
}

#[gtk::template_callbacks]
impl JournalPanelImp {
    #[template_callback]
    fn refresh_journal_clicked(&self, _button: &gtk::Button) {
        info!("journal refresh button click");
        self.clean_refresh();
    }

    #[template_callback]
    fn toggle_sort_clicked(&self, button: &gtk::Button) {
        info!("toggle_sort_clicked");

        let child = button.child().and_downcast::<adw::ButtonContent>().unwrap();

        let icon_name = child.icon_name();

        let display = if icon_name == ASCD {
            JournalDisplayOrder::Descending
        } else {
            JournalDisplayOrder::Ascending
        };

        let (label, icon) = display.label_icon();
        child.set_icon_name(icon);
        child.set_label(label);
        self.display_order.set(display);

        self.clean_refresh();
    }

    #[template_callback]
    fn boot_id_text_change(&self, entry: &adw::EntryRow) {
        let text = entry.text();
        info!("boot id entry_changed {}", text);
    }

    #[template_callback]
    fn journal_menu_popover_closed(&self) {
        info!("journal_menu_popover_closed");

        /*        let boot_filter_op = if self.journal_boot_all_button.has_css_class(CLASS_SUCCESS) {
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
        }; */

        /*  if let Some(boot_filter) = boot_filter_op {
            let replaced = self.boot_filter.replace(boot_filter.clone());

            if replaced != boot_filter {
                //filter updated
                self.update_journal(EventGrabbing::Default);
            }
        } */
    }

    #[template_callback]
    fn journal_menu_popover_activate_default(&self) {
        info!("journal_menu_popover_activate_default");
    }

    #[template_callback]
    fn journal_menu_popover_show(&self) {
        info!("journal_menu_popover_show");

        self.clear_boot_id_style();

        let boot_filter = { self.boot_filter.borrow().clone() };

        match boot_filter {
            BootFilter::Current => self
                .journal_boot_current_button
                .add_css_class(CLASS_SUCCESS),
            BootFilter::All => self.journal_boot_all_button.add_css_class(CLASS_SUCCESS),
            BootFilter::Id(boot_id) => {
                self.journal_boot_id_entry.set_text(&boot_id);
                self.journal_boot_id_entry.add_css_class(CLASS_SUCCESS);
            }
        }
    }

    #[template_callback]
    fn journal_boot_current_button_clicked(&self) {
        info!("journal_boot_current_button_clicked");
        self.clear_boot_id_style();
        self.journal_boot_current_button
            .add_css_class(CLASS_SUCCESS);
        self.update_boot_filter(BootFilter::Current);
    }

    #[template_callback]
    fn journal_boot_all_button_clicked(&self) {
        info!("journal_boot_all_button_clicked");
        self.clear_boot_id_style();
        self.journal_boot_all_button.add_css_class(CLASS_SUCCESS);
        self.update_boot_filter(BootFilter::All);
    }

    #[template_callback]
    fn continuous_switch_state_set(&self, state: bool) -> bool {
        info!("continuous switch state {}", state);

        if state {
            self.continuous_entry()
        } else {
            JournalPanelImp::set_or_send_cancelling(self, None);
        }

        true //TRUE to stop the signal emission.
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

    #[template_callback]
    fn list_boots_clicked(&self, button: gtk::Button) {
        if let Err(e) = button.activate_action(APP_ACTION_LIST_BOOT, None) {
            warn!("Send action Error : {:?}", e);
        }
    }

    fn on_position(&self, position: gtk::PositionType) {
        let display_order = self.display_order.get();
        info!(
            "call for new {:?}, display order {:?}",
            position, display_order
        );

        match (position, display_order) {
            (gtk::PositionType::Bottom, JournalDisplayOrder::Descending) => {
                self.update_journal(WhatGrab::Newer)
            }

            (gtk::PositionType::Bottom, JournalDisplayOrder::Ascending) => {
                self.update_journal(WhatGrab::Older)
            }
            (gtk::PositionType::Top, JournalDisplayOrder::Descending) => {
                self.update_journal(WhatGrab::Older)
            }

            (gtk::PositionType::Top, JournalDisplayOrder::Ascending) => {
                self.update_journal(WhatGrab::Newer)
            }
            _ => {}
        }
    }

    fn clear_boot_id_style(&self) {
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

    #[template_callback]
    fn journal_boot_id_entry_activated(&self, _entry: adw::EntryRow) {
        info!("journal_boot_id_entry_activated");
        self.set_boot_id_style();
    }

    /*     #[template_callback]
    fn journal_boot_id_entry_apply(&self, _entry: adw::EntryRow) {
        info!("journal_boot_id_entry_apply");
        self.set_boot_id_style();
    } */
}

impl JournalPanelImp {
    fn set_visible_on_page(&self, value: bool) {
        debug!("set_visible_on_page val {value}");
        self.visible_on_page.set(value);

        self.update_journal_according_to_display_order();
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

            JournalPanelImp::set_or_send_cancelling(self, None);
        }

        self.update_journal_according_to_display_order();
    }

    fn update_journal_according_to_display_order(&self) {
        let grabber = match self.display_order.get() {
            JournalDisplayOrder::Ascending => WhatGrab::Older,
            JournalDisplayOrder::Descending => WhatGrab::Newer,
        };

        self.update_journal(grabber);
    }

    /// Updates the associated journal `TextView` with the contents of the unit's journal log.
    fn update_journal(&self, grabbing: WhatGrab) {
        if !self.visible_on_page.get() {
            info!("not visible --> quit");
            return;
        }

        let sender_op = self.cancel_continuous_sender.borrow();
        if sender_op.is_some() {
            info!("under tail management --> quit");
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
        let journal_max_events_batch_size: usize =
            PREFERENCES.journal_max_events_batch_size() as usize;
        let panel_stack = self.panel_stack.clone();
        let boot_filter = self.boot_filter.borrow().clone();
        //let from_time = self.from_time.get();
        //let most_recent_time = self.most_recent_time.get();

        let (oldest_event_time, newest_event_time) = self
            .time_old_new
            .get()
            .map_or_else(|| (None, None), |(a, b)| (Some(a), Some(b)));

        let journal_panel = self.obj().clone();

        debug!(
            "Call from time old {:?} new {:?}",
            oldest_event_time, newest_event_time
        );

        debug!("grabbing {:?}", grabbing);

        let range = EventRange::new(
            grabbing,
            journal_max_events_batch_size,
            oldest_event_time,
            newest_event_time,
        );

        debug!("range {:?}", range);

        info!("boot filter {:?}", boot_filter);

        glib::spawn_future_local(async move {
            panel_stack.set_visible_child_name(PANEL_SPINNER);
            journal_refresh_button.set_sensitive(false);
            let boot_filter2 = boot_filter.clone();
            let journal_events: JournalEventChunk = gio::spawn_blocking(move || {
                match systemd::get_unit_journal(&unit, boot_filter, range) {
                    Ok(journal_output) => journal_output,
                    Err(error) => {
                        warn!("Journal Events Error {:?}", error);
                        JournalEventChunk::error(grabbing)
                    }
                }
            })
            .await
            .expect("Task needs to finish successfully.");

            journal_panel.imp().handle_journal_events(&journal_events);

            match journal_events.info() {
                JournalEventChunkInfo::NoMore if boot_filter2 == BootFilter::Current => {
                    let journal_panel_imp = journal_panel.imp();
                    journal_panel_imp.continuous_switch.set_state(true);
                    if journal_panel_imp.continuous_switch.is_active() {
                        // call thread
                        journal_panel_imp.continuous_entry();
                    }
                }
                JournalEventChunkInfo::Error => {}
                _ => journal_panel.imp().continuous_switch.set_state(false),
            };
        });
    }

    pub fn append_journal_event(&self, journal_event: JournalEventChunk) {
        self.handle_journal_events(&journal_event);
    }

    fn handle_journal_events(&self, journal_events: &JournalEventChunk) {
        let size = journal_events.len();

        let text_buffer = {
            let text_view = self.journal_text_view.borrow();
            text_view.buffer()
        };

        if self.time_old_new.get().is_none() {
            text_buffer.set_text("");
        }

        let display_order = self.display_order.get();

        let times = journal_events.times();
        self.set_times(times);

        let text_iter = match (journal_events.what_grab, display_order) {
            (WhatGrab::Newer, JournalDisplayOrder::Ascending) => text_buffer.start_iter(),
            (WhatGrab::Newer, JournalDisplayOrder::Descending) => text_buffer.end_iter(),
            (WhatGrab::Older, JournalDisplayOrder::Ascending) => text_buffer.end_iter(),
            (WhatGrab::Older, JournalDisplayOrder::Descending) => text_buffer.start_iter(),
        };

        let is_dark = self.is_dark.get();
        let mut writer = UnitInfoWriter::new(text_buffer, text_iter, is_dark);
        let journal_color = PREFERENCES.journal_colors();
        for journal_event in journal_events.iter() {
            fill_journal_event(journal_event, &mut writer, journal_color);
        }

        info!("Finish added {size} journal events!");

        let panel = if writer.char_count() <= 0 {
            PANEL_EMPTY
        } else {
            PANEL_JOURNAL
        };

        self.journal_refresh_button.set_sensitive(true);
        self.panel_stack.set_visible_child_name(panel);
    }

    fn continuous_entry(&self) {
        let binding = self.unit.borrow();
        let Some(unit_ref) = binding.as_ref() else {
            info!("No unit file");
            return;
        };

        //self.unit_journal_loaded.set(true); // maybe wait at the full loaded
        let unit = unit_ref.clone();

        let journal_max_events_batch_size: usize =
            PREFERENCES.journal_max_events_batch_size() as usize;

        let (oldest_event_time, newest_event_time) = self
            .time_old_new
            .get()
            .map_or_else(|| (None, None), |(a, b)| (Some(a), Some(b)));

        let range = EventRange::new(
            WhatGrab::Newer,
            journal_max_events_batch_size,
            oldest_event_time,
            newest_event_time,
        );

        let (journal_continuous_sender, journal_continuous_receiver) = std::sync::mpsc::channel();

        let (sender, receiver) = std::sync::mpsc::channel();

        //let (sender1, receiver1) = glib::MainContext::channel();
        let journal_panel = self.obj().clone();
        super::GLOBAL.with(|global| {
            *global.borrow_mut() = Some((journal_panel, receiver));
        });

        self.set_or_send_cancelling(Some(journal_continuous_sender));

        thread::spawn(move || {
            let unit_name = unit.primary();
            let level = unit.dbus_level();
            systemd::get_unit_journal_continuous(
                unit_name,
                level,
                range,
                journal_continuous_receiver,
                sender,
            )
        });
    }

    fn set_or_send_cancelling(&self, cancel_sender: Option<std::sync::mpsc::Sender<()>>) {
        let sender_op = self.cancel_continuous_sender.replace(cancel_sender);
        if let Some(cancel_continuous_sender) = sender_op {
            let res = cancel_continuous_sender.send(());
            if res.is_err() {
                warn!("Error close thtread sender")
            }
            info!("Cancel journal trail")
        }
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
                self.clear_boot_id_style();
                self.journal_boot_id_entry.add_css_class(CLASS_SUCCESS);
                let boot_filter = BootFilter::Id(boot_id_text.to_string());
                self.update_boot_filter(boot_filter);
            }
            BootIdValidation::Over => {
                self.journal_boot_id_entry.remove_css_class(CLASS_SUCCESS);
                self.journal_boot_id_entry.remove_css_class(CLASS_ERROR);
                self.journal_boot_id_entry.add_css_class(CLASS_WARNING)
            }
        };
    }

    pub(super) fn refresh_panels(&self) {
        self.update_journal_according_to_display_order();
    }

    pub(super) fn set_inter_message(&self, action: &InterPanelMessage) {
        match action {
            InterPanelMessage::IsDark(is_dark) => self.set_dark(*is_dark),
            InterPanelMessage::FontProvider(old, new) => {
                let text_view = self.journal_text_view.borrow();
                set_text_view_font(*old, *new, &text_view);
            }
            InterPanelMessage::PanelVisible(visible) => self.set_visible_on_page(*visible),
            InterPanelMessage::JournalFilterBoot(boot_filter) => {
                self.update_boot_filter(boot_filter.clone());
            }
            _ => {}
        }
    }

    pub(super) fn set_times(&self, times: Option<(u64, u64)>) {
        let Some((new_oldest_time, new_recent_time)) = times else {
            return;
        };

        if let Some((old_oldest_time, old_recent_time)) = self.time_old_new.get() {
            let a = old_oldest_time.min(new_oldest_time);
            let b = old_recent_time.max(new_recent_time);
            self.time_old_new.set(Some((a, b)));
        } else {
            self.time_old_new
                .set(Some((new_oldest_time, new_recent_time)));
        }
    }

    fn set_dark(&self, is_dark: bool) {
        self.is_dark.set(is_dark);
    }

    fn new_text_view(&self) {
        info!("new_text_view");
        let tv: gtk::TextView = gtk::TextView::builder().build();
        self.scrolled_window.set_child(Some(&tv));
        self.journal_text_view.replace(tv);
        self.time_old_new.set(None);
        //self.unit_journal_loaded.set(false);
    }

    fn clean_refresh(&self) {
        self.new_text_view();
        self.set_or_send_cancelling(None);
        self.update_journal_according_to_display_order();
    }

    fn update_boot_filter(&self, boot_filter: BootFilter) {
        let replaced = self.boot_filter.replace(boot_filter.clone());

        if replaced != boot_filter {
            //filter updated
            self.clean_refresh();
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

#[glib::derived_properties]
impl ObjectImpl for JournalPanelImp {
    fn constructed(&self) {
        self.parent_constructed();

        self.new_text_view();

        let sort_toggle_button_content = self
            .journal_toggle_sort_button
            .child()
            .and_downcast::<adw::ButtonContent>()
            .unwrap();

        let display = self.display_order.get();
        let (label, icon) = display.label_icon();
        sort_toggle_button_content.set_icon_name(icon);
        sort_toggle_button_content.set_label(label);
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
