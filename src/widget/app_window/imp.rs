use std::cell::{OnceCell, RefCell};

use adw::{subclass::prelude::*, Toast};
use gtk::{gio, glib::{self, property::PropertySet}, prelude::*};
use log::{error, info, warn};

use crate::{
    systemd::{
        self,
        data::UnitInfo,
        enums::{ActiveState, EnablementStatus},
    },
    systemd_gui,
    widget::{
        journal::JournalPanel, title_bar::menu, unit_file_panel::UnitFilePanel,
        unit_info::UnitInfoPanel, unit_list::UnitListPanel,
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
    restart_button: TemplateChild<gtk::Button>,

    current_unit: RefCell<Option<UnitInfo>>,
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
        //let obj = self.obj();
        self.setup_settings();
        self.load_window_size();

        let menu_button = menu::build_menu();
        self.header_bar.pack_end(&menu_button);

        self.unit_list_panel.register_selection_change(&self.obj());
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

    fn load_window_size(&self) {
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
    }

    #[template_callback]
    fn switch_ablement_state_set(&self, state: bool, switch: &gtk::Switch) -> bool {
        let unit = current_unit!(self, true);

        controls::switch_ablement_state_set(self, state, switch, &unit);

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
    fn button_search_clicked(&self, _button: &gtk::Button) {}

    pub(super) fn selection_change(&self, unit: &UnitInfo) {
        self.unit_info_panel.display_unit_info(unit);
        self.unit_file_panel.set_file_content(unit);
        self.unit_journal_panel.display_journal(unit);

        controls::handle_switch_sensivity(EnablementStatus::Unknown, &self.ablement_switch, unit);

        self.start_button.set_sensitive(true);
        self.stop_button.set_sensitive(true);
        self.restart_button.set_sensitive(true);

        self.current_unit.set(Some(unit.clone()));
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
