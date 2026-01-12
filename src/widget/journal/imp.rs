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
use systemd::journal_data::BOOT_IDX;

use std::{
    cell::{Cell, OnceCell, RefCell},
    thread,
};

use log::{debug, error, info, warn};

use crate::{
    consts::{APP_ACTION_LIST_BOOT, CLASS_ERROR, CLASS_SUCCESS, CLASS_WARNING},
    systemd::{
        self, BootFilter,
        data::UnitInfo,
        journal_data::{
            EventRange, JournalEvent, JournalEventChunk, JournalEventChunkInfo, WhatGrab,
        },
    },
    systemd_gui, upgrade,
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
        text_search::{self, TextSearchBar},
    },
};

const PANEL_EMPTY: &str = "empty";
const PANEL_JOURNAL: &str = "journal";
/*const PANEL_SPINNER: &str = "spinner"; */

const ASCD: &str = "view-sort-ascending";
const DESC: &str = "view-sort-descending";

const KEY_ASCENDING: &str = "Ascending";
const KEY_DESCENDING: &str = "Descending";

const TEXT_FIND_ACTION: &str = "unit_journal_text_find";

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

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/journal_panel.ui")]
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

    #[template_child]
    text_search_bar: TemplateChild<gtk::SearchBar>,

    #[template_child]
    find_text_button: TemplateChild<gtk::ToggleButton>,

    visible_on_page: Cell<bool>,

    //unit_journal_loaded: Cell<bool>,

    //list_store: RefCell<Option<gio::ListStore>>,
    unit: RefCell<Option<UnitInfo>>,

    is_dark: Cell<bool>,

    boot_filter: RefCell<BootFilter>,

    time_old_new: Cell<Option<(u64, u64)>>,

    journal_text_view: RefCell<gtk::TextView>,

    //old_to_recent_order: Cell<bool>,
    display_order: Cell<JournalDisplayOrder>,
    cancel_continuous_sender: RefCell<Option<std::sync::mpsc::Sender<()>>>,

    settings: OnceCell<gio::Settings>,
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

        if let Err(e) = self
            .settings
            .get()
            .expect("settings not none")
            .set_string(KEY_PREF_JOURNAL_DISPLAY_ORDER, display.key())
        {
            warn!(
                "Can't set setting key {:?} value {:?} error {:}",
                KEY_PREF_JOURNAL_DISPLAY_ORDER,
                display.key(),
                e
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
    fn continuous_switch_state_set(&self, active: bool, continuous_switch: &gtk::Switch) -> bool {
        info!("continuous switch state {active}");

        if active {
            if continuous_switch.state() {
                self.continuous_entry()
            }
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
        info!("call for new {position:?}, display order {display_order:?}");

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
    pub(super) fn register(&self, app_window: &AppWindow) {
        let text_search_bar = self.text_search_bar.clone();
        let daemon_reload_all_units_with_bus: gio::ActionEntry<AppWindow> =
            gio::ActionEntry::builder(TEXT_FIND_ACTION)
                .activate(
                    move |_app_window: &AppWindow,
                          _simple_action,
                          _variant: Option<&glib::Variant>| {
                        text_search_bar.set_search_mode(true);
                        if let Some(search) =
                            text_search_bar.child().and_downcast_ref::<TextSearchBar>()
                        {
                            search.grab_focus_on_search_entry();
                        }
                    },
                )
                .build();

        app_window.add_action_entries([daemon_reload_all_units_with_bus]);
    }

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
            "journal unit {:?} boot filter {boot_filter:?} Range {range:?}",
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
                _ => journal_panel_imp.continuous_switch.set_state(false),
            };
        });
    }

    fn set_continuous_marker(&self) {
        self.continuous_switch.set_state(true);
        if self.continuous_switch.is_active() {
            // call thread
            self.continuous_entry();
        }
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

        let mark_l = gtk::TextMark::new(None, true);
        let mark_r = gtk::TextMark::new(None, false);

        text_buffer.add_mark(&mark_l, &text_iter);
        text_buffer.add_mark(&mark_r, &text_iter);

        println!("1iter {:?}", text_iter.offset());
        let is_dark = self.is_dark.get();
        let mut writer = UnitInfoWriter::new(text_buffer, text_iter, is_dark);
        let journal_color = PREFERENCES.journal_colors();
        let mut journal_filler = JournalFiller::new(is_dark, journal_color);
        for journal_event in journal_events.iter() {
            journal_filler.fill_journal_event(journal_event, &mut writer);
        }

        info!("Finish added {size} journal events!");

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

        println!("2iter {:?}", writer.text_iterator.offset());

        let start_iter = writer.buffer.iter_at_mark(&mark_l);
        let end_iter = writer.buffer.iter_at_mark(&mark_r);
        text_search::new_added_text(&self.text_search_bar, &writer.buffer, start_iter, end_iter);
        writer.buffer.delete_mark(&mark_l);
        writer.buffer.delete_mark(&mark_r);
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
            InterPanelMessage::UnitChange(unit) => {
                self.set_unit(*unit);
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
        let tv: gtk::TextView = gtk::TextView::builder().editable(false).build();
        self.scrolled_window.set_child(Some(&tv));
        self.journal_text_view.replace(tv);
        self.time_old_new.set(None);
        self.continuous_switch.set_state(false);

        let text_view = self.journal_text_view.borrow();
        text_search::update_text_view(&self.text_search_bar, &text_view);
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

impl ObjectImpl for JournalPanelImp {
    fn constructed(&self) {
        self.parent_constructed();

        self.new_text_view();

        let settings = systemd_gui::new_settings();
        self.settings
            .set(settings.clone())
            .expect("Settings set once only");

        settings
            .bind(
                KEY_PREF_JOURNAL_DISPLAY_FOLLOW,
                &self.continuous_switch.clone(),
                "active",
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

        let text_view = self.journal_text_view.borrow();
        text_search::text_search_construct(
            &text_view,
            &self.text_search_bar,
            &self.find_text_button,
            TEXT_FIND_ACTION,
        );
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
    fn new(is_dark: bool, journal_color: bool) -> Self {
        let red = TermColor::from(palette::red(is_dark));
        let red = [Token::FgColor(red), Token::Intensity(Intensity::Bold)];

        let yellow = TermColor::from(palette::yellow(is_dark));
        let yellow = [Token::FgColor(yellow), Token::Intensity(Intensity::Bold)];

        let bold = [Token::Intensity(Intensity::Bold)];

        let grey = TermColor::from(palette::grey(is_dark));
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
