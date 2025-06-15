use std::cell::{Cell, OnceCell, RefCell};

use adw::{prelude::*, subclass::prelude::*};
use const_format::concatcp;
use gettextrs::pgettext;
use gtk::{
    gio::{self, MENU_ATTRIBUTE_TARGET},
    glib::{self, Variant, VariantTy},
};
use log::warn;

use crate::{
    systemd::{self, data::UnitInfo, enums::StartStopMode, errors::SystemdErrors},
    widget::{
        InterPanelMessage,
        app_window::AppWindow,
        clean_dialog::CleanUnitDialog,
        control_action_dialog::{ControlActionDialog, ControlActionType},
        kill_panel::KillPanel,
        unit_control_panel::{UnitControlPanel, work_around_dialog},
    },
};

use super::SideControlPanel;
use strum::IntoEnumIterator;

const MENU_ACTION: &str = "unit-reload";
const WIN_MENU_ACTION: &str = concatcp!("win.", MENU_ACTION);

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

    #[template_child]
    reload_unit_button: TemplateChild<adw::SplitButton>,

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

    fn lambda_out(
        method_name: &str,
        unit: Option<&UnitInfo>,
        result: Result<(), SystemdErrors>,
        control_panel: &UnitControlPanel,
    ) {
        if let Err(error) = result {
            if let SystemdErrors::ZAccessDenied(_, _) = error {
                let cmd = format!(
                    "sudo systemctl {} -u {}",
                    method_name.to_ascii_lowercase(),
                    unit.expect("Unit not None").primary()
                );

                let window = control_panel.parent_window();
                work_around_dialog(&cmd, &error, method_name, &window)
            }
        }
    }

    #[template_callback]
    fn freeze_button_clicked(&self, button: &gtk::Button) {
        self.parent().call_method(
            "Freeze",
            true,
            button,
            systemd::freeze_unit,
            Self::lambda_out,
        )
    }

    #[template_callback]
    fn thaw_button_clicked(&self, button: &gtk::Button) {
        self.parent().call_method(
            /*Message answer*/ &pgettext("thaw", "Thaw"),
            true,
            button,
            systemd::thaw_unit,
            Self::lambda_out,
        )
    }

    #[template_callback]
    fn reload_unit_button_clicked(&self, button: &adw::SplitButton) {
        let Some(app_window) = self.app_window.get() else {
            warn!("no app window");
            return;
        };

        let Some(value) = app_window.action_state(MENU_ACTION) else {
            warn!("Reload unit has no mode");
            return;
        };

        let mode: StartStopMode = value.into();

        let lambda =
            move |unit: Option<&UnitInfo>| systemd::reload_unit(unit.expect("Unit not None"), mode);

        self.parent().call_method(
            /*Message answer*/ &pgettext("reload", "Reload"),
            true,
            button,
            lambda,
            Self::lambda_out,
        )
    }

    #[template_callback]
    fn clean_button_clicked(&self, _button: &gtk::Widget) {
        let unit_binding = self.current_unit.borrow();
        let app_window = self.app_window.get();
        let parent = self.parent();

        let clean_dialog = CleanUnitDialog::new(
            unit_binding.as_ref(),
            self.is_dark.get(),
            app_window,
            parent,
        );

        clean_dialog.set_transient_for(app_window);
        //clean_dialog.set_modal(true);

        clean_dialog.present();
    }

    #[template_callback]
    fn enable_unit_button_clicked(&self, _button: &gtk::Widget) {
        let app_window = self.app_window.get();
        let parent = self.parent();
        let unit_binding = self.current_unit.borrow();

        let enable_unit_dialog = ControlActionDialog::new(
            unit_binding.as_ref(),
            app_window,
            parent,
            ControlActionType::EnableUnitFiles,
        );

        enable_unit_dialog.set_transient_for(app_window);
        //clean_dialog.set_modal(true);

        enable_unit_dialog.present();
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
        let unit_binding = self.current_unit.borrow();
        let app_window = self.app_window.get();
        let parent = self.parent();

        let dialog = ControlActionDialog::new(unit_binding.as_ref(), app_window, parent, action);
        dialog.set_transient_for(app_window);

        dialog.present();
    }

    #[template_callback]
    fn unmask_button_clicked(&self, button: &gtk::Widget) {
        let lambda = |unit: Option<&UnitInfo>| -> Result<(), SystemdErrors> {
            if let Some(unit) = unit {
                let runtime = unit.enable_status_enum().is_runtime();
                systemd::unmask_unit_files(unit, runtime)?;
            }
            Ok(())
        };

        self.parent().call_method(
            "Unmask",
            true,
            button,
            lambda,
            crate::widget::control_action_dialog::imp::after_unit_file_action,
        );
    }
}

impl SideControlPanelImpl {
    pub(super) fn parent(&self) -> &UnitControlPanel {
        self.parent.get().expect("Parent not supposed to be None")
    }

    pub(super) fn reload_unit_mode_changed(&self, mode: StartStopMode) {
        self.reload_unit_button.set_tooltip_text(Some(&format!(
            "Asks the specified unit to reload its configuration, mode: {}",
            mode.as_str()
        )));
    }

    pub(super) fn set_app_window(&self, app_window: &AppWindow) {
        self.app_window
            .set(app_window.clone())
            .expect("app_window set once");

        let default_mode = StartStopMode::default();
        self.reload_unit_mode_changed(default_mode);

        let default_state = default_mode.as_str().to_variant();

        let side_control = self.obj().clone();
        let reload_params_action_entry: gio::ActionEntry<AppWindow> =
            gio::ActionEntry::builder(MENU_ACTION)
                .activate(move |_app_window: &AppWindow, action, value| {
                    let Some(value) = value else {
                        warn!("{WIN_MENU_ACTION} has no value");
                        return;
                    };

                    let mode: StartStopMode = value.into();
                    side_control.imp().reload_unit_mode_changed(mode);
                    println!("asdfasdfasdf asdfas dfasd");
                    action.set_state(value);
                })
                .parameter_type(Some(VariantTy::STRING))
                .state(default_state)
                .build();

        app_window.add_action_entries([reload_params_action_entry]);
    }

    pub fn set_inter_message(&self, action: &InterPanelMessage) {
        match *action {
            InterPanelMessage::UnitChange(unit) => {
                #[allow(clippy::map_clone)]
                self.current_unit.replace(unit.map(|u| u.clone()));
            }
            InterPanelMessage::IsDark(is_dark) => {
                self.is_dark.set(is_dark);
            }
            _ => (),
        }

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
        let binding = self.current_unit.borrow();
        let create_new = {
            let kill_signal_window = window_cell.borrow();
            if let Some(kill_signal_window) = kill_signal_window.as_ref() {
                kill_signal_window
                    .set_inter_message(&InterPanelMessage::UnitChange(binding.as_ref()));
                kill_signal_window
                    .set_inter_message(&InterPanelMessage::IsDark(self.is_dark.get()));

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
            let kill_signal_window =
                new_kill_window_fn(binding.as_ref(), self.is_dark.get(), &self.obj());
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

    pub(super) fn set_parent(&self, parent: &UnitControlPanel) {
        let _ = self.parent.set(parent.clone());
    }
}

#[glib::derived_properties]
impl ObjectImpl for SideControlPanelImpl {
    fn constructed(&self) {
        self.parent_constructed();

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
