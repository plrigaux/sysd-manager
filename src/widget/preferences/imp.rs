use gettextrs::pgettext;
use gio::Settings;
use systemd::time_handling::TimestampStyle;

use super::data::{
    KEY_PREF_APP_FIRST_CONNECTION, KEY_PREF_JOURNAL_COLORS, KEY_PREF_JOURNAL_EVENT_MAX_SIZE,
    KEY_PREF_JOURNAL_EVENTS_BATCH_SIZE, KEY_PREF_STYLE_TEXT_FONT_FAMILY,
    KEY_PREF_STYLE_TEXT_FONT_SIZE, KEY_PREF_TIMESTAMP_STYLE, KEY_PREF_UNIT_FILE_LINE_NUMBERS,
    KEY_PREF_UNIT_FILE_STYLE_SCHEME, PREFERENCES,
};
use crate::widget::InterPanelMessage;
use crate::{
    consts::ADWAITA,
    systemd_gui::new_settings,
    utils::font_management::FONT_CONTEXT,
    widget::{
        app_window::AppWindow,
        preferences::{
            data::{
                KEY_PREF_UNIT_DESCRIPTION_WRAP, KEY_PREF_UNIT_LIST_DISPLAY_COLORS,
                KEY_PREF_UNIT_LIST_DISPLAY_SUMMARY,
            },
            drop_down_elem::{build_pane_orientation_selector, build_preferred_color_scheme},
            style_scheme::style_schemes,
        },
    },
};
use adw::{prelude::*, subclass::prelude::*};
use gtk::{
    StringObject, gio,
    glib::{self, BoolError},
    pango::{self, FontFace},
};
use log::{debug, error, info, warn};
use std::cell::{OnceCell, RefCell};
use strum::IntoEnumIterator;

#[derive(Debug, Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/preferences.ui")]
pub struct PreferencesDialogImpl {
    settings: OnceCell<Settings>,

    #[template_child]
    journal_colors: TemplateChild<gtk::Switch>,

    #[template_child]
    unit_file_line_numbers: TemplateChild<gtk::Switch>,

    #[template_child]
    unit_file_style: TemplateChild<adw::ComboRow>,

    #[template_child]
    pub preference_banner: TemplateChild<adw::Banner>,

    #[template_child]
    journal_max_events: TemplateChild<adw::SpinRow>,

    #[template_child]
    journal_event_max_size: TemplateChild<adw::SpinRow>,

    #[template_child]
    timestamp_style: TemplateChild<adw::ComboRow>,

    #[template_child]
    select_font_row: TemplateChild<adw::ActionRow>,

    #[template_child]
    unit_list_colors: TemplateChild<adw::SwitchRow>,

    #[template_child]
    preferred_color_scheme: TemplateChild<adw::ComboRow>,

    #[template_child]
    app_orientation: TemplateChild<adw::ComboRow>,

    #[template_child]
    unit_list_summay: TemplateChild<adw::SwitchRow>,

    #[template_child]
    unit_description_wrap: TemplateChild<adw::SwitchRow>,

    #[template_child]
    proxy_all_switch: TemplateChild<adw::SwitchRow>,

    #[template_child]
    proxy_clean_switch: TemplateChild<adw::SwitchRow>,

    #[template_child]
    proxy_freeze_switch: TemplateChild<adw::SwitchRow>,

    #[template_child]
    proxy_thaw_switch: TemplateChild<adw::SwitchRow>,

    #[template_child]
    proxy_enable_unit_file_switch: TemplateChild<adw::SwitchRow>,

    #[template_child]
    proxy_disable_unit_file_switch: TemplateChild<adw::SwitchRow>,

    #[template_child]
    proxy_create_dropin_switch: TemplateChild<adw::SwitchRow>,

    #[template_child]
    proxy_save_file_switch: TemplateChild<adw::SwitchRow>,

    #[template_child]
    proxy_revert_unit_file_switch: TemplateChild<adw::SwitchRow>,

    #[template_child]
    proxy_reload_daemon_switch: TemplateChild<adw::SwitchRow>,

    #[template_child]
    start_proxy_at_startup_switch: TemplateChild<adw::SwitchRow>,

    #[template_child]
    stop_proxy_at_close_switch: TemplateChild<adw::SwitchRow>,

    #[template_child]
    proxy_banner: TemplateChild<adw::Banner>,

    #[template_child]
    pref_proxy_page: TemplateChild<adw::PreferencesPage>,

