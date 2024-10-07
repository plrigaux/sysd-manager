use gio::Settings;

use adw::subclass::prelude::{ObjectImpl, ObjectSubclass, WidgetImpl, WindowImpl, ApplicationWindowImpl, AdwApplicationWindowImpl};
use gtk::subclass::prelude::{ObjectImplExt, ObjectSubclassExt};
use gtk::{gio, glib};
use std::cell::OnceCell;

// ANCHOR: imp
#[derive(Default)]
pub struct Window {
    pub settings: OnceCell<Settings>,
}

#[glib::object_subclass]
impl ObjectSubclass for Window {
    const NAME: &'static str = "SysdAppWindow";
    type Type = super::AppWindow;
    type ParentType = adw::ApplicationWindow;
}
impl ObjectImpl for Window {
    fn constructed(&self) {
        self.parent_constructed();
        // Load latest window state
        let obj = self.obj();
        obj.setup_settings();
        obj.load_window_size();
    }
}
impl WidgetImpl for Window {}
impl WindowImpl for Window {
    // Save window state right before the window will be closed
    fn close_request(&self) -> glib::Propagation {
        // Save window size
        log::debug!("Close window");
        self.obj()
            .save_window_size()
            .expect("Failed to save window state");
        // Allow to invoke other event handlers
        glib::Propagation::Proceed
    }
}
impl AdwApplicationWindowImpl for Window {}
impl ApplicationWindowImpl for Window {}
// ANCHOR_END: imp
