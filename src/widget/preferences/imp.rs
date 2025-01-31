use gio::Settings;

use adw::{prelude::*, subclass::prelude::*, EnumListItem};
use gtk::{
    gio,
    glib::{self, BoolError},
    pango::{self, FontFace},
};
use log::{debug, error, info, warn};
use std::cell::{OnceCell, RefCell};

use crate::{
    systemd_gui::new_settings, utils::font_management::FONT_CONTEXT, widget::app_window::AppWindow,
};
use crate::{utils::th::TimestampStyle, widget::InterPanelAction};

use super::data::{
    KEY_PREF_APP_FIRST_CONNECTION, KEY_PREF_JOURNAL_COLORS, KEY_PREF_JOURNAL_EVENT_MAX_SIZE,
    KEY_PREF_JOURNAL_MAX_EVENTS, KEY_PREF_STYLE_TEXT_FONT_FAMILY, KEY_PREF_STYLE_TEXT_FONT_SIZE,
    KEY_PREF_TIMESTAMP_STYLE, KEY_PREF_UNIT_FILE_HIGHLIGHTING, PREFERENCES,
};

#[derive(Debug, Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/preferences.ui")]
pub struct PreferencesDialogImpl {
    pub settings: OnceCell<Settings>,

    #[template_child]
    pub journal_colors: TemplateChild<gtk::Switch>,

    #[template_child]
    pub unit_file_highlight: TemplateChild<gtk::Switch>,

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

    app_window: RefCell<Option<AppWindow>>,
}

#[gtk::template_callbacks]
impl PreferencesDialogImpl {
    #[template_callback]
    fn journal_switch_state_set(&self, state: bool) -> bool {
        info!("journal_colors_switch {}", state);

        self.journal_colors.set_state(state);
        PREFERENCES.set_journal_colors(state);

        true
    }

    #[template_callback]
    fn journal_max_events_changed(&self, spin: adw::SpinRow) {
        let value32_parse = Self::get_spin_row_value("journal_events_changed", spin);

        PREFERENCES.set_journal_events(value32_parse);
    }

    #[template_callback]
    fn journal_event_max_size_changed(&self, spin: adw::SpinRow) {
        let value32_parse = Self::get_spin_row_value("journal_event_max_size_changed", spin);

        PREFERENCES.set_journal_event_max_size(value32_parse);
    }

    #[template_callback]
    fn unit_file_highlighting_state_set(&self, state: bool) -> bool {
        info!("unit_file_highlighting_switch {}", state);

        self.unit_file_highlight.set_state(state);
        PREFERENCES.set_unit_file_highlighting(state);

        true
    }

    #[template_callback]
    fn select_font_clicked(&self) {
        let filter = gtk::CustomFilter::new(move |object| {
            let Some(font_face) = object.downcast_ref::<FontFace>() else {
                error!("some wrong downcast_ref {:?}", object);
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

        /*         if let Some(family) = font_description.family() {
            for sub_family in family.split(",") {
                info!("set sub {sub_family}");
                font_description.set_family(sub_family);
                break;
            }
        } */

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
                        let action = InterPanelAction::SetFont(Some(&font_description));

                        window.set_inter_action(&action);
                    }

                    select_font_row.set_subtitle(&font_name);
                }
                Err(e) => warn!("Select font error: {:?}", e),
            },
        );
    }

    #[template_callback]
    fn select_font_default(&self) {
        PREFERENCES.set_font_default();

        let window = self.app_window.borrow();
        if let Some(window) = window.as_ref() {
            let action = crate::widget::InterPanelAction::SetFont(None);
            window.set_inter_action(&action);
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
}

impl PreferencesDialogImpl {
    pub(super) fn set_app_window(&self, app_window: Option<&AppWindow>) {
        if let Some(app_window) = app_window {
            self.app_window.replace(Some(app_window.clone()));
        }
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
        let journal_colors = PREFERENCES.journal_colors();
        let unit_file_colors = PREFERENCES.unit_file_colors();
        let is_app_first_connection = PREFERENCES.is_app_first_connection();

        self.journal_colors.set_state(journal_colors);
        self.journal_colors.set_active(journal_colors);

        let journal_max_events = PREFERENCES.journal_max_events();
        self.journal_max_events.set_value(journal_max_events as f64);

        let journal_event_max_size = PREFERENCES.journal_event_max_size();
        self.journal_event_max_size
            .set_value(journal_event_max_size as f64);

        self.unit_file_highlight.set_state(unit_file_colors);
        self.unit_file_highlight.set_active(unit_file_colors);

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

        info!("{var_name} to {:?} , text {:?}", value, text);

        match text.parse::<u32>() {
            Ok(a) => a,
            Err(_e) => {
                info!("Parse error {:?} to u32 do falback to f64", text);
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

        let journal_events = PREFERENCES.journal_max_events();
        settings.set_uint(KEY_PREF_JOURNAL_MAX_EVENTS, journal_events)?;

        let journal_event_max_size = PREFERENCES.journal_event_max_size();
        settings.set_uint(KEY_PREF_JOURNAL_EVENT_MAX_SIZE, journal_event_max_size)?;

        let unit_file_colors = PREFERENCES.unit_file_colors();
        settings.set_boolean(KEY_PREF_UNIT_FILE_HIGHLIGHTING, unit_file_colors)?;

        let timestamp_style = PREFERENCES.timestamp_style();
        settings.set_string(KEY_PREF_TIMESTAMP_STYLE, &timestamp_style.to_string())?;

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

        let model = adw::EnumListModel::new(TimestampStyle::static_type());

        self.timestamp_style.set_model(Some(&model));

        let expression = gtk::PropertyExpression::new(
            adw::EnumListItem::static_type(),
            None::<gtk::Expression>,
            "name",
        );

        self.timestamp_style.set_expression(Some(expression));

        let cur_style = PREFERENCES.timestamp_style();
        self.timestamp_style.set_selected(cur_style as u32);

        self.timestamp_style
            .connect_selected_item_notify(|combo_box| {
                let selected_item = combo_box.selected_item();

                let Some(timestamp_style) = selected_item else {
                    return;
                };

                let timestamp_style = timestamp_style
                    .downcast::<EnumListItem>()
                    .expect("Needs to be TimestampStyle");

                combo_box.set_tooltip_text(Some(&timestamp_style.nick()));

                let tss = TimestampStyle::from(timestamp_style.value());
                PREFERENCES.set_timestamp_style(tss);
            });

        // Load latest window state
        self.setup_settings();
        self.load_preferences_values();
    }
}
impl WidgetImpl for PreferencesDialogImpl {}
impl WindowImpl for PreferencesDialogImpl {}

impl AdwDialogImpl for PreferencesDialogImpl {
    fn closed(&self) {
        log::info!("Close preferences window");

        PREFERENCES.set_app_first_connection(false);

        if let Err(error) = self.save_preference_settings() {
            warn!("Save setting  error {:?}", error)
        }

        let binding = self.app_window.borrow();
        if let Some(app_window) = binding.as_ref() {
            app_window.refresh_panels()
        };
    }
}

impl adw::subclass::prelude::PreferencesDialogImpl for PreferencesDialogImpl {}
