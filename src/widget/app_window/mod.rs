use glib::Object;
use gtk::{gio, glib, pango::FontDescription, subclass::prelude::*};

use crate::systemd::data::UnitInfo;

mod imp;
pub mod menu;

glib::wrapper! {
    pub struct AppWindow(ObjectSubclass<imp::AppWindowImpl>)
        @extends adw::ApplicationWindow, gtk::Window, adw::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl AppWindow {
    pub fn new(app: &adw::Application) -> Self {
        // Create new window
        let obj: Self = Object::builder().property("application", app).build();

        obj.imp().build_action(app);

        obj
    }

    pub fn selection_change(&self, unit: Option<&UnitInfo>) {
        self.imp().selection_change(unit);
    }

    pub fn set_dark(&self, is_dark: bool) {
        self.imp().set_dark(is_dark);
    }

    pub fn add_toast(&self, toast: adw::Toast) {
        self.imp().add_toast(toast);
    }

    pub fn set_unit(&self, unit: Option<&UnitInfo>) {
        self.imp().set_unit(unit);
    }

    pub fn refresh_panels(&self) {
        self.imp().refresh_panels();
    }

    pub fn set_text_font(&self, font_description: FontDescription) {
        self.imp().set_text_font(font_description);
    }
}
