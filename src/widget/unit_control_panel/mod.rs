use gtk::{glib, prelude::*, subclass::prelude::*};

use crate::systemd::{data::UnitInfo, errors::SystemdErrors};

use super::{InterPanelMessage, app_window::AppWindow};

mod controls;
mod enums;
mod imp;
pub mod side_control_panel;

glib::wrapper! {
    pub struct UnitControlPanel(ObjectSubclass<imp::UnitControlPanelImpl>)
    @extends gtk::Box, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl UnitControlPanel {
    pub fn selection_change(&self, unit: Option<&UnitInfo>) {
        self.imp().selection_change(unit);
    }

    pub fn set_app_window(&self, app_window: &AppWindow) {
        self.imp().set_overlay(app_window);
    }

    pub(super) fn add_toast_message(&self, message: &str, use_markup: bool) {
        self.imp().add_toast_message(message, use_markup);
    }

    pub fn display_info_page(&self) {
        self.imp().display_info_page();
    }

    pub fn display_dependencies_page(&self) {
        self.imp().display_dependencies_page();
    }

    pub fn display_journal_page(&self) {
        self.imp().display_journal_page();
    }

    pub fn display_definition_file_page(&self) {
        self.imp().display_definition_file_page();
    }

    pub fn refresh_panels(&self) {
        self.imp().refresh_panels();
    }

    pub fn set_inter_message(&self, action: &InterPanelMessage) {
        self.imp().set_inter_message(action);
    }

    pub fn unlink_child(&self, is_signal: bool) {
        self.imp().unlink_child(is_signal);
    }

    pub(super) fn call_method<T>(
        &self,
        method_name: &str,
        need_selected_unit: bool,
        button: &impl IsA<gtk::Widget>,
        systemd_method: impl Fn(Option<&UnitInfo>) -> Result<T, SystemdErrors>
        + std::marker::Send
        + 'static,
        return_handle: impl FnOnce(&str, Option<&UnitInfo>, Result<T, SystemdErrors>, &UnitControlPanel)
        + 'static,
    ) where
        T: Send + 'static,
    {
        self.imp().call_method(
            method_name,
            need_selected_unit,
            button,
            systemd_method,
            return_handle,
        );
    }

    pub fn parent_window(&self) -> gtk::Window {
        self.imp().parent_window()
    }
}

pub fn work_around_dialog(cmd: &str, err: &SystemdErrors, method: &str, window: &gtk::Window) {
    let content_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(15)
        .margin_start(10)
        .margin_end(10)
        .margin_top(5)
        .margin_bottom(15)
        .build();

    content_box.append(
        &gtk::Label::builder()
            .label(format!(
                "Unfortunately <b>SysD-Manager</b> can't perfom <b>{}</b> action.",
                method
            ))
            .use_markup(true)
            .build(),
    );
    content_box.append(
        &gtk::Label::builder()
            .label("The authorisation can be configured in the following file :")
            .build(),
    );

    let file_path = "/usr/share/dbus-1/system.d/org.freedesktop.systemd1.conf";
    content_box.append(
        &gtk::LinkButton::builder()
            .label(file_path)
            .uri(format!("file://{}", file_path))
            .build(),
    );

    content_box.append(
        &gtk::Label::builder()
            .label("\n\nOtherwise, you can try the bellow command line in your terminal")
            .build(),
    );

    let label_fallback = gtk::Label::builder()
        .label(cmd)
        .selectable(true)
        .wrap(true)
        .css_classes(["journal_message"])
        .build();

    content_box.append(&label_fallback);

    let tool_bar = adw::ToolbarView::builder().content(&content_box).build();
    tool_bar.add_top_bar(&adw::HeaderBar::new());

    let dialog = adw::Window::builder()
        .title(format!("Error {}", err.human_error_type()))
        .content(&tool_bar)
        .transient_for(window)
        .modal(true)
        .build();

    dialog.present();
}
