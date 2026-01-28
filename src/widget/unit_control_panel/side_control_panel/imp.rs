use std::cell::RefCell;

use gettextrs::pgettext;
use glib::{WeakRef, property::PropertySet};

use crate::{
    consts::{MENU_ACTION, WIN_MENU_ACTION},
    systemd::{self, data::UnitInfo, enums::StartStopMode, errors::SystemdErrors},
    systemd_gui::is_dark,
    widget::{
        InterPanelMessage,
        app_window::AppWindow,
        clean_dialog::CleanUnitDialog,
        control_action_dialog::{ControlActionDialog, ControlActionType},
        kill_panel::KillPanel,
        unit_control_panel::{UnitControlPanel, work_around_dialog},
    },
};
use base::enums::UnitDBusLevel;
use gtk::{
    gio::{self, MENU_ATTRIBUTE_TARGET},
    glib::{self, Variant, VariantTy},
    prelude::*,
    subclass::prelude::*,
};
use log::{error, warn};

use super::SideControlPanel;
use strum::IntoEnumIterator;

#[derive(Default, gtk::CompositeTemplate, glib::Properties)]
#[template(resource = "/io/github/plrigaux/sysd-manager/side_control_panel.ui")]
#[properties(wrapper_type = super::SideControlPanel)]
pub struct SideControlPanelImpl {
    #[property(get, set)]
    pub start_mode: RefCell<String>,
    #[property(get, set)]
    pub stop_mode: RefCell<String>,
    #[property(get, set)]
    pub restart_mode: RefCell<String>,

    #[template_child]
    enable_unit_file_button: TemplateChild<gtk::Button>,
    #[template_child]
    disable_unit_file: TemplateChild<gtk::Button>,
    #[template_child]
    send_kill_button: TemplateChild<gtk::Button>,
    #[template_child]
    queue_signal_button: TemplateChild<gtk::Button>,
    #[template_child]
    clean_button: TemplateChild<gtk::Button>,
    #[template_child]
    freeze_button: TemplateChild<gtk::Button>,
    #[template_child]
    thaw_unit_button: TemplateChild<gtk::Button>,
    #[template_child]
    mask_button: TemplateChild<gtk::Button>,
    #[template_child]
    unmask_button: TemplateChild<gtk::Button>,
    #[template_child]
    reload_unit_button: TemplateChild<adw::SplitButton>,
    #[template_child]
    preset_button: TemplateChild<gtk::Button>,
    #[template_child]
    reenable_button: TemplateChild<gtk::Button>,
    #[template_child]
    link_button: TemplateChild<gtk::Button>,

    kill_signal_window: RefCell<Option<KillPanel>>,
    queue_signal_window: RefCell<Option<KillPanel>>,

    control_panel: RefCell<WeakRef<UnitControlPanel>>,
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

    fn lambda_out<T>(
        method_name: &str,
        unit: Option<&UnitInfo>,
        result: Result<T, SystemdErrors>,
        control_panel: &UnitControlPanel,
    ) {
        if let Err(error) = result
            && let SystemdErrors::ZAccessDenied(_, _) = error
        {
            let cmd = format!(
                "sudo systemctl {} -u {}",
                method_name.to_ascii_lowercase(),
                unit.expect("Unit not None").primary()
            );

            let window = control_panel.parent_window();
            work_around_dialog(&cmd, &error, method_name, &window)
        }
    }

    #[template_callback]
    fn freeze_button_clicked(&self, button: &gtk::Button) {
        if let Some(parent) = self.control_panel() {
            parent.call_method(
                //action name
                &pgettext("action", "Freeze"),
                true,
                button,
                systemd::freeze_unit,
                Self::lambda_out,
            )
        }
    }

    #[template_callback]
    fn thaw_button_clicked(&self, button: &gtk::Button) {
        if let Some(parent) = self.control_panel() {
            parent.call_method(
                //action name
                &pgettext("action", "Thaw"),
                true,
                button,
                systemd::thaw_unit,
                Self::lambda_out,
            )
        }
    }