    app_window: RefCell<Option<AppWindow>>,
}

#[gtk::template_callbacks]
impl PreferencesDialogImpl {
    #[template_callback]
    fn journal_switch_state_set(&self, state: bool) -> bool {
        info!("journal_colors_switch {state}");

        self.journal_colors.set_state(state);
        PREFERENCES.set_journal_colors(state);

        true
    }

    #[template_callback]
    fn journal_max_events_changed(&self, spin: adw::SpinRow) {
        let value32_parse = Self::get_spin_row_value("journal_events_changed", spin);

        PREFERENCES.set_journal_events_batch_size(value32_parse);
    }

    #[template_callback]
    fn journal_event_max_size_changed(&self, spin: adw::SpinRow) {
        let value32_parse = Self::get_spin_row_value("journal_event_max_size_changed", spin);

        PREFERENCES.set_journal_event_max_size(value32_parse);
    }

    #[template_callback]
    fn select_font_clicked(&self) {
        let filter = gtk::CustomFilter::new(move |object| {
            let Some(font_face) = object.downcast_ref::<FontFace>() else {
                error!("some wrong downcast_ref {object:?}");
                return false;
            };

            let font_familly = font_face.family();

            font_familly.is_monospace()
        });

        let font_dialog = gtk::FontDialog::builder()
            .modal(false)
            .filter(&filter)
            .title("Pick a Monospace Font")
            .build();

        let parent = self.app_window.borrow();
        let window = parent.as_ref().map(|w| w.clone());
        let select_font_row = self.select_font_row.clone();

        let font_description = FONT_CONTEXT.font_description();

        debug!(
            "FD {} family {:?} size {}",
            font_description.to_str(),
            font_description.family(),
            font_description.size() / pango::SCALE
        );

        warn!("FD {} ", font_description.to_str(),);

        font_dialog.choose_font(
            parent.as_ref(),
            Some(&font_description),
            None::<&gio::Cancellable>,
            move |result| match result {
                Ok(font_description) => {
                    let font_name = font_description.to_string();
                    info!("Selected Font {:?}", font_description.to_string());

                    PREFERENCES.set_font(&font_description);

                    if let Some(window) = window {
                        let action = InterPanelMessage::Font(Some(&font_description));

                        window.set_inter_message(&action);
                    }

                    select_font_row.set_subtitle(&font_name);
                }
                Err(e) => warn!("Select font error: {e:?}"),
            },
        );
    }

    #[template_callback]
    fn select_font_default(&self) {
        PREFERENCES.set_font_default();

        let window = self.app_window.borrow();
        if let Some(window) = window.as_ref() {
            let action = crate::widget::InterPanelMessage::Font(None);
            window.set_inter_message(&action);
        }

        let select_font_row = self.select_font_row.clone();

        glib::spawn_future_local(async move {
            gio::spawn_blocking(move || {})
                .await
                .expect("Task needs to finish successfully.");

            let font_description = FONT_CONTEXT.font_description();
            select_font_row.set_subtitle(&font_description.to_string());
        });
    }

    fn select_style_scheme(&self, vec: &Vec<&str>, style_scheme_id: &str) -> bool {
        if style_scheme_id.is_empty() {
            self.unit_file_style.set_selected(0);
            return true;
        }

        for (position, style_scheme_id_list) in vec.iter().enumerate() {
            if style_scheme_id == *style_scheme_id_list {
                self.unit_file_style.set_selected(position as u32);
                return true;
            }
        }
        false
    }
}

impl PreferencesDialogImpl {
    pub(super) fn set_app_window(&self, app_window: Option<&AppWindow>) {
        let Some(app_window) = app_window else {
            self.app_window.replace(None);
            return;
        };

        self.app_window.replace(Some(app_window.clone()));

        let window = app_window.clone();

        self.unit_file_style
            .connect_selected_item_notify(move |combo_box| {
                let selected_item_position = combo_box.selected();
                let selected_item = combo_box.selected_item();
                let Some(style_scheme) = selected_item else {
                    return;
                };

                let style_scheme = if selected_item_position == 0 {
                    glib::GString::new()
                } else {
                    style_scheme
                        .downcast::<StringObject>()
                        .expect("Needs to be TimestampStyle")
                        .string()
                };

                PREFERENCES.set_unit_file_style_scheme(&style_scheme);

                let style_scheme_op = if selected_item_position == 0
                /*"None"*/
                {
                    None
                } else {
                    Some(style_scheme.as_str())
                };

                let action = crate::widget::InterPanelMessage::NewStyleScheme(style_scheme_op);
                window.set_inter_message(&action);
            });
    }

