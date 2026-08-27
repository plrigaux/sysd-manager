use glib::Object;
use gtk::{gio, glib, subclass::prelude::*};
use systemd::errors::SystemdErrors;

use crate::systemd::data::UnitInfo;

use super::{InterPanelMessage, signals_dialog::SignalsWindow};

mod imp;
pub mod menu;

glib::wrapper! {
    pub struct AppWindow(ObjectSubclass<imp::AppWindowImpl>)
        @extends adw::ApplicationWindow, gtk::ApplicationWindow, gtk::Window, gtk::Widget,
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

    pub fn set_unit(&self, unit: Option<&UnitInfo>) -> Option<UnitInfo> {
        self.imp().set_unit(unit)
    }

    pub fn set_inter_message(&self, action: &InterPanelMessage) {
        self.imp().set_inter_message(action);
    }

    pub fn add_toast_message(&self, message: &str, markup: bool, action: Option<ToastAction<'_>>) {
        self.imp().add_toast_message(message, markup, action);
    }

    pub(super) fn add_toast_message_error(
        &self,
        message: &str,
        use_markup: bool,
        error: &SystemdErrors,
    ) {
        self.imp()
            .add_toast_message_error(message, use_markup, error);
    }

    pub fn selected_unit(&self) -> Option<UnitInfo> {
        let unit = self.imp().selected_unit.borrow();
        unit.clone()
    }

    pub fn signals_window(&self) -> Option<SignalsWindow> {
        self.imp().signals_window.borrow().as_ref().cloned()
    }

    pub fn set_signal_window(&self, signals_window: Option<&SignalsWindow>) {
        self.imp().signals_window.replace(signals_window.cloned());
    }
}

pub struct ToastAction<'a> {
    action_name: &'a str,
    button_label: String,
    target_value: Option<glib::Variant>,
}

impl<'a> ToastAction<'a> {
    pub(crate) fn new(
        action_name: &'a str,
        button_label: String,
        target_value: Option<glib::Variant>,
    ) -> Self {
        Self {
            action_name,
            button_label,
            target_value,
        }
    }

    pub(crate) fn new_t(
        action_name: &'a str,
        button_label: String,
        target_value: glib::Variant,
    ) -> Self {
        Self::new(action_name, button_label, Some(target_value))
    }
}
