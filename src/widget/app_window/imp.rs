use std::cell::OnceCell;

use adw::subclass::prelude::*;
use gtk::{gio, glib, prelude::*};
use log::{debug, info};

use crate::{
    systemd::data::UnitInfo,
    systemd_gui::new_settings,
    widget::{
        preferences::data::{DbusLevel, PREFERENCES},
        unit_control_panel::UnitControlPanel,
        unit_list::UnitListPanel,
    },
};

const WINDOW_WIDTH: &str = "window-width";
const WINDOW_HEIGHT: &str = "window-height";
const PANED_SEPARATOR_POSITION: &str = "paned-separator-position";
const IS_MAXIMIZED: &str = "is-maximized";

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
    unit_name_label: TemplateChild<gtk::Label>,

    #[template_child]
    search_toggle_button: TemplateChild<gtk::ToggleButton>,

    #[template_child]
    refresh_unit_list_button: TemplateChild<gtk::Button>,

    #[template_child]
    system_session_dropdown: TemplateChild<gtk::DropDown>,
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
        self.setup_settings();

        self.load_window_size();
        let app_window = self.obj();
        self.unit_list_panel
            .register_selection_change(&app_window, &self.refresh_unit_list_button);

        self.unit_control_panel
            .set_overlay(&app_window, &self.toast_overlay);

        self.setup_dropdown();
    }
}

#[gtk::template_callbacks]
impl AppWindowImpl {
    fn setup_settings(&self) {
        let settings: gio::Settings = new_settings();

        self.settings
            .set(settings)
            .expect("`settings` should not be set before calling `setup_settings`.");
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

    pub fn save_window_size(&self) -> Result<(), glib::BoolError> {
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

        Ok(())
    }

    fn load_window_size(&self) {
        // Get the window state from `settings`
        let settings = self.settings();

        let mut width = settings.int(WINDOW_WIDTH);
        let mut height = settings.int(WINDOW_HEIGHT);
        let is_maximized = settings.boolean(IS_MAXIMIZED);
        let mut separator_position = settings.int(PANED_SEPARATOR_POSITION);

        info!("Window settings: width {width}, height {height}, is-maximized {is_maximized}");

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
    }

    #[template_callback]
    fn button_search_toggled(&self, toggle_button: &gtk::ToggleButton) {
        self.unit_list_panel
            .button_search_toggled(toggle_button.is_active());
    }

    #[template_callback]
    fn refresh_button_clicked(&self, button: &gtk::Button) {
        info!("refresh false");
        button.set_sensitive(false);
        self.unit_list_panel.fill_store();
        button.set_sensitive(true);
        info!("refresh true");
    }

    pub(super) fn selection_change(&self, unit: &UnitInfo) {
        self.unit_name_label.set_label(&unit.primary());
        self.unit_control_panel.selection_change(unit);
    }

    pub(super) fn set_unit(&self, unit: Option<UnitInfo>) {
        if let Some(unit) = unit {
            self.selection_change(&unit);
            self.unit_list_panel.set_unit(&unit);
        }
    }

    pub(super) fn set_dark(&self, is_dark: bool) {
        self.unit_control_panel.set_dark(is_dark);
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

        application.add_action_entries([search_units, open_info, open_dependencies, open_journal, open_file]);

        application.set_accels_for_action("app.search_units", &["<Ctrl>f"]);
        application.set_accels_for_action("app.open_info", &["<Ctrl>i"]);
        application.set_accels_for_action("app.open_dependencies", &["<Ctrl>d"]);
        application.set_accels_for_action("app.open_journal", &["<Ctrl>j"]);
        application.set_accels_for_action("app.open_file", &["<Ctrl>u"]);
    }

    pub fn overlay(&self) -> &adw::ToastOverlay {
        &self.toast_overlay
    }

    pub(super) fn add_toast(&self, toast: adw::Toast) {
        self.toast_overlay.add_toast(toast)
    }
}

impl WidgetImpl for AppWindowImpl {}
impl WindowImpl for AppWindowImpl {
    // Save window state right before the window will be closed
    fn close_request(&self) -> glib::Propagation {
        // Save window size
        log::debug!("Close window");
        self.save_window_size()
            .expect("Failed to save window state");

        self.parent_close_request();
        // Allow to invoke other event handlers
        glib::Propagation::Proceed
    }
}
impl AdwApplicationWindowImpl for AppWindowImpl {}
impl ApplicationWindowImpl for AppWindowImpl {}