    fn setup_settings(&self) {
        let settings = new_settings();
        {
            let settings1 = settings.clone();
            self.settings
                .set(settings1)
                .expect("`settings` should not be set before calling `setup_settings`.");
        }
    }

    fn settings(&self) -> &gio::Settings {
        self.settings
            .get()
            .expect("`settings` should be set in `setup_settings`.")
    }

    fn load_preferences_values(&self) {
        let settings = self.settings();

        let journal_colors = PREFERENCES.journal_colors();
        let is_app_first_connection = PREFERENCES.is_app_first_connection();

        let unit_file_line_numbers = settings.boolean(KEY_PREF_UNIT_FILE_LINE_NUMBERS);

        self.journal_colors.set_state(journal_colors);
        self.journal_colors.set_active(journal_colors);

        let journal_max_events = PREFERENCES.journal_max_events_batch_size();
        self.journal_max_events.set_value(journal_max_events as f64);

        let journal_event_max_size = PREFERENCES.journal_event_max_size();
        self.journal_event_max_size
            .set_value(journal_event_max_size as f64);

        self.unit_file_line_numbers
            .set_state(unit_file_line_numbers);
        self.unit_file_line_numbers
            .set_active(unit_file_line_numbers);

        self.preference_banner.set_revealed(is_app_first_connection);

        let timestamp_style = PREFERENCES.timestamp_style();
        self.timestamp_style.set_selected(timestamp_style as u32);

        let font_description = FONT_CONTEXT.font_description();
        self.select_font_row
            .set_subtitle(&font_description.to_string());
    }

    fn get_spin_row_value(var_name: &str, spin: adw::SpinRow) -> u32 {
        let value = spin.value();
        let text = spin.text();

        info!("{var_name} to {value:?} , text {text:?}");

        match text.parse::<u32>() {
            Ok(a) => a,
            Err(_e) => {
                info!("Parse error {text:?} to u32 do falback to f64");
                //spin.set_text(&value32.to_string());
                if value > f64::from(i32::MAX) {
                    u32::MAX
                } else if value < f64::from(i32::MIN) {
                    u32::MIN
                } else {
                    value.round() as u32
                }
            }
        }
    }

    fn save_preference_settings(&self) -> Result<(), BoolError> {
        let settings = self.settings();

        let app_first_connection = PREFERENCES.is_app_first_connection();
        settings.set_boolean(KEY_PREF_APP_FIRST_CONNECTION, app_first_connection)?;

        let journal_colors = PREFERENCES.journal_colors();
        settings.set_boolean(KEY_PREF_JOURNAL_COLORS, journal_colors)?;

        let journal_events_batch_size = PREFERENCES.journal_max_events_batch_size();
        settings.set_uint(
            KEY_PREF_JOURNAL_EVENTS_BATCH_SIZE,
            journal_events_batch_size,
        )?;

        let journal_event_max_size = PREFERENCES.journal_event_max_size();
        settings.set_uint(KEY_PREF_JOURNAL_EVENT_MAX_SIZE, journal_event_max_size)?;

        let unit_file_line_numbers = self.unit_file_line_numbers.is_active();
        settings.set_boolean(KEY_PREF_UNIT_FILE_LINE_NUMBERS, unit_file_line_numbers)?;

        let unit_file_style_scheme = PREFERENCES.unit_file_style_scheme();
        settings.set_string(KEY_PREF_UNIT_FILE_STYLE_SCHEME, &unit_file_style_scheme)?;

        let timestamp_style = PREFERENCES.timestamp_style();
        settings.set_string(KEY_PREF_TIMESTAMP_STYLE, timestamp_style.code())?;

        let font_family = PREFERENCES.font_family();
        settings.set_string(KEY_PREF_STYLE_TEXT_FONT_FAMILY, &font_family)?;

        let font_size = PREFERENCES.font_size();
        settings.set_uint(KEY_PREF_STYLE_TEXT_FONT_SIZE, font_size)?;

        Ok(())
    }
}

#[glib::object_subclass]
impl ObjectSubclass for PreferencesDialogImpl {
    const NAME: &'static str = "PreferencesWindow";
    type Type = super::PreferencesDialog;
    type ParentType = adw::PreferencesDialog;

