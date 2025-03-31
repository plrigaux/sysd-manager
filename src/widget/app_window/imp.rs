use std::{
    borrow::Cow,
    cell::{Cell, OnceCell},
    sync::OnceLock,
};

use adw::subclass::prelude::*;
use gio::{
    glib::VariantTy,
    prelude::{ActionMapExtManual, SettingsExt},
};
use gtk::{
    gio, glib,
    prelude::{GtkApplicationExt, GtkWindowExt, OrientableExt, ToggleButtonExt, WidgetExt},
};
use log::{debug, error, info, warn};
use regex::Regex;
use sourceview5::prelude::StaticType;

use crate::{
    systemd::data::UnitInfo,
    systemd_gui::new_settings,
    utils::palette::{blue, green, red},
    widget::{
        InterPanelMessage,
        preferences::data::{DbusLevel, KEY_PREF_ORIENTATION_MODE, OrientationMode, PREFERENCES},
        unit_control_panel::UnitControlPanel,
        unit_list::UnitListPanel,
    },
};

const WINDOW_WIDTH: &str = "window-width";
const WINDOW_HEIGHT: &str = "window-height";
const PANED_SEPARATOR_POSITION: &str = "paned-separator-position";
const WINDOW_PANES_ORIENTATION: &str = "window-panes-orientation";
const IS_MAXIMIZED: &str = "is-maximized";
const HORIZONTAL: &str = "horizontal";

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/app_window.ui")]
pub struct AppWindowImpl {
    settings: OnceCell<gio::Settings>,

    #[template_child]
    header_bar: TemplateChild<adw::HeaderBar>,

    #[template_child]
    pub(super) toast_overlay: TemplateChild<adw::ToastOverlay>,

    #[template_child]
    unit_list_panel: TemplateChild<UnitListPanel>,

    #[template_child]
    unit_control_panel: TemplateChild<UnitControlPanel>,

    #[template_child]
    paned: TemplateChild<gtk::Paned>,

    #[template_child]
    search_toggle_button: TemplateChild<gtk::ToggleButton>,

    #[template_child]
    refresh_unit_list_button: TemplateChild<gtk::Button>,

    #[template_child]
    system_session_dropdown: TemplateChild<gtk::DropDown>,

    #[template_child]
    app_title: TemplateChild<adw::WindowTitle>,

    #[template_child]
    breakpoint: TemplateChild<adw::Breakpoint>,

    is_dark: Cell<bool>,

    orientation_mode: Cell<OrientationMode>,
}

#[glib::object_subclass]
impl ObjectSubclass for AppWindowImpl {
    const NAME: &'static str = "SysdMainAppWindow";
    type Type = super::AppWindow;
    type ParentType = adw::ApplicationWindow;

