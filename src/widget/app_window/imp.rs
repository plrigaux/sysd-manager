use std::cell::{OnceCell, RefCell};

use adw::{subclass::prelude::*, Toast};
use gtk::{
    gio,
    glib::{self, property::PropertySet},
    prelude::*,
};
use log::{debug, error, info, warn};

use crate::{
    systemd::{self, data::UnitInfo, enums::ActiveState},
    systemd_gui,
    widget::{
        journal::JournalPanel, kill_panel::KillPanel, title_bar::menu,
        unit_file_panel::UnitFilePanel, unit_info::UnitInfoPanel, unit_list::UnitListPanel,
    },
};

use super::controls;

const WINDOW_WIDTH: &str = "window-width";
const WINDOW_HEIGHT: &str = "window-height";
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
    unit_info_panel: TemplateChild<UnitInfoPanel>,

    #[template_child]
    unit_file_panel: TemplateChild<UnitFilePanel>,

    #[template_child]
    unit_journal_panel: TemplateChild<JournalPanel>,

    #[template_child]
    ablement_switch: TemplateChild<gtk::Switch>,

    #[template_child]
    start_button: TemplateChild<gtk::Button>,

    #[template_child]
    stop_button: TemplateChild<gtk::Button>,

    #[template_child]
    kill_button: TemplateChild<gtk::Button>,

    #[template_child]
    restart_button: TemplateChild<gtk::Button>,

    #[template_child]
    paned: TemplateChild<gtk::Paned>,

    #[template_child]
    unit_name_label: TemplateChild<gtk::Label>,

    #[template_child]
    side_overlay: TemplateChild<adw::OverlaySplitView>,

    #[template_child]
    kill_panel: TemplateChild<KillPanel>,

    current_unit: RefCell<Option<UnitInfo>>,

    search_bar: RefCell<gtk::SearchBar>,
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

macro_rules! current_unit {
    ($app:expr) => {{
        current_unit!($app, ())
    }};

    ($app:expr, $opt:expr) => {{
        let unit_op = $app.current_unit.borrow();
        let Some(unit) = unit_op.as_ref() else {
            warn!("No selected unit!");
            return $opt;
        };

        unit.clone()
    }};
}

impl ObjectImpl for AppWindowImpl {
    fn constructed(&self) {
        self.parent_constructed();
        // Load latest window state
        //let obj = self.obj();
        self.setup_settings();
        let (width, _height) = self.load_window_size();

        let menu_button = menu::build_menu();
        self.header_bar.pack_end(&menu_button);

        let app_window = self.obj();
        self.unit_list_panel.register_selection_change(&app_window);

        self.kill_panel.register(&self.side_overlay, &self.toast_overlay);

        let search_bar = self.unit_list_panel.search_bar();

        self.search_bar.set(search_bar);

        self.paned.set_position(width / 2);
    }
}

#[gtk::template_callbacks]
impl AppWindowImpl {
    fn setup_settings(&self) {
        let settings: gio::Settings = gio::Settings::new(systemd_gui::APP_ID);
        self.settings
            .set(settings)
            .expect("`settings` should not be set before calling `setup_settings`.");
    }

    fn settings(&self) -> &gio::Settings {
        self.settings
            .get()
            .expect("`settings` should be set in `setup_settings`.")
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

        Ok(())
    }

    fn load_window_size(&self) -> (i32, i32) {
        // Get the window state from `settings`
        let settings = self.settings();

        let mut width = settings.int(WINDOW_WIDTH);
        let mut height = settings.int(WINDOW_HEIGHT);
        let is_maximized = settings.boolean(IS_MAXIMIZED);

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

        (width, height)
    }

    #[template_callback]
    fn switch_ablement_state_set(&self, switch_new_state: bool, switch: &gtk::Switch) -> bool {
        //let unit = current_unit!(self, true);

        //controls::switch_ablement_state_set(self, state, switch, &unit);

        info!(
            "switch_ablement_state_set new {switch_new_state} ss {}",
            switch.state()
        );

        if switch_new_state == switch.state() {
            debug!("no state change");
            return true;
        }

        let unit_op = self.current_unit.borrow();

        let Some(unit) = unit_op.as_ref() else {
            warn!("No selected unit!");
            return true;
        };

        controls::switch_ablement_state_set(&self.toast_overlay, switch_new_state, switch, &unit);

        true // to stop the signal emission
    }

    #[template_callback]
    fn button_start_clicked(&self, _button: &gtk::Button) {
        let unit = current_unit!(self);

        match systemd::start_unit(&unit) {
            Ok(_job) => {
                let info = format!("Unit \"{}\" has been started!", unit.primary());

                info!("{info}");

                let toast = Toast::new(&info);
                self.toast_overlay.add_toast(toast);

                controls::update_active_state(&unit, ActiveState::Active);
            }
            Err(e) => error!("Can't start the unit {}, because: {:?}", unit.primary(), e),
        }
    }

    #[template_callback]
    fn button_stop_clicked(&self, _button: &gtk::Button) {
        let unit = current_unit!(self);

        match systemd::stop_unit(&unit) {
            Ok(_job) => {
                let info = format!("Unit \"{}\" has been stopped!", unit.primary());
                info!("{info}");
                let toast = Toast::new(&info);
                self.toast_overlay.add_toast(toast);

                controls::update_active_state(&unit, ActiveState::Inactive)
            }

            Err(e) => error!("Can't stop the unit {}, because: {:?}", unit.primary(), e),
        }
    }

    #[template_callback]
    fn button_restart_clicked(&self, _button: &gtk::Button) {
        let unit = current_unit!(self);

        match systemd::restart_unit(&unit) {
            Ok(_job) => {
                let info = format!("Unit \"{}\" has been restarted!", unit.primary());
                info!("{info}");
                let toast = Toast::new(&info);
                self.toast_overlay.add_toast(toast);

                controls::update_active_state(&unit, ActiveState::Active);
            }
            Err(e) => error!("Can't stop the unit {}, because: {:?}", unit.primary(), e),
        }
    }

    #[template_callback]
    fn button_kill_clicked(&self, _button: &gtk::Button) {
        //let unit = current_unit!(self);
        let collapsed = self.side_overlay.is_collapsed();
        self.side_overlay.set_collapsed(!collapsed);
    }

    #[template_callback]
    fn button_search_toggled(&self, toggle_button: &gtk::ToggleButton) {
        self.search_bar
            .borrow()
            .set_search_mode(toggle_button.is_active());
    }

    pub(super) fn selection_change(&self, unit: &UnitInfo) {
        self.current_unit.set(Some(unit.clone()));

        self.unit_name_label.set_label(&unit.primary());
        self.unit_info_panel.display_unit_info(unit);
        self.unit_file_panel.set_file_content(unit);
        self.unit_journal_panel.display_journal(unit);

        controls::handle_switch_sensivity(&self.ablement_switch, unit, true);

        self.start_button.set_sensitive(true);
        self.stop_button.set_sensitive(true);
        self.restart_button.set_sensitive(true);
    }

    pub(super) fn set_dark(&self, is_dark: bool) {
        self.unit_file_panel.set_dark(is_dark);
        self.unit_info_panel.set_dark(is_dark);
        self.unit_journal_panel.set_dark(is_dark);
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
        // Allow to invoke other event handlers
        glib::Propagation::Proceed
    }
}
impl AdwApplicationWindowImpl for AppWindowImpl {}
impl ApplicationWindowImpl for AppWindowImpl {}
