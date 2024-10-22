use std::cell::OnceCell;

use adw::subclass::prelude::*;
use gtk::{gio, glib, prelude::*, subclass::widget::*};
use log::info;

use crate::systemd_gui;

const WINDOW_WIDTH: &str = "window-width";
const WINDOW_HEIGHT: &str = "window-height";
const IS_MAXIMIZED: &str = "is-maximized";

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/app_window.ui")]
pub struct AppWindowImpl {
    pub settings: OnceCell<gio::Settings>,
}

#[glib::object_subclass]
impl ObjectSubclass for AppWindowImpl {
    const NAME: &'static str = "SysdMainAppWindow";
    type Type = super::AppWindow;
    type ParentType = adw::ApplicationWindow;

    fn class_init(klass: &mut Self::Class) {
        // The layout manager determines how child widgets are laid out.
        klass.bind_template();
        //klass.bind_template_callbacks();
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
    }
}

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
}

impl WidgetImpl for AppWindowImpl {}
impl WindowImpl for AppWindowImpl {}
/* impl WindowImpl for AppWindowImpl {
   /*  // Save window state right before the window will be closed
    fn close_request(&self) -> glib::Propagation {
        // Save window size
        log::debug!("Close window");
        self.obj()
            .save_window_size()
            .expect("Failed to save window state");
        // Allow to invoke other event handlers
        glib::Propagation::Proceed
    } */
} */
impl AdwApplicationWindowImpl for AppWindowImpl {}
impl ApplicationWindowImpl for AppWindowImpl {}