    fn class_init(klass: &mut Self::Class) {
        // The layout manager determines how child widgets are laid out.
        klass.bind_template();
        klass.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for AppWindowImpl {
    fn constructed(&self) {
        self.parent_constructed();

        // Load latest window state
        let settings = self.setup_settings();
        {
            let app_window = self.obj().clone();
            let paned = self.paned.clone();
            settings.connect_changed(Some(KEY_PREF_ORIENTATION_MODE), move |settings, _key| {
                let value = settings.string(KEY_PREF_ORIENTATION_MODE);
                let orientation_mode = OrientationMode::from_key(&value);
                app_window.imp().orientation_mode.set(orientation_mode);
                let window_panes_orientation = paned.orientation();
                app_window.imp().set_orientation(window_panes_orientation);
            });
        }
        self.load_window_size();
        let app_window = self.obj();
        self.unit_list_panel
            .register_selection_change(&app_window, &self.refresh_unit_list_button);

        self.unit_control_panel.set_app_window(&app_window);

        self.setup_dropdown();

        let condition_1 = adw::BreakpointCondition::new_ratio(
            adw::BreakpointConditionRatioType::MinAspectRatio,
            4,
            3,
        );

        let condition_2 = adw::BreakpointCondition::new_ratio(
            adw::BreakpointConditionRatioType::MaxAspectRatio,
            16,
            9,
        );

        let condition = adw::BreakpointCondition::new_and(condition_1, condition_2);

        self.breakpoint.set_condition(Some(&condition));

        {
            let paned = self.paned.clone();
            let app_window = self.obj().clone();
            self.breakpoint.connect_unapply(move |_breakpoint| {
                debug!("connect_unapply");

                let window_panes_orientation = paned.orientation();
                app_window.imp().set_orientation(window_panes_orientation);
            });
        }
    }
}

#[gtk::template_callbacks]
impl AppWindowImpl {
    fn setup_settings(&self) -> &gio::Settings {
        let settings: gio::Settings = new_settings();

        self.settings
            .set(settings)
            .expect("`settings` should not be set before calling `setup_settings`.");

        self.settings()
    }

    fn settings(&self) -> &gio::Settings {
        self.settings
            .get()
            .expect("`settings` should be set in `setup_settings`.")
    }

    fn setup_dropdown(&self) {
        let expression = gtk::PropertyExpression::new(
            adw::EnumListItem::static_type(),
            None::<gtk::Expression>,
            "nick",
        );

        self.system_session_dropdown
            .set_expression(Some(expression));

        let model = adw::EnumListModel::new(DbusLevel::static_type());

        self.system_session_dropdown.set_model(Some(&model));

        {
            let settings = self.settings().clone();
            let unit_list_panel = self.unit_list_panel.clone();

            let level = PREFERENCES.dbus_level();
            let level_num = level as u32;
            self.system_session_dropdown.set_selected(level_num);
            let selected = self.system_session_dropdown.selected();
            info!(
                "Set system_session_dropdown {:?} {} selected {}",
                level, level_num, selected
            );

            self.system_session_dropdown
                .connect_selected_item_notify(move |dropdown| {
                    let idx = dropdown.selected();
                    let level: DbusLevel = idx.into();

                    debug!(
                        "System Session Values Selected idx {:?} level {:?}",
                        idx, level
                    );

                    PREFERENCES.set_dbus_level(level);
                    PREFERENCES.save_dbus_level(&settings);

                    unit_list_panel.fill_store();
                });
        }
    }

    pub fn save_window_context(&self) -> Result<(), glib::BoolError> {
        // Get the size of the window

        let obj = self.obj();
        let (width, height) = obj.default_size();

        // Set the window state in `settings`
        let settings = self.settings();

        settings.set_int(WINDOW_WIDTH, width)?;
        settings.set_int(WINDOW_HEIGHT, height)?;
        settings.set_boolean(IS_MAXIMIZED, obj.is_maximized())?;

        let separator_position = self.paned.position();
        settings.set_int(PANED_SEPARATOR_POSITION, separator_position)?;

        let window_panes_orientation = if self.paned.orientation() == gtk::Orientation::Horizontal {
            HORIZONTAL
        } else {
            "vertical"
        };

        settings.set_string(WINDOW_PANES_ORIENTATION, window_panes_orientation)?;

        Ok(())
    }

    #[template_callback]
    fn button_search_toggled(&self, toggle_button: &gtk::ToggleButton) {
        self.unit_list_panel
            .button_search_toggled(toggle_button.is_active());
    }

    #[template_callback]
    fn refresh_button_clicked(&self, _button: &gtk::Button) {
        info!("refresh false");
        //button.set_sensitive(false);
        self.unit_list_panel.fill_store();
        //button.set_sensitive(true);
        info!("refresh true");
    }
}

impl AppWindowImpl {
    fn load_window_size(&self) {
        // Get the window state from `settings`
        let settings = self.settings();

        let mut width = settings.int(WINDOW_WIDTH);
        let mut height = settings.int(WINDOW_HEIGHT);
        let is_maximized = settings.boolean(IS_MAXIMIZED);
        let mut separator_position = settings.int(PANED_SEPARATOR_POSITION);
        let window_panes_orientation = settings.string(WINDOW_PANES_ORIENTATION);
        let pref_orientation_mode = settings.string(KEY_PREF_ORIENTATION_MODE);

        info!(
            "Window settings: width {width}, height {height}, is-maximized {is_maximized}, panes orientation {window_panes_orientation}"
        );

        let obj = self.obj();
        let (def_width, def_height) = obj.default_size();

        if width < 0 {
            width = def_width;
            if width < 0 {
                width = 1280;
            }
        }

        if height < 0 {
            height = def_height;
            if height < 0 {
                height = 720;
            }
        }

        // Set the size of the window
        obj.set_default_size(width, height);

        // If the window was maximized when it was closed, maximize it again
        if is_maximized {
            obj.maximize();
        }

        if separator_position < 0 {
            separator_position = width / 2;
        }

        self.paned.set_position(separator_position);

        let orientation_mode = OrientationMode::from_key(&pref_orientation_mode);
        self.orientation_mode.set(orientation_mode);
        let window_panes_orientation = if window_panes_orientation == HORIZONTAL {
            gtk::Orientation::Horizontal
        } else {
            gtk::Orientation::Vertical
        };
        self.set_orientation(window_panes_orientation);
    }

    #[allow(clippy::if_same_then_else)]
    fn set_orientation(&self, window_panes_orientation: gtk::Orientation) {
        let orientation_mode = self.orientation_mode.get();

        let orientation = match orientation_mode {
            OrientationMode::Automatic => {
                let (width, height) = self.obj().default_size();

                let ratio = height as f32 / width as f32;

                //Enforce the rules
                if ratio >= 3.0 / 4.0 {
                    gtk::Orientation::Vertical
                } else if ratio <= 9.0 / 16.0 {
                    gtk::Orientation::Horizontal
                } else {
                    window_panes_orientation
                }
            }
            OrientationMode::ForceHorizontal => gtk::Orientation::Horizontal,
            OrientationMode::ForceVertical => gtk::Orientation::Vertical,
        };

        self.paned.set_orientation(orientation);
    }

    pub(super) fn selection_change(&self, unit: Option<&UnitInfo>) {
        if let Some(unit) = unit {
            self.app_title.set_subtitle(&unit.primary());
        } else {
            self.app_title.set_subtitle("");
        }

        self.unit_control_panel.selection_change(unit);
    }

    pub(super) fn set_unit(&self, unit: Option<&UnitInfo>) {
        self.selection_change(unit);
        self.unit_list_panel.set_unit(unit);
    }

    pub(super) fn refresh_panels(&self) {
        self.unit_control_panel.refresh_panels()
    }

    pub fn set_inter_message(&self, action: &InterPanelMessage) {
        self.unit_control_panel.set_inter_message(action);
        self.unit_list_panel.set_inter_message(action);

        if let InterPanelMessage::IsDark(is_dark) = *action {
            self.is_dark.set(is_dark);
        }
    }

    pub(super) fn build_action(&self, application: &adw::Application) {
        let search_toggle_button = self.search_toggle_button.clone();
        let unit_list_panel = self.unit_list_panel.clone();
        let search_units: gio::ActionEntry<adw::Application> =
            gio::ActionEntry::builder("search_units")
                .activate(move |_application: &adw::Application, _, _| {
                    if !search_toggle_button.is_active() {
                        search_toggle_button.activate();
                    } else {
                        unit_list_panel.button_search_toggled(true);
                    }
                })
                .build();

        let open_info: gio::ActionEntry<adw::Application> = {
            let unit_control_panel = self.unit_control_panel.clone();
            gio::ActionEntry::builder("open_info")
                .activate(move |_application: &adw::Application, _, _| {
                    unit_control_panel.display_info_page();
                })
                .build()
        };

        let open_dependencies: gio::ActionEntry<adw::Application> = {
            let unit_control_panel = self.unit_control_panel.clone();
            gio::ActionEntry::builder("open_dependencies")
                .activate(move |_application: &adw::Application, _, _| {
                    unit_control_panel.display_dependencies_page();
                })
                .build()
        };

        let open_journal: gio::ActionEntry<adw::Application> = {
            let unit_control_panel = self.unit_control_panel.clone();
            gio::ActionEntry::builder("open_journal")
                .activate(move |_application: &adw::Application, _, _| {
                    unit_control_panel.display_journal_page();
                })
                .build()
        };

        let open_file: gio::ActionEntry<adw::Application> = {
            let unit_control_panel = self.unit_control_panel.clone();
            gio::ActionEntry::builder("open_file")
                .activate(move |_application: &adw::Application, _, _| {
                    unit_control_panel.display_definition_file_page();
                })
                .build()
        };

        let default_state = glib::variant::ToVariant::to_variant(&"auto");
        let orientation_mode: gio::ActionEntry<adw::Application> = gio::ActionEntry::builder(
            KEY_PREF_ORIENTATION_MODE,
        )
        .activate(move |_win: &adw::Application, action, variant| {
            warn!("action {:?} variant {:?}", action, variant);

            println!("asdfasdfasdfasd asdfasdfdfddddddddddddddddddddddddddddddddddddddddddddddddd");
        })
        .parameter_type(Some(VariantTy::STRING))
        .state(default_state)
        .build();

        application.add_action_entries([
            search_units,
            open_info,
            open_dependencies,
            open_journal,
            open_file,
            orientation_mode,
        ]);

        application.set_accels_for_action("app.search_units", &["<Ctrl>f"]);
        application.set_accels_for_action("app.open_info", &["<Ctrl>i"]);
        application.set_accels_for_action("app.open_dependencies", &["<Ctrl>d"]);
        application.set_accels_for_action("app.open_journal", &["<Ctrl>j"]);
        application.set_accels_for_action("app.open_file", &["<Ctrl>u"]);
    }

    pub fn overlay(&self) -> &adw::ToastOverlay {
        &self.toast_overlay
    }

    fn blue(&self) -> &str {
        blue(self.is_dark.get()).get_color()
    }

    fn green(&self) -> &str {
        green(self.is_dark.get()).get_color()
    }

    fn red(&self) -> &str {
        red(self.is_dark.get()).get_color()
    }

    pub(super) fn add_toast_message(&self, message: &str, use_markup: bool) {
        let msg = if use_markup {
            let out = self.replace_tags(message);
            Cow::from(out)
        } else {
            Cow::from(message)
        };

        let toast = adw::Toast::builder()
            .title(msg)
            .use_markup(use_markup)
            .build();
        self.toast_overlay.add_toast(toast)
    }

    fn replace_tags(&self, message: &str) -> String {
        debug!("{}", message);
        let mut out = String::with_capacity(message.len() * 2);
        let re = toast_regex();

        let mut i: usize = 0;
        for capture in re.captures_iter(message) {
            let m = capture.get(0).unwrap();
            out.push_str(&message[i..m.start()]);

            let tag = &capture[1];
            match tag {
                "unit" => {
                    out.push_str("<span fgcolor='");
                    out.push_str(self.blue());
                    out.push_str("' font_family='monospace' size='larger'>");
                    out.push_str(&capture[2]);
                    out.push_str("</span>");
                }

                "red" => {
                    out.push_str("<span fgcolor='");
                    out.push_str(self.red());
                    out.push_str("'>");
                    out.push_str(&capture[2]);
                    out.push_str("</span>");
                }

                "green" => {
                    out.push_str("<span fgcolor='");
                    out.push_str(self.green());
                    out.push_str("'>");
                    out.push_str(&capture[2]);
                    out.push_str("</span>");
                }
                _ => {
                    out.push_str(&capture[0]);
                }
            }
            i = m.end();
        }
        out.push_str(&message[i..message.len()]);
        debug!("{}", out);
        out
    }
}

impl WidgetImpl for AppWindowImpl {}
impl WindowImpl for AppWindowImpl {
    // Save window state right before the window will be closed
    fn close_request(&self) -> glib::Propagation {
        // Save window size
        debug!("Close window");
        if let Err(_err) = self.save_window_context() {
            error!("Failed to save window state");
        }

        self.parent_close_request();
        // Allow to invoke other event handlers
        glib::Propagation::Proceed
    }
}

impl AdwApplicationWindowImpl for AppWindowImpl {}
impl ApplicationWindowImpl for AppWindowImpl {}

pub fn toast_regex() -> &'static Regex {
    static TOAST_REGEX: OnceLock<Regex> = OnceLock::new();
    TOAST_REGEX
        .get_or_init(|| Regex::new(r"<(\w+).*?>(.*?)</(\w+?)>").expect("Rexgex compile error :"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reg_ex1() {
        let r = toast_regex();

        let test_str = "asdf <unit>unit.serv</unit> ok";

        for capt in r.captures_iter(test_str) {
            println!("capture: {:#?}", capt)
        }
    }

    #[test]
    fn test_reg_ex2() {
        let r = toast_regex();

        let test_str = [
            "asdf <unit arg=\"test\">unit.serv</unit> ok",
            "Clean unit <unit>tiny_daemon.service</unit> with parameters <b>cache</b> and <b>configuration</b> failed",
        ];

        for test in test_str {
            for capt in r.captures_iter(test) {
                println!("capture: {:#?}", capt)
            }
        }
    }
}