    fn class_init(klass: &mut Self::Class) {
        // The layout manager determines how child widgets are laid out.
        klass.bind_template();
        klass.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for PreferencesDialogImpl {
    fn constructed(&self) {
        self.parent_constructed();
        self.setup_settings();

        let mut levels_string = Vec::new();
        for ts in TimestampStyle::iter() {
            levels_string.push(ts.label());
        }

        let level_str: Vec<&str> = levels_string.iter().map(|x| &**x).collect();
        let string_list = gtk::StringList::new(&level_str);
        self.timestamp_style.set_model(Some(&string_list));

        let cur_style = PREFERENCES.timestamp_style();
        self.timestamp_style.set_selected(cur_style as u32);

        self.timestamp_style
            .connect_selected_item_notify(|combo_box| {
                let selected_item_position = combo_box.selected() as i32;

                let timestamp_style: TimestampStyle = selected_item_position.into();

                combo_box.set_tooltip_text(Some(&timestamp_style.details()));

                PREFERENCES.set_timestamp_style(timestamp_style);
            });
        let settings = self.settings();
        build_preferred_color_scheme(&self.preferred_color_scheme, settings);

        build_pane_orientation_selector(&self.app_orientation, settings);

        debug!("All styles {:?}", style_schemes());

        //unit file no preference style selected
        let mut styles = vec![pgettext("pref file style", "None")];
        let mut vec_style_schemes: Vec<String> = style_schemes().keys().cloned().collect();
        styles.append(&mut vec_style_schemes);
        let vec: Vec<&str> = styles.iter().map(|x| &**x).collect();
        let model = gtk::StringList::new(&vec);
        self.unit_file_style.set_model(Some(&model));

        let style_scheme_id = PREFERENCES.unit_file_style_scheme();

        if !self.select_style_scheme(&vec, &style_scheme_id) {
            warn!("style not found {style_scheme_id:?}");
            self.select_style_scheme(&vec, ADWAITA);
        }

        // Load latest window state
        self.load_preferences_values();

        settings
            .bind(
                KEY_PREF_UNIT_LIST_DISPLAY_COLORS,
                &self.unit_list_colors.get(),
                "active",
            )
            .build();

        settings
            .bind(
                KEY_PREF_UNIT_LIST_DISPLAY_SUMMARY,
                &self.unit_list_summay.get(),
                "active",
            )
            .build();

        settings
            .bind(
                KEY_PREF_UNIT_DESCRIPTION_WRAP,
                &self.unit_description_wrap.get(),
                "active",
            )
            .build();

        #[cfg(not(feature = "flatpak"))]
        {
            use systemd::proxy_switcher::{
                KEY_PREF_PROXY_START_AT_STARTUP, KEY_PREF_PROXY_STOP_AT_CLOSE,
                KEY_PREF_USE_PROXY_CLEAN, KEY_PREF_USE_PROXY_CREATE_DROP_IN,
                KEY_PREF_USE_PROXY_DISABLE_UNIT_FILE, KEY_PREF_USE_PROXY_ENABLE_UNIT_FILE,
                KEY_PREF_USE_PROXY_FREEZE, KEY_PREF_USE_PROXY_RELOAD_DAEMON,
                KEY_PREF_USE_PROXY_REVERT_UNIT_FILE, KEY_PREF_USE_PROXY_SAVE_FILE,
                KEY_PREF_USE_PROXY_THAW, PROXY_SWITCHER,
            };

            use crate::format2;

            settings
                .bind::<adw::SwitchRow>(
                    KEY_PREF_USE_PROXY_CLEAN,
                    self.proxy_clean_switch.as_ref(),
                    "active",
                )
                .build();

            settings
                .bind::<adw::SwitchRow>(
                    KEY_PREF_USE_PROXY_FREEZE,
                    self.proxy_freeze_switch.as_ref(),
                    "active",
                )
                .build();

            settings
                .bind::<adw::SwitchRow>(
                    KEY_PREF_USE_PROXY_THAW,
                    self.proxy_thaw_switch.as_ref(),
                    "active",
                )
                .build();

            settings
                .bind::<adw::SwitchRow>(
                    KEY_PREF_USE_PROXY_ENABLE_UNIT_FILE,
                    self.proxy_enable_unit_file_switch.as_ref(),
                    "active",
                )
                .build();

            settings
                .bind::<adw::SwitchRow>(
                    KEY_PREF_USE_PROXY_DISABLE_UNIT_FILE,
                    self.proxy_disable_unit_file_switch.as_ref(),
                    "active",
                )
                .build();

            settings
                .bind::<adw::SwitchRow>(
                    KEY_PREF_USE_PROXY_RELOAD_DAEMON,
                    self.proxy_reload_daemon_switch.as_ref(),
                    "active",
                )
                .build();

            settings
                .bind::<adw::SwitchRow>(
                    KEY_PREF_USE_PROXY_CREATE_DROP_IN,
                    self.proxy_create_dropin_switch.as_ref(),
                    "active",
                )
                .build();

            settings
                .bind::<adw::SwitchRow>(
                    KEY_PREF_USE_PROXY_SAVE_FILE,
                    self.proxy_save_file_switch.as_ref(),
                    "active",
                )
                .build();

            settings
                .bind::<adw::SwitchRow>(
                    KEY_PREF_USE_PROXY_REVERT_UNIT_FILE,
                    self.proxy_revert_unit_file_switch.as_ref(),
                    "active",
                )
                .build();

            settings
                .bind::<adw::SwitchRow>(
                    KEY_PREF_PROXY_START_AT_STARTUP,
                    self.start_proxy_at_startup_switch.as_ref(),
                    "active",
                )
                .build();

            settings
                .bind::<adw::SwitchRow>(
                    KEY_PREF_PROXY_STOP_AT_CLOSE,
                    self.stop_proxy_at_close_switch.as_ref(),
                    "active",
                )
                .build();

            self.proxy_clean_switch.connect_active_notify(|switch| {
                PROXY_SWITCHER.set_clean(switch.is_active());
            });

            self.proxy_freeze_switch.connect_active_notify(|switch| {
                PROXY_SWITCHER.set_freeze(switch.is_active());
            });

            self.proxy_thaw_switch.connect_active_notify(|switch| {
                PROXY_SWITCHER.set_thaw(switch.is_active());
            });

            self.proxy_enable_unit_file_switch
                .connect_active_notify(|switch| {
                    PROXY_SWITCHER.set_enable_unit_file(switch.is_active());
                });

            self.proxy_disable_unit_file_switch
                .connect_active_notify(|switch| {
                    PROXY_SWITCHER.set_disable_unit_file(switch.is_active());
                });

            self.proxy_reload_daemon_switch
                .connect_active_notify(|switch| {
                    PROXY_SWITCHER.set_reload(switch.is_active());
                });

            self.proxy_create_dropin_switch
                .connect_active_notify(|switch| {
                    PROXY_SWITCHER.set_create_dropin(switch.is_active());
                });

            self.proxy_revert_unit_file_switch
                .connect_active_notify(|switch| {
                    PROXY_SWITCHER.set_revert_unit_file(switch.is_active());
                });

            self.proxy_save_file_switch.connect_active_notify(|switch| {
                PROXY_SWITCHER.set_save_file(switch.is_active());
            });

            self.start_proxy_at_startup_switch
                .connect_active_notify(|switch| {
                    PROXY_SWITCHER.set_start_at_startup(switch.is_active());
                });

            self.stop_proxy_at_close_switch
                .connect_active_notify(|switch| {
                    PROXY_SWITCHER.set_stop_at_close(switch.is_active());
                });

            let proxy_clean_switch = self.proxy_clean_switch.clone();
            let proxy_freeze_switch = self.proxy_freeze_switch.clone();
            let proxy_thaw_switch = self.proxy_thaw_switch.clone();
            let proxy_enable_unit_file = self.proxy_enable_unit_file_switch.clone();
            let proxy_disable_unit_file = self.proxy_disable_unit_file_switch.clone();
            let proxy_reload_switch = self.proxy_reload_daemon_switch.clone();
            let proxy_create_dropin_switch = self.proxy_create_dropin_switch.clone();
            let proxy_save_file = self.proxy_save_file_switch.clone();
            let proxy_revert_unit_file_switch = self.proxy_revert_unit_file_switch.clone();

            let group_of_switches = [
                proxy_clean_switch,
                proxy_freeze_switch,
                proxy_thaw_switch,
                proxy_enable_unit_file,
                proxy_disable_unit_file,
                proxy_reload_switch,
                proxy_create_dropin_switch,
                proxy_save_file,
                proxy_revert_unit_file_switch,
            ];

            let sum: usize = group_of_switches
                .iter()
                .map(|s| if s.is_active() { 1 } else { 0 })
                .sum();
            let ratio = sum as f32 / group_of_switches.len() as f32;
            let all_active = ratio > 0.5;
            self.proxy_all_switch.set_active(all_active);

            self.proxy_all_switch
                .connect_active_notify(move |all_switch| {
                    let all_active = all_switch.is_active();
                    for switch in group_of_switches.iter() {
                        switch.set_active(all_active);
                    }
                });

            let service = "sysd-manager-proxy.service";
            let service_style = format!("<b>{service}</b>");
            let description = pgettext(
                "preference",
                format2!(
                    "List Dbus Messages and Actions that are preformed or not by the {} <a href=\"unit://{}\" >Proxy</a> for privilege elevation purposes.",
                    service_style,
                    service
                ),
            );

            self.pref_proxy_page.set_description(&description);
            if let Some(label) = find_description_label(self.pref_proxy_page.as_ref()) {
                label_link_handler(&label, &self.obj());
            }
        }

        #[cfg(feature = "flatpak")]
        {
            //Note the switch are set to active false by default

            self.proxy_all_switch.set_sensitive(false);
            self.proxy_clean_switch.set_sensitive(false);
            self.proxy_freeze_switch.set_sensitive(false);
            self.proxy_thaw_switch.set_sensitive(false);
            self.proxy_enable_unit_file_switch.set_sensitive(false);
            self.proxy_disable_unit_file_switch.set_sensitive(false);
            self.proxy_create_dropin_switch.set_sensitive(false);
            self.proxy_reload_daemon_switch.set_sensitive(false);
            self.proxy_save_file_switch.set_sensitive(false);
            self.proxy_revert_unit_file_switch.set_sensitive(false);
            self.start_proxy_at_startup_switch.set_sensitive(false);
            self.stop_proxy_at_close_switch.set_sensitive(false);

            self.proxy_banner.set_revealed(true);
        }
    }
}

#[cfg(not(feature = "flatpak"))]
fn find_description_label(node: &gtk::Widget) -> Option<gtk::Label> {
    let id = node.buildable_id();
    if let Some(id) = id
        && id.as_str() == "description"
        && let Some(label) = node.downcast_ref::<gtk::Label>()
    {
        return Some(label.clone());
    }

    if let Some(child) = node.first_child()
        && let Some(label) = find_description_label(&child)
    {
        return Some(label);
    }

    let mut sibling = node.next_sibling();
    while let Some(sibling1) = sibling {
        let label = find_description_label(&sibling1);
        if label.is_some() {
            return label;
        }
        sibling = sibling1.next_sibling();
    }
    None
}

#[cfg(not(feature = "flatpak"))]
fn label_link_handler(label: &gtk::Label, pref_dialog: &super::PreferencesDialog) {
    let pref_dialog = pref_dialog.clone();
    label.connect_activate_link(move |_label, uri| {
        use base::enums::UnitDBusLevel;

        info!("link uri: {uri}");

        if !uri.starts_with("unit://") {
            return glib::Propagation::Proceed;
        }

        let Some(unit_name) = uri.strip_prefix("unit://") else {
            return glib::Propagation::Proceed;
        };

        let (unit_name, level) = match unit_name.split_once("?") {
            Some((prefix, suffix)) => (prefix, UnitDBusLevel::from_short(suffix)),
            None => (unit_name, UnitDBusLevel::System),
        };

        info!("open unit {:?} at level {}", unit_name, level.short());

        let unit = systemd::fetch_unit(level, unit_name)
            .inspect_err(|e| warn!("Cli unit: {e:?}"))
            .ok();

        if let Some(app_window) = pref_dialog.imp().app_window.borrow().as_ref() {
            app_window.set_unit(unit.as_ref());
        } else {
            warn!("app_window missing");
        }
        glib::Propagation::Stop
    });
}

impl WidgetImpl for PreferencesDialogImpl {}
//impl WindowImpl for PreferencesDialogImpl {}

impl AdwDialogImpl for PreferencesDialogImpl {
    fn closed(&self) {
        log::info!("Close preferences window");

        PREFERENCES.set_app_first_connection(false);

        if let Err(error) = self.save_preference_settings() {
            warn!("Save setting  error {error:?}")
        }

        let binding = self.app_window.borrow();
        if let Some(app_window) = binding.as_ref() {
            app_window.refresh_panels()
        };
    }
}

impl adw::subclass::prelude::PreferencesDialogImpl for PreferencesDialogImpl {}
