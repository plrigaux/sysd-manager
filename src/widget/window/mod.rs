mod imp;

use glib::Object;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};
use log::info;

use crate::systemd_gui;

const WINDOW_WIDTH: &str = "window-width";
const WINDOW_HEIGHT: &str = "window-height";
const IS_MAXIMIZED: &str = "is-maximized";

// ANCHOR: mod
glib::wrapper! {
    pub struct AppWindow(ObjectSubclass<imp::Window>)
        @extends adw::ApplicationWindow, gtk::Window, adw::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl AppWindow {
    pub fn new(app: &adw::Application) -> Self {
        // Create new window
        Object::builder().property("application", app).build()
    }

    fn setup_settings(&self) {
        let settings = gio::Settings::new(systemd_gui::APP_ID);
        self.imp()
            .settings
            .set(settings)
            .expect("`settings` should not be set before calling `setup_settings`.");
    }

    fn settings(&self) -> &gio::Settings {
        self.imp()
            .settings
            .get()
            .expect("`settings` should be set in `setup_settings`.")
    }

    pub fn save_window_size(&self) -> Result<(), glib::BoolError> {
        // Get the size of the window
        let size = self.default_size();

        // Set the window state in `settings`
        let settings = self.settings();

        settings.set_int(WINDOW_WIDTH, size.0)?;
        settings.set_int(WINDOW_HEIGHT, size.1)?;
        settings.set_boolean(IS_MAXIMIZED, self.is_maximized())?;

        Ok(())
    }

    fn load_window_size(&self) {
        // Get the window state from `settings`
        let settings = self.settings();

        let mut width = settings.int(WINDOW_WIDTH);
        let mut height = settings.int(WINDOW_HEIGHT);
        let is_maximized = settings.boolean(IS_MAXIMIZED);

        info!("Window settings: width {width}, height {height}, is-maximized {is_maximized}");

        if width < 0 {
            width = 1280;
        }

        if height < 0 {
            height = 720;
        }
        // Set the size of the window
        self.set_default_size(width, height);

        // If the window was maximized when it was closed, maximize it again
        if is_maximized {
            self.maximize();
        }
    }

}
