use crate::{
    consts::{
        ACTION_WIN_KEY_JOURNAL_WRAP_WORD, APP_ACTION_LIST_BOOT, CLASS_ERROR, CLASS_SUCCESS,
        CLASS_WARNING, SETTING_FIND_IN_TEXT_OPEN,
    },
    systemd::{
        BootFilter,
        data::UnitInfo,
        journal_data::{
            EventRange, JournalEvent, JournalEventChunk, JournalEventChunkInfo, WhatGrab,
        },
    },
    systemd_gui::{self},
    upgrade,
    utils::{
        font_management::set_text_view_font,
        more_colors::{Intensity, TermColor},
        palette,
        writer::UnitInfoWriter,
    },
    widget::{
        InterPanelMessage,
        app_window::AppWindow,
        journal::colorize::{self, Token},
        preferences::data::{
            KEY_PREF_JOURNAL_DISPLAY_FOLLOW, KEY_PREF_JOURNAL_DISPLAY_ORDER, PREFERENCES,
        },
        text_search::{self, TextSearchEntry},
    },
};
use gettextrs::pgettext;
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
    cell::{Cell, OnceCell, RefCell},
    thread,
};
use systemd::journal_data::BOOT_IDX;
use tracing::{debug, error, info, warn};

const PANEL_EMPTY: &str = "empty";
const PANEL_JOURNAL: &str = "journal";
/*const PANEL_SPINNER: &str = "spinner"; */

const ASCD: &str = "view-sort-ascending";
const DESC: &str = "view-sort-descending";

const KEY_ASCENDING: &str = "Ascending";
const KEY_DESCENDING: &str = "Descending";

#[derive(Default, Clone, Copy, Debug, PartialEq)]
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

    pub fn key(&self) -> &str {
        match self {
            JournalDisplayOrder::Ascending => KEY_ASCENDING,
            JournalDisplayOrder::Descending => KEY_DESCENDING,
        }
    }

    pub fn from_key(key: &str) -> Self {
        match key {
            KEY_ASCENDING => JournalDisplayOrder::Ascending,
            KEY_DESCENDING => JournalDisplayOrder::Descending,
            _ => {
                warn!("Journal Display Order key {key:?} not found");
                JournalDisplayOrder::default()
            }
        }
    }
}

