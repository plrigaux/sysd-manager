use gettextrs::pgettext;
use gio::Settings;
use systemd::time_handling::TimestampStyle;

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

use super::data::{
    KEY_PREF_APP_FIRST_CONNECTION, KEY_PREF_JOURNAL_COLORS, KEY_PREF_JOURNAL_EVENT_MAX_SIZE,
    KEY_PREF_JOURNAL_EVENTS_BATCH_SIZE, KEY_PREF_STYLE_TEXT_FONT_FAMILY,
    KEY_PREF_STYLE_TEXT_FONT_SIZE, KEY_PREF_TIMESTAMP_STYLE, KEY_PREF_UNIT_FILE_LINE_NUMBERS,
    KEY_PREF_UNIT_FILE_STYLE_SCHEME, PREFERENCES,
};

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

        self.preference_banner.set_use_markup(true);
        self.preference_banner.set_title(
            "It's your first connection
You can set the application's Dbus level to <u>System</u> if you want to see all Systemd units.",
        );

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
    }
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
