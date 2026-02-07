use std::{
    borrow::Cow,
    cell::{Cell, OnceCell, Ref, RefCell, RefMut},
    rc::Rc,
    sync::OnceLock,
};

use crate::{
    consts::{
        ACTION_LIST_BOOT, ACTION_PROPERTIES_SELECTOR, ACTION_PROPERTIES_SELECTOR_GENERAL,
        ACTION_UNIT_PROPERTIES_DISPLAY, APP_ACTION_LIST_BOOT,
        APP_ACTION_PROPERTIES_SELECTOR_GENERAL, APP_ACTION_UNIT_PROPERTIES_DISPLAY,
        NS_ACTION_REFRESH_UNIT_LIST,
    },
    systemd::data::UnitInfo,
    systemd_gui::new_settings,
    utils::palette::{blue, green, red},
    widget::{
        InterPanelMessage,
        info_window::InfoWindow,
        journal::list_boots::ListBootsWindow,
        preferences::data::{DbusLevel, KEY_PREF_ORIENTATION_MODE, OrientationMode, PREFERENCES},
        signals_dialog::SignalsWindow,
        unit_control_panel::UnitControlPanel,
        unit_list::{UnitListPanel, UnitListView},
        unit_properties_selector::UnitPropertiesSelectorDialog,
    },
};
use adw::subclass::prelude::*;
use glib::{self, VariantTy, types::StaticType};
use gtk::{
    gio::{
        self,
        prelude::{ActionMapExtManual, SettingsExt},
    },
    prelude::{
        GtkApplicationExt, GtkWindowExt, OrientableExt, ToVariant, ToggleButtonExt, WidgetExt,
    },
};
use log::{debug, error, info, warn};
use regex::Regex;