#[derive(Default, gtk::CompositeTemplate, glib::Properties)]
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
    follow_check: TemplateChild<gtk::CheckButton>,

    #[template_child]
    find_text_button: TemplateChild<gtk::ToggleButton>,

    #[template_child]
    journal_text_view: TemplateChild<gtk::TextView>,

    text_search_entry: OnceCell<TextSearchEntry>,

    visible_on_page: Cell<bool>,

    //list_store: RefCell<Option<gio::ListStore>>,
    unit: RefCell<Option<UnitInfo>>,

    boot_filter: RefCell<BootFilter>,

    time_old_new: Cell<Option<(u64, u64)>>,

    //old_to_recent_order: Cell<bool>,
    display_order: Cell<JournalDisplayOrder>,
    cancel_continuous_sender: RefCell<Option<std::sync::mpsc::Sender<()>>>,
    // settings: OnceCell<gio::Settings>,
    #[property(get, set= Self::set_wrap_word)]
    wrap_word: Cell<bool>,
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

        let settings = systemd_gui::new_settings();
        if let Err(e) = settings.set_string(KEY_PREF_JOURNAL_DISPLAY_ORDER, display.key()) {
            let key = display.key();
            warn!(
                "Can't set setting key {:?} value {:?} error {:?}",
                KEY_PREF_JOURNAL_DISPLAY_ORDER, key, e
            )
        }

        self.clean_refresh();
    }

    #[template_callback]
    fn boot_id_text_change(&self, entry: &adw::EntryRow) {
        let text = entry.text();
        info!("boot id entry_changed {text}");
    }

    #[template_callback]
    fn journal_menu_popover_closed(&self) {
        info!("journal_menu_popover_closed");
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
        info!("scwin_edge_overshot {position:?}");

        self.on_position(position);
    }

    #[template_callback]
    fn scwin_edge_reached(&self, position: gtk::PositionType) {
        info!("scwin_edge_reached {position:?}");

        self.on_position(position);
    }

    #[template_callback]
    fn list_boots_clicked(&self, button: gtk::Button) {
        if let Err(e) = button.activate_action(APP_ACTION_LIST_BOOT, None) {
            warn!("Send action Error : {e:?}");
        }
    }

    fn on_position(&self, position: gtk::PositionType) {
        let display_order = self.display_order.get();
        info!("Call for new position: {position:?}, display order: {display_order:?}");

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
    pub fn set_text_search_entry(&self, text_search_entry: &TextSearchEntry) {
        let _ = self.text_search_entry.set(text_search_entry.clone());

        // text_search_entry.set_text_view(self.unit_status_textview.as_ref());
    }

    pub(super) fn register(&self, app_window: &AppWindow) {
        let settings = systemd_gui::new_settings();
        let action = settings.create_action(&ACTION_WIN_KEY_JOURNAL_WRAP_WORD[4..]);

        app_window.add_action(&action);

        let wrap = settings.boolean(&ACTION_WIN_KEY_JOURNAL_WRAP_WORD[4..]);
        self.set_wrap_word(wrap);
    }

    fn set_visible_on_page(&self, visible: bool) {
        debug!("set_visible_on_page val {visible}");
        self.visible_on_page.set(visible);

        self.update_journal_according_to_display_order();

        if visible && let Some(text_search_entry) = self.text_search_entry.get() {
            text_search_entry.set_text_view(&self.journal_text_view);
        }
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

        //Assume that the ne unit is not None
        if old_unit.is_none_or(|o_unit| o_unit.primary() != unit.primary()) {
            self.new_text_view();
            self.set_or_send_cancelling(None);
            self.update_journal_according_to_display_order(); //TODO CHECK if needed to be include tin if clause 
        }
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
        debug!("BEGIN update_journal {grabbing:?}");
        if !self.visible_on_page.get() {
            debug!("not visible --> quit");
            return;
        }

        let sender_op = self.cancel_continuous_sender.borrow();
        if sender_op.is_some() && grabbing == WhatGrab::Newer {
            info!("Under tail management for newer event --> quit");
            return;
        }

        let binding = self.unit.borrow();
        let Some(unit) = binding.as_ref() else {
            info!("No unit file");
            self.panel_stack.set_visible_child_name(PANEL_EMPTY);
            return;
        };

        //self.unit_journal_loaded.set(true); // maybe wait at the full loaded

        let journal_max_events_batch_size: usize =
            PREFERENCES.journal_max_events_batch_size() as usize;
        //let panel_stack = self.panel_stack.clone();
        let boot_filter = self.boot_filter.borrow().clone();
        //let from_time = self.from_time.get();
        //let most_recent_time = self.most_recent_time.get();

        let (oldest_event_time, newest_event_time) = self
            .time_old_new
            .get()
            .map_or_else(|| (None, None), |(a, b)| (Some(a), Some(b)));

        debug!("Call from time old {oldest_event_time:?} new {newest_event_time:?}");

        let range = EventRange::new(
            grabbing,
            journal_max_events_batch_size,
            oldest_event_time,
            newest_event_time,
        );

        info!(
            "Journal unit {:?} boot filter \"{boot_filter:?}\" Range {range:#?}",
            unit.primary()
        );

        let journal_panel = self.obj().downgrade();
        let journal_refresh_button = self.journal_refresh_button.downgrade();
        let level = unit.dbus_level();
        let primary_name = unit.primary();

        glib::spawn_future_local(async move {
            let journal_panel = upgrade!(journal_panel);
            let journal_refresh_button = upgrade!(journal_refresh_button);

            //panel_stack.set_visible_child_name(PANEL_SPINNER);
            journal_refresh_button.set_sensitive(false);
            let boot_filter2 = boot_filter.clone();

            let journal_events: JournalEventChunk = gio::spawn_blocking(move || {
                let message_max_char = PREFERENCES.journal_event_max_size() as usize;
                let timestamp_style = PREFERENCES.timestamp_style();
                match systemd::get_unit_journal(
                    primary_name,
                    level,
                    boot_filter,
                    range,
                    message_max_char,
                    timestamp_style,
                ) {
                    Ok(journal_output) => journal_output,
                    Err(error) => {
                        warn!("Journal Events Error {error:?}");
                        JournalEventChunk::error(grabbing)
                    }
                }
            })
            .await
            .expect("Task needs to finish successfully.");

            let journal_panel_imp = journal_panel.imp();
            journal_panel_imp.handle_journal_events(&journal_events);

            //TODO better check all cases
            match journal_events.info() {
                JournalEventChunkInfo::NoMore
                    if boot_filter2 == BootFilter::Current || boot_filter2 == BootFilter::All =>
                {
                    journal_panel_imp.set_continuous_marker();
                }
                JournalEventChunkInfo::ChunkMaxReached
                    if journal_panel_imp.display_order.get() == JournalDisplayOrder::Ascending
                        && (boot_filter2 == BootFilter::Current
                            || boot_filter2 == BootFilter::All) =>
                {
                    journal_panel_imp.set_continuous_marker();
                }
                JournalEventChunkInfo::Error => {
                    warn!("Journal Events Chunk {:?}", journal_events.what_grab)
                }
                _ => {}
            };
        });
    }

    fn set_continuous_marker(&self) {
        if self.follow_check.is_active() {
            // call thread
            self.continuous_entry();
        }
    }

    pub fn append_journal_event(&self, journal_event: JournalEventChunk) {
        self.handle_journal_events(&journal_event);
    }

    fn handle_journal_events(&self, journal_events: &JournalEventChunk) {
        let text_buffer = self.journal_text_view.buffer();

        let display_order = self.display_order.get();

        let times = journal_events.times();
        self.set_times(times);

        let what_grab = journal_events.what_grab;

        let (text_iter, journal_events_iter): (
            gtk::TextIter,
            Box<dyn Iterator<Item = &JournalEvent>>,
        ) = match (what_grab, display_order) {
            (WhatGrab::Newer, JournalDisplayOrder::Ascending) => {
                (text_buffer.start_iter(), Box::new(journal_events.iter()))
            }
            (WhatGrab::Newer, JournalDisplayOrder::Descending) => {
                (text_buffer.end_iter(), Box::new(journal_events.iter()))
            }
            (WhatGrab::Older, JournalDisplayOrder::Ascending) => {
                (text_buffer.end_iter(), Box::new(journal_events.iter()))
            }
            (WhatGrab::Older, JournalDisplayOrder::Descending) => (
                text_buffer.start_iter(),
                Box::new(journal_events.iter().rev()),
            ),
        };

        // dbg!(
        //     self.follow_check.is_active(),
        //     display_order,
        //     journal_events.what_grab
        // );

        let mut writer = UnitInfoWriter::new(text_buffer, text_iter);
        const LEFT: &str = "left";
        const RIGHT: &str = "right";

        let left_mark = match writer.buffer.mark(LEFT) {
            Some(mark) => {
                writer.buffer.move_mark(&mark, &text_iter);
                mark
            }
            None => {
                let mark = gtk::TextMark::new(Some(LEFT), true);
                writer.buffer.add_mark(&mark, &text_iter);
                mark
            }
        };

        let right_mark = match writer.buffer.mark(RIGHT) {
            Some(mark) => {
                writer.buffer.move_mark(&mark, &text_iter);
                mark
            }
            None => {
                let mark = gtk::TextMark::new(Some(RIGHT), false);
                writer.buffer.add_mark(&mark, &text_iter);
                mark
            }
        };

        let journal_color = PREFERENCES.journal_colors();
        let mut journal_filler = JournalFiller::new(journal_color);

        for journal_event in journal_events_iter {
            journal_filler.fill_journal_event(journal_event, &mut writer);
        }

        info!("Finish adding {} journal events!", journal_events.len());

        if writer.char_count() <= 0 {
            self.panel_stack.set_visible_child_name(PANEL_EMPTY);
        } else if let Some(child_name) = self.panel_stack.visible_child_name()
            && child_name.as_str() == PANEL_JOURNAL
        {
            //Do nothing
        } else {
            self.panel_stack.set_visible_child_name(PANEL_JOURNAL);
        }

        self.journal_refresh_button.set_sensitive(true);
        //TODO put  a load notification
        //TODO fix PgDown annoying sound

        let start_iter = writer.buffer.iter_at_mark(&left_mark);
        let end_iter = writer.buffer.iter_at_mark(&right_mark);

        if let Some(text_search_entry) = self.text_search_entry.get() {
            text_search_entry.new_added_text(&writer.buffer, start_iter, end_iter);
        }

        if what_grab == WhatGrab::Newer && self.follow_check.is_active() {
            let mut end_iter = writer.buffer.end_iter();
            //go to the beging of the line
            end_iter.set_line_offset(0);

            const SCROLL: &str = "scroll";
            let scroll = match writer.buffer.mark(SCROLL) {
                Some(mark) => {
                    writer.buffer.move_mark(&mark, &end_iter);
                    mark
                }
                None => {
                    let mark = gtk::TextMark::new(Some(SCROLL), true);
                    writer.buffer.add_mark(&mark, &end_iter);
                    mark
                }
            };

            let this = self.obj().clone();
            glib::spawn_future_local(async move {
                let text_view = this.imp().journal_text_view.get();
                info!("scroll to {:?}", display_order);
                match display_order {
                    JournalDisplayOrder::Ascending => {
                        // text_view.scroll_to_mark(&left_mark, 0.0, true, 0.0, 1.0);
                        text_view.scroll_mark_onscreen(&left_mark);
                    }
                    JournalDisplayOrder::Descending => {
                        text_view.scroll_mark_onscreen(&scroll);
                    }
                }
            });
        }
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

        let unit_name = unit.primary();
        let level = unit.dbus_level();
        thread::spawn(move || {
            let message_max_char = PREFERENCES.journal_event_max_size() as usize;
            let timestamp_style = PREFERENCES.timestamp_style();
            systemd::get_unit_journal_continuous(
                unit_name,
                level,
                range,
                journal_continuous_receiver,
                sender,
                message_max_char,
                timestamp_style,
                super::check_for_new_journal_entry,
            )
        });
    }

    fn set_or_send_cancelling(&self, cancel_sender: Option<std::sync::mpsc::Sender<()>>) {
        let sender_op = self.cancel_continuous_sender.replace(cancel_sender);
        if let Some(cancel_continuous_sender) = sender_op {
            let res = cancel_continuous_sender.send(());
            if res.is_err() {
                warn!("Error close thread sender")
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
            InterPanelMessage::FontProvider(old, new) => {
                let text_view = self.journal_text_view.get();
                set_text_view_font(*old, *new, &text_view);
            }
            InterPanelMessage::PanelVisible(visible) => self.set_visible_on_page(*visible),
            InterPanelMessage::JournalFilterBoot(boot_filter) => {
                self.update_boot_filter(boot_filter.clone());
            }
            InterPanelMessage::UnitChange(unit) => {
                self.set_unit(*unit);
            }
            InterPanelMessage::Refresh(_) => {
                self.refresh_panels();
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

    fn new_text_view(&self) {
        debug!("new_text_view");
        self.time_old_new.set(None);

        let buffer = self.journal_text_view.buffer();

        //remove any marks

        //clear logs
        buffer.set_text("");
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

    pub(crate) fn focus_text_search(&self) {
        // text_search::focus_on_text_entry(&self.text_search_bar)
    }

    fn set_wrap_word(&self, wrap: bool) {
        if wrap {
            self.journal_text_view
                .set_wrap_mode(gtk::WrapMode::WordChar)
        } else {
            self.journal_text_view.set_wrap_mode(gtk::WrapMode::None)
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

        let settings = systemd_gui::new_settings();
        settings
            .bind(
                KEY_PREF_JOURNAL_DISPLAY_FOLLOW,
                &self.follow_check.clone(),
                "active",
            )
            .build();

        settings
            .bind(
                &ACTION_WIN_KEY_JOURNAL_WRAP_WORD[4..],
                self.obj().as_ref(),
                "wrap-word",
            )
            .build();

        let display_order = settings.string(KEY_PREF_JOURNAL_DISPLAY_ORDER);
        let display_order = JournalDisplayOrder::from_key(&display_order);

        let sort_toggle_button_content = self
            .journal_toggle_sort_button
            .child()
            .and_downcast::<adw::ButtonContent>()
            .unwrap();

        self.display_order.set(display_order);
        let (label, icon) = display_order.label_icon();
        sort_toggle_button_content.set_icon_name(icon);
        sort_toggle_button_content.set_label(label);

        settings
            .bind(
                &SETTING_FIND_IN_TEXT_OPEN[4..],
                &self.find_text_button.get(),
                "active",
            )
            .build();

        let menu = gio::Menu::new();
        let section_menu = gio::Menu::new();
        let find_text_mi = text_search::create_menu_item();
        section_menu.append_item(&find_text_mi);
        let menu_label = pgettext("menu", "Wrap Word");
        let wrap_word_toggle_menu = gio::MenuItem::new(Some(&menu_label), None);
        wrap_word_toggle_menu
            .set_action_and_target_value(Some(ACTION_WIN_KEY_JOURNAL_WRAP_WORD), None);
        section_menu.append_item(&wrap_word_toggle_menu);
        menu.append_section(None, &section_menu);
        self.journal_text_view.set_extra_menu(Some(&menu));

        let journal_panel = self.obj().clone();
        self.follow_check.connect_active_notify(move |button| {
            let active = button.is_active();
            info!("Follow: {active}");

            if active {
                journal_panel.imp().continuous_entry();
            } else {
                journal_panel.imp().set_or_send_cancelling(None);
            }
        });
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

struct JournalFiller {
    token_buffer: Vec<Token>,
    red: [Token; 2],
    yellow: [Token; 2],
    bold: [Token; 1],
    grey: [Token; 1],
    empty: [Token; 0],
    journal_color: bool,
}

impl JournalFiller {
    fn new(journal_color: bool) -> Self {
        let red = TermColor::from(palette::red());
        let red = [Token::FgColor(red), Token::Intensity(Intensity::Bold)];

        let yellow = TermColor::from(palette::yellow());
        let yellow = [Token::FgColor(yellow), Token::Intensity(Intensity::Bold)];

        let bold = [Token::Intensity(Intensity::Bold)];

        let grey = TermColor::from(palette::grey());
        let grey = [Token::FgColor(grey)];

        Self {
            token_buffer: vec![],
            red,
            yellow,
            bold,
            grey,
            empty: [],
            journal_color,
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
    fn fill_journal_event(&mut self, journal_event: &JournalEvent, writer: &mut UnitInfoWriter) {
        writer.insert(&journal_event.prefix);

        let priority_format = if self.journal_color {
            let tokens: &[Token] = match journal_event.priority {
                0..=3 => &self.red,
                4 => &self.yellow,
                5 => &self.bold,
                6 => &self.empty,
                7 => &self.grey,
                BOOT_IDX => &self.bold,

                _ => {
                    warn!("Priority {} not handeled", journal_event.priority);
                    &self.empty
                }
            };
            tokens
        } else {
            &self.empty
        };

        let mut lines = journal_event.message.lines();

        if let Some(line) = lines.next() {
            if self.journal_color {
                colorize::write(writer, line, &mut self.token_buffer, priority_format);
            } else {
                writer.insert(line);
            }
        }

        for line in lines {
            writer.newline();

            let space_padding = " ".repeat(journal_event.prefix.len());
            writer.insert(&space_padding);
            if self.journal_color {
                colorize::write(writer, line, &mut self.token_buffer, priority_format);
            } else {
                writer.insert(line);
            }
        }
        writer.newline();
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

            assert_eq!(res, answer, "boot_id {boot_id}");
        }
    }
}