    #[template_callback]
    fn reload_unit_button_clicked(&self, button: &adw::SplitButton) {
        let Some(app_window) = self.app_window() else {
            warn!("no app window");
            return;
        };

        let Some(value) = app_window.action_state(MENU_ACTION) else {
            warn!("Reload unit has no mode");
            return;
        };

        let mode: StartStopMode = value.into();

        let lambda = move |params: Option<(UnitDBusLevel, String)>| {
            if let Some((level, primary_name)) = params {
                systemd::reload_unit(level, &primary_name, mode)
            } else {
                Err(SystemdErrors::NoUnit)
            }
        };

        if let Some(parent) = self.control_panel() {
            parent.call_method(
                //action name
                &pgettext("action", "Reload"),
                true,
                button,
                lambda,
                Self::lambda_out,
            )
        }
    }

    #[template_callback]
    fn clean_button_clicked(&self, _button: &gtk::Widget) {
        let app_window = self.app_window();

        if let Some(unit_control_panel) = self.control_panel() {
            let clean_dialog = CleanUnitDialog::new(&unit_control_panel);

            clean_dialog.set_transient_for(app_window.as_ref());
            //clean_dialog.set_modal(true);

            clean_dialog.present();
        }
    }

    #[template_callback]
    fn enable_unit_button_clicked(&self, _button: &gtk::Widget) {
        let app_window = self.app_window();

        if let Some(control_panel) = self.control_panel() {
            let enable_unit_dialog =
                ControlActionDialog::new(&control_panel, ControlActionType::EnableUnitFiles);

            enable_unit_dialog.set_transient_for(app_window.as_ref());
            //clean_dialog.set_modal(true);

            enable_unit_dialog.present();
        }
    }

    #[template_callback]
    fn preset_unit_files_button_clicked(&self, _button: &gtk::Widget) {
        self.show_dialog(ControlActionType::Preset);
    }

    #[template_callback]
    fn mask_button_clicked(&self) {
        self.show_dialog(ControlActionType::MaskUnit);
    }

    #[template_callback]
    fn disable_unit_files_button_clicked(&self, _button: &gtk::Button) {
        self.show_dialog(ControlActionType::DisableUnitFiles);
    }

    #[template_callback]
    fn reenable_unit_files_button_clicked(&self) {
        self.show_dialog(ControlActionType::Reenable);
    }

    #[template_callback]
    fn link_unit_files_button_clicked(&self) {
        self.show_dialog(ControlActionType::Link);
    }

    fn show_dialog(&self, action: ControlActionType) {
        let app_window = self.app_window();

        if let Some(parent) = self.control_panel() {
            let dialog = ControlActionDialog::new(&parent, action);
            dialog.set_transient_for(app_window.as_ref());

            dialog.present();
        }
    }

    #[template_callback]
    fn unmask_button_clicked(&self, button: &gtk::Widget) {
        let Some(unit) = self.current_unit() else {
            error!("No unit");
            return;
        };

        let runtime = unit.enable_status().is_runtime();
        let lambda = move |params: Option<(UnitDBusLevel, String)>| -> Result<(), SystemdErrors> {
            if let Some((level, primary_name)) = params {
                systemd::unmask_unit_files(level, &primary_name, runtime)?;
                Ok(())
            } else {
                Err(SystemdErrors::NoUnit)
            }
        };

        if let Some(parent) = self.control_panel() {
            parent.call_method(
                //action name
                &pgettext("action", "Unmask"),
                true,
                button,
                lambda,
                crate::widget::control_action_dialog::imp::after_unit_file_action,
            )
        }
    }
}

impl SideControlPanelImpl {
    pub(super) fn control_panel(&self) -> Option<UnitControlPanel> {
        self.control_panel.borrow().upgrade()
    }

    fn current_unit(&self) -> Option<UnitInfo> {
        self.control_panel().and_then(|p| p.current_unit())
    }

    pub(super) fn reload_unit_mode_changed(&self, mode: StartStopMode) {
        self.reload_unit_button.set_tooltip_text(Some(&format!(
            "Asks the specified unit to reload its configuration, mode: {}",
            mode.as_str()
        )));
    }