use systemd::journal_data::Boot;

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
    unit_list_view_menubutton: TemplateChild<gtk::MenuButton>,

    #[template_child]
    app_title: TemplateChild<adw::WindowTitle>,

    #[template_child]
    breakpoint: TemplateChild<adw::Breakpoint>,

    orientation_mode: Cell<OrientationMode>,

    list_boots: RefCell<Option<Vec<Rc<Boot>>>>,

    pub(super) selected_unit: RefCell<Option<UnitInfo>>,

    pub signals_window: RefCell<Option<SignalsWindow>>,
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

        let menu_views = UnitListView::menu_items();

        self.unit_list_view_menubutton
            .set_menu_model(Some(&menu_views));
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
        let model = adw::EnumListModel::new(DbusLevel::static_type());

        let empty: [gtk::Expression; 0] = [];
        let expression = gtk::ClosureExpression::new::<String>(
            empty,
            glib::closure!(|s: adw::EnumListItem| {
                let dbus: DbusLevel = s.value().into();
                dbus.label()
            }),
        );

        self.system_session_dropdown
            .set_expression(Some(expression));

        self.system_session_dropdown.set_model(Some(&model));

        {
            let settings = self.settings().clone();

            let level = PREFERENCES.dbus_level();
            self.system_session_dropdown.set_selected(level as u32);

            self.system_session_dropdown
                .connect_selected_item_notify(move |dropdown| {
                    let idx = dropdown.selected();
                    let level: DbusLevel = idx.into();

                    debug!("System Session Values Selected idx {idx:?} level {level:?}");

                    PREFERENCES.set_and_save_dbus_level(level, &settings);

                    if let Err(err) = dropdown.activate_action(NS_ACTION_REFRESH_UNIT_LIST, None) {
                        warn!("call action {NS_ACTION_REFRESH_UNIT_LIST} error: {err}");
                    }
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

        self.selected_unit.replace(unit.cloned());

        self.unit_control_panel.selection_change(unit);
    }

    pub(super) fn set_unit(&self, unit: Option<&UnitInfo>) -> Option<UnitInfo> {
        self.selection_change(unit);
        self.unit_list_panel.set_unit(unit)
    }

    pub(super) fn refresh_panels(&self) {
        self.unit_control_panel.refresh_panels()
    }

    pub fn set_inter_message(&self, action: &InterPanelMessage) {
        self.unit_control_panel.set_inter_message(action);
        self.unit_list_panel.set_inter_message(action);
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
        let orientation_mode: gio::ActionEntry<adw::Application> =
            gio::ActionEntry::builder(KEY_PREF_ORIENTATION_MODE)
                .activate(move |_win: &adw::Application, action, variant| {
                    warn!("action {action:?} variant {variant:?}");
                })
                .parameter_type(Some(VariantTy::STRING))
                .state(default_state)
                .build();

        let list_boots = {
            let app_window = self.obj().clone();

            gio::ActionEntry::builder(ACTION_LIST_BOOT)
                .activate(move |_, _action, _variant| {
                    let list_boots_window = ListBootsWindow::new(&app_window);
                    //    list_boots_window.set_transient_for(Some(&list_boots_window));
                    list_boots_window.set_modal(false);

                    list_boots_window.present();
                })
                .build()
        };

        let properties_selector = {
            let app_window = self.obj().clone();
            let unit_list_panel = self.unit_list_panel.clone();
            gio::ActionEntry::builder(ACTION_PROPERTIES_SELECTOR)
                .activate(move |_, _action, variant| {
                    let column_id = variant.map(|v| v.get::<String>().unwrap());
                    let dialog = UnitPropertiesSelectorDialog::new(&unit_list_panel, column_id);
                    dialog.set_transient_for(Some(&app_window));
                    //dialog.set_modal(true);
                    dialog.present();
                })
                .parameter_type(Some(VariantTy::STRING))
                .build()
        };

        let properties_selector_general = {
            let app_window = self.obj().clone();
            let unit_list_panel = self.unit_list_panel.clone();
            gio::ActionEntry::builder(ACTION_PROPERTIES_SELECTOR_GENERAL)
                .activate(move |_, _action, variant| {
                    let column_id = variant.map(|v| v.get::<String>().unwrap());
                    let dialog = UnitPropertiesSelectorDialog::new(&unit_list_panel, column_id);
                    dialog.set_transient_for(Some(&app_window));
                    //dialog.set_modal(true);
                    dialog.present();
                })
                .build()
        };

        let print_debug = {
            let unit_list_panel = self.unit_list_panel.clone();
            gio::ActionEntry::builder("debug")
                .activate(move |_, _action, _variant| {
                    unit_list_panel.print_scroll_adj_logs();
                })
                .build()
        };

        let display_unit_properties = {
            let app_window = self.obj().clone();

            gio::ActionEntry::builder(ACTION_UNIT_PROPERTIES_DISPLAY)
                .activate(move |_, _action, _variant| {
                    let Some(selected_unit) = app_window.selected_unit() else {
                        warn!("Can't display unit properties, No unit selected");
                        return;
                    };

                    info!(
                        "Displaying unit properties for {:?}",
                        selected_unit.primary()
                    );

                    let unit_properties = InfoWindow::new(Some(&selected_unit));

                    unit_properties.set_transient_for(Some(&app_window));
                    //dialog.set_modal(true);
                    unit_properties.present();
                })
                .build()
        };

        application.add_action_entries([
            search_units,
            open_info,
            open_dependencies,
            open_journal,
            open_file,
            orientation_mode,
            list_boots,
            properties_selector,
            properties_selector_general,
            print_debug,
            display_unit_properties,
        ]);

        application.set_accels_for_action("app.search_units", &["<Ctrl>f"]);
        application.set_accels_for_action("app.open_info", &["<Ctrl>i"]);
        application.set_accels_for_action("app.open_dependencies", &["<Ctrl>d"]);
        application.set_accels_for_action("app.open_journal", &["<Ctrl>j"]);
        application.set_accels_for_action("app.open_file", &["<Ctrl>u"]);
        application.set_accels_for_action("win.unit_list_filter_blank", &["<Ctrl><Shift>f"]);
        application.set_accels_for_action(APP_ACTION_LIST_BOOT, &["<Ctrl>b"]);
        application.set_accels_for_action("app.signals", &["<Ctrl>g"]);
        application.set_accels_for_action(APP_ACTION_PROPERTIES_SELECTOR_GENERAL, &["<Ctrl>r"]);
        application.set_accels_for_action("app.debug", &["<Ctrl>q"]);
        application.set_accels_for_action(APP_ACTION_UNIT_PROPERTIES_DISPLAY, &["<Ctrl>p"]);
    }

    pub fn overlay(&self) -> &adw::ToastOverlay {
        &self.toast_overlay
    }

    fn blue(&self) -> &str {
        blue().get_color()
    }

    fn green(&self) -> &str {
        green().get_color()
    }

    fn red(&self) -> &str {
        red().get_color()
    }

    pub(super) fn add_toast_message(
        &self,
        message: &str,
        use_markup: bool,
        action: Option<(&str, String, bool)>,
    ) {
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

        if let Some((action_name, ref button_label, user_session)) = action {
            info!("Toast action {:?} user_session {user_session}", action);
            toast.set_action_name(Some(action_name));
            toast.set_action_target_value(Some(&user_session.to_variant()));
            toast.set_button_label(Some(button_label));
        }

        self.toast_overlay.add_toast(toast)
    }

    pub fn update_list_boots(&self, boots: Vec<Rc<Boot>>) {
        self.list_boots.replace(Some(boots));
    }

    pub fn cached_list_boots(&self) -> Ref<'_, Option<Vec<Rc<Boot>>>> {
        self.list_boots.borrow()
    }

    pub fn cached_list_boots_mut(&self) -> RefMut<'_, Option<Vec<Rc<Boot>>>> {
        self.list_boots.borrow_mut()
    }

    fn replace_tags(&self, message: &str) -> String {
        debug!("{message}");
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
        debug!("{out}");
        out
    }
}

impl WidgetImpl for AppWindowImpl {}
impl WindowImpl for AppWindowImpl {
    // Save window state right before the window will be closed
    fn close_request(&self) -> glib::Propagation {
        #[cfg(not(feature = "flatpak"))]
        systemd::sysdbus::shut_down_proxy();
        // Save window size
        debug!("Close window");
        if let Err(_err) = self.save_window_context() {
            error!("Failed to save window state");
        }

        self.unit_list_panel.save_column_config();

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
            println!("capture: {capt:#?}")
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
                println!("capture: {capt:#?}")
            }
        }
    }
}
