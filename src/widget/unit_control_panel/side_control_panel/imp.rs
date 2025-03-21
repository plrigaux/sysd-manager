use std::cell::{Cell, OnceCell, RefCell};

use adw::{prelude::*, subclass::prelude::*};
use gtk::glib::{self};
use log::warn;

use crate::{
    systemd::data::UnitInfo,
    widget::{
        InterPanelAction, app_window::AppWindow, clean_dialog::CleanDialog, kill_panel::KillPanel,
        unit_control_panel::UnitControlPanel,
    },
};

#[derive(Default, gtk::CompositeTemplate, glib::Properties)]
#[template(resource = "/io/github/plrigaux/sysd-manager/side_control_panel.ui")]
#[properties(wrapper_type = super::SideControlPanel)]
pub struct SideControlPanelImpl {
    app_window: OnceCell<AppWindow>,

    current_unit: RefCell<Option<UnitInfo>>,

    #[property(get, set)]
    pub start_mode: RefCell<String>,
    #[property(get, set)]
    pub stop_mode: RefCell<String>,
    #[property(get, set)]
    pub restart_mode: RefCell<String>,

    kill_signal_window: RefCell<Option<KillPanel>>,
    queue_signal_window: RefCell<Option<KillPanel>>,

    parent: OnceCell<UnitControlPanel>,

    is_dark: Cell<bool>,
}

#[glib::object_subclass]
impl ObjectSubclass for SideControlPanelImpl {
    const NAME: &'static str = "SideControlPanel";
    type Type = super::SideControlPanel;
    type ParentType = gtk::Box;

    fn class_init(klass: &mut Self::Class) {
        // The layout manager determines how child widgets are laid out.
        klass.bind_template();
        klass.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

#[gtk::template_callbacks]
impl SideControlPanelImpl {
    #[template_callback]
    fn sidebar_close_button_clicked(&self, _button: &gtk::Button) {
        //self.side_overlay.set_collapsed(true);
    }

    #[template_callback]
    fn kill_button_clicked(&self, _button: &gtk::Button) {
        self.kill_or_queue_new_window(&self.kill_signal_window, KillPanel::new_kill_window);
    }

    #[template_callback]
    fn send_signal_button_clicked(&self, _button: &gtk::Button) {
        self.kill_or_queue_new_window(&self.queue_signal_window, KillPanel::new_signal_window);
    }

    #[template_callback]
    fn clean_button_clicked(&self, _button: &gtk::Button) {
        let binding = self.current_unit.borrow();

        let app_window = self.app_window.get();

        let clean_dialog = CleanDialog::new(binding.as_ref(), self.is_dark.get(), app_window);

        clean_dialog.set_transient_for(app_window);
        //clean_dialog.set_modal(true);

        clean_dialog.present();
    }
}

impl SideControlPanelImpl {
    pub(super) fn set_app_window(&self, app_window: &AppWindow) {
        self.app_window
            .set(app_window.clone())
            .expect("app_window set once");
    }

    pub fn set_inter_action(&self, action: &InterPanelAction) {
        match *action {
            InterPanelAction::UnitChange(unit) => {
                #[allow(clippy::map_clone)]
                self.current_unit.replace(unit.map(|u| u.clone()));
            }
            InterPanelAction::IsDark(is_dark) => {
                self.is_dark.set(is_dark);
            }
            _ => (),
        }

        let kill_signal_window = self.kill_signal_window.borrow();
        if let Some(kill_signal_window) = kill_signal_window.as_ref() {
            kill_signal_window.set_inter_action(action);
        }

        let send_signal_window = self.queue_signal_window.borrow();
        if let Some(send_signal_window) = send_signal_window.as_ref() {
            send_signal_window.set_inter_action(action);
        }
    }

    fn kill_or_queue_new_window(
        &self,
        window_cell: &RefCell<Option<KillPanel>>,
        new_kill_window_fn: fn(Option<&UnitInfo>, bool, &UnitControlPanel) -> KillPanel,
    ) {
        let binding = self.current_unit.borrow();
        let create_new = {
            let kill_signal_window = window_cell.borrow();
            if let Some(kill_signal_window) = kill_signal_window.as_ref() {
                kill_signal_window
                    .set_inter_action(&InterPanelAction::UnitChange(binding.as_ref()));
                kill_signal_window.set_inter_action(&InterPanelAction::IsDark(self.is_dark.get()));

                if let Some(app_window) = self.app_window.get() {
                    //kill_signal_window.set_application(app_window.application().as_ref());
                    kill_signal_window.set_transient_for(Some(app_window));
                    kill_signal_window.set_modal(true);
                } else {
                    warn!("No app_window");
                }

                kill_signal_window.present();
                false
            } else {
                true
            }
        };

        if create_new {
            if let Some(parent) = self.parent.get() {
                let kill_signal_window =
                    new_kill_window_fn(binding.as_ref(), self.is_dark.get(), parent);
                kill_signal_window.present();

                window_cell.replace(Some(kill_signal_window));
            } else {
                warn!("No parent panel link, ask dev");
            }
        }
    }

    pub fn unlink_child(&self, is_signal: bool) {
        if is_signal {
            self.queue_signal_window.replace(None);
        } else {
            self.kill_signal_window.replace(None);
        }
    }

    pub(super) fn set_parent(&self, parent: &UnitControlPanel) {
        let _ = self.parent.set(parent.clone());
    }
}

#[glib::derived_properties]
impl ObjectImpl for SideControlPanelImpl {
    fn constructed(&self) {
        self.parent_constructed();
    }
}

impl WidgetImpl for SideControlPanelImpl {}
impl BoxImpl for SideControlPanelImpl {}