    pub(super) fn init(&self, unit_control_panel: &UnitControlPanel) {
        self.control_panel.set(unit_control_panel.downgrade());
        let default_mode = StartStopMode::default();
        self.reload_unit_mode_changed(default_mode);

        let default_state = default_mode.as_str().to_variant();

        let side_control = self.obj().clone();

        //FIXME: It has  too move away
        let reload_params_action_entry: gio::ActionEntry<AppWindow> =
            gio::ActionEntry::builder(MENU_ACTION)
                .activate(move |_app_window: &AppWindow, action, value| {
                    let Some(value) = value else {
                        warn!("{} has no value", WIN_MENU_ACTION);
                        return;
                    };

                    let mode: StartStopMode = value.into();
                    side_control.imp().reload_unit_mode_changed(mode);
                    action.set_state(value);
                })
                .parameter_type(Some(VariantTy::STRING))
                .state(default_state)
                .build();

        self.app_window()
            .expect("AppWindow has to be set")
            .add_action_entries([reload_params_action_entry]);
    }

    pub fn set_inter_message(&self, action: &InterPanelMessage) {
        if let InterPanelMessage::IsDark(_is_dark) = *action {}

        let kill_signal_window = self.kill_signal_window.borrow();
        if let Some(kill_signal_window) = kill_signal_window.as_ref() {
            kill_signal_window.set_inter_message(action);
        }

        let send_signal_window = self.queue_signal_window.borrow();
        if let Some(send_signal_window) = send_signal_window.as_ref() {
            send_signal_window.set_inter_message(action);
        }
    }

    fn kill_or_queue_new_window(
        &self,
        window_cell: &RefCell<Option<KillPanel>>,
        new_kill_window_fn: fn(Option<&UnitInfo>, bool, &SideControlPanel) -> KillPanel,
    ) {
        let unit = self.current_unit();
        let create_new = {
            let kill_signal_window = window_cell.borrow();
            if let Some(kill_signal_window) = kill_signal_window.as_ref() {
                kill_signal_window.set_inter_message(&InterPanelMessage::UnitChange(unit.as_ref()));
                kill_signal_window.set_inter_message(&InterPanelMessage::IsDark(is_dark()));

                if let Some(app_window) = self.app_window() {
                    //kill_signal_window.set_application(app_window.application().as_ref());
                    kill_signal_window.set_transient_for(Some(&app_window));
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
            let kill_signal_window = new_kill_window_fn(unit.as_ref(), is_dark(), &self.obj());
            kill_signal_window.present();

            window_cell.replace(Some(kill_signal_window));
        }
    }

    pub fn unlink_child(&self, is_signal: bool) {
        if is_signal {
            self.queue_signal_window.replace(None);
        } else {
            self.kill_signal_window.replace(None);
        }
    }

    pub fn more_action_popover_shown(&self) {
        let unit_option = self.current_unit();

        let sensitive = if let Some(unit) = unit_option {
            self.clean_button
                .set_sensitive(unit.active_state().is_inactive());
            true
        } else {
            self.clean_button.set_sensitive(false);
            false
        };

        self.send_kill_button.set_sensitive(sensitive);
        self.queue_signal_button.set_sensitive(sensitive);
        self.freeze_button.set_sensitive(sensitive);
        self.thaw_unit_button.set_sensitive(sensitive);
        self.mask_button.set_sensitive(sensitive);
        self.unmask_button.set_sensitive(sensitive);
        self.reload_unit_button.set_sensitive(sensitive);
        self.preset_button.set_sensitive(sensitive);
        self.reenable_button.set_sensitive(sensitive);
    }

    fn app_window(&self) -> Option<AppWindow> {
        self.control_panel().and_then(|cp| cp.app_window())
    }
}

#[glib::derived_properties]
impl ObjectImpl for SideControlPanelImpl {
    fn constructed(&self) {
        self.parent_constructed();

        //FIXME: It has  too move away
        let menu = gio::Menu::new();
        for mode in StartStopMode::iter() {
            let item = gio::MenuItem::new(Some(mode.as_str()), Some(WIN_MENU_ACTION));
            let target_value: Variant = mode.as_str().into();
            item.set_attribute_value(MENU_ATTRIBUTE_TARGET, Some(&target_value));
            menu.append_item(&item);
        }

        let popover = gtk::PopoverMenu::from_model(Some(&menu));

        self.reload_unit_button.set_popover(Some(&popover));
    }
}

impl WidgetImpl for SideControlPanelImpl {}
impl BoxImpl for SideControlPanelImpl {}
