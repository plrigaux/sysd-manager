use std::cell::{OnceCell, RefCell};

use adw::{prelude::*, subclass::window::AdwWindowImpl};
use gettextrs::pgettext;
use gio::glib::BoolError;
use gtk::{
    glib::{self},
    subclass::{
        prelude::*,
        widget::{
            CompositeTemplateCallbacksClass, CompositeTemplateClass,
            CompositeTemplateInitializingExt, WidgetImpl,
        },
    },
};
use log::{info, warn};

use super::MaskUnitDialog;
use crate::{
    gtk::glib::property::PropertySet,
    systemd::enums::ActiveState,
    widget::{mask_unit_dialog::after_mask, unit_control_panel::enums::UnitContolType},
};
use crate::{
    systemd::{
        self,
        data::{DisEnAbleUnitFiles, UnitInfo},
        enums::StartStopMode,
        errors::SystemdErrors,
    },
    systemd_gui,
    widget::{InterPanelMessage, app_window::AppWindow, unit_control_panel::UnitControlPanel},
};

const SAVE_CONTEXT_MASK_UNIT_RUNTIME: &str = "save-context-mask-unit-runtime";
const SAVE_CONTEXT_MASK_UNIT_FORCE: &str = "save-context-mask-unit-force";
const SAVE_CONTEXT_MASK_UNIT_STOP_NOW: &str = "save-context-mask-unit-stop-now";
const SAVE_CONTEXT_MASK_UNIT_STOP_MODE: &str = "save-context-mask-unit-stop-mode";

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/mask_unit_dialog.ui")]
pub struct MaskUnitDialogImp {
    #[template_child]
    mask_button: TemplateChild<gtk::Button>,

    #[template_child]
    runtime_switch: TemplateChild<adw::SwitchRow>,

    #[template_child]
    force_switch: TemplateChild<adw::SwitchRow>,

    #[template_child]
    stop_now_switch: TemplateChild<adw::SwitchRow>,

    #[template_child]
    mask_unit_dialog_title: TemplateChild<adw::WindowTitle>,

    #[template_child]
    stop_mode_combo: TemplateChild<adw::ComboRow>,

    unit: RefCell<Option<UnitInfo>>,

    app_window: OnceCell<AppWindow>,

    unit_control: OnceCell<UnitControlPanel>,

    settings: OnceCell<gio::Settings>,
}

#[gtk::template_callbacks]
impl MaskUnitDialogImp {
    #[template_callback]
    fn mask_unit_button_clicked(&self, button: gtk::Button) {
        let stop_now = self.stop_now_switch.is_active();
        let mode = self.stop_mode_combo.selected_item();
        let mode: StartStopMode = mode.into();

        let lambda_out = {
            move |_method: &str,
                  unit: Option<&UnitInfo>,
                  result: Result<Vec<DisEnAbleUnitFiles>, SystemdErrors>,
                  control: &UnitControlPanel| {
                match result {
                    Ok(ref vec) => {
                        info!("Unit Masked {:?}", vec);

                        if let Some(unit) = unit {
                            if stop_now {
                                info!("Stop Unit {:?} mode {:?}", unit.primary(), mode);
                                let stop_results = systemd::stop_unit(unit, mode);

                                control.start_restart(
                                    &unit.primary(),
                                    Some(unit),
                                    stop_results,
                                    UnitContolType::Stop,
                                    ActiveState::Inactive,
                                    mode,
                                );
                            }
                        }

                        let result = result.map(|_arg| ());
                        after_mask("Mask", unit, result, control);
                    }
                    Err(_error) => {}
                }
            }
        };

        let runtime = self.runtime_switch.is_active();
        let force = self.force_switch.is_active();

        let lambda = move |unit: Option<&UnitInfo>| {
            systemd::mask_unit_files(unit.expect("Unit not None"), runtime, force)
        };

        self.unit_control
            .get()
            .expect("unit_control not None")
            .call_method(
                /*Message answer*/ &pgettext("mask", "Mask Unit File"),
                true,
                &button,
                lambda,
                lambda_out,
            );
    }

    #[template_callback]
    fn unit_file_apply(&self, _entry: adw::EntryRow) {
        info!("unit_file_apply");
    }

    #[template_callback]
    fn unit_file_insert_text(
        &self,
        text: &str,
        position: i32,
        pointer: glib::Value,
        _entry: adw::EntryRow,
    ) {
        info!(
            "unit_file_insert_text {:?} {:?} {:?}",
            text, position, pointer
        );
    }

    #[template_callback]
    fn reset_button_clicked(&self) {
        info!("reset_button_clicked");

        let settings = self.settings.get().expect("setting nor None");

        let runtime = settings.boolean(SAVE_CONTEXT_MASK_UNIT_RUNTIME);
        let force = settings.boolean(SAVE_CONTEXT_MASK_UNIT_FORCE);
        let stop_now = settings.boolean(SAVE_CONTEXT_MASK_UNIT_STOP_NOW);
        let stop_mode = settings.string(SAVE_CONTEXT_MASK_UNIT_STOP_MODE);

        self.runtime_switch.set_active(runtime);
        self.force_switch.set_active(force);
        self.stop_now_switch.set_active(stop_now);

        let stop_mode: StartStopMode = stop_mode.as_str().into();
        let position = stop_mode.discriminant();
        self.stop_mode_combo.set_selected(position);
    }

    pub(crate) fn set_app_window(
        &self,
        app_window: Option<&AppWindow>,
        unit_control: &UnitControlPanel,
    ) {
        if let Some(app_window) = app_window {
            self.app_window
                .set(app_window.clone())
                .expect("app_window set once");
        }

        let _ = self.unit_control.set(unit_control.clone());
    }

    pub(super) fn set_inter_message(&self, _action: &InterPanelMessage) {}

    fn set_send_button_sensitivity(&self) {
        let mut sensitive = self.unit.borrow().is_some();

        let stop_mode = self.stop_mode_combo.selected_item();
        let stop_mode: StartStopMode = stop_mode.into();

        let stop_switch_active = self.stop_now_switch.is_active();

        self.stop_mode_combo.set_sensitive(stop_switch_active);

        if stop_switch_active {
            sensitive &= StartStopMode::Isolate != stop_mode;
        }

        self.mask_button.set_sensitive(sensitive);
    }

    pub fn set_unit(&self, unit: Option<&UnitInfo>) {
        let unit = match unit {
            Some(u) => u,
            None => {
                warn!("set unit to None");
                self.unit.set(None);
                let sub_title = pgettext("mask", "No Unit Selected");
                self.mask_unit_dialog_title.set_subtitle(&sub_title);
                return;
            }
        };

        self.unit.set(Some(unit.clone()));

        let label_text = &unit.primary();

        self.mask_unit_dialog_title.set_subtitle(label_text);

        self.set_send_button_sensitivity();
    }
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for MaskUnitDialogImp {
    const NAME: &'static str = "MASK_UNIT_DIALOG";
    type Type = MaskUnitDialog;
    type ParentType = adw::Window;

    fn class_init(klass: &mut Self::Class) {
        // The layout manager determines how child widgets are laid out.
        klass.bind_template();
        klass.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for MaskUnitDialogImp {
    fn constructed(&self) {
        self.parent_constructed();

        let settings = systemd_gui::new_settings();
        self.settings
            .set(settings.clone())
            .expect("Settings set once only");

        let model = adw::EnumListModel::new(StartStopMode::static_type());

        let expression = gtk::PropertyExpression::new(
            adw::EnumListItem::static_type(),
            None::<gtk::Expression>,
            "nick",
        );

        self.stop_mode_combo.set_expression(Some(expression));
        self.stop_mode_combo.set_model(Some(&model));

        let dialog = self.obj().clone();
        self.stop_mode_combo
            .connect_selected_item_notify(move |combo_row| {
                let stop_mode = combo_row.selected_item();
                let stop_mode: StartStopMode = stop_mode.into();

                if StartStopMode::Isolate == stop_mode {
                    combo_row.add_css_class("warning");
                } else {
                    combo_row.remove_css_class("warning");
                }

                dialog.imp().set_send_button_sensitivity();
            });

        let dialog = self.obj().clone();
        self.stop_now_switch
            .connect_active_notify(move |_switch_row| {
                dialog.imp().set_send_button_sensitivity();
            });

        self.reset_button_clicked();
    }
}

impl WidgetImpl for MaskUnitDialogImp {}
impl WindowImpl for MaskUnitDialogImp {
    fn close_request(&self) -> glib::Propagation {
        // Save window size
        info!("Close window");

        let runtime = self.runtime_switch.is_active();
        let force = self.force_switch.is_active();
        let run_now = self.stop_now_switch.is_active();
        let stop_mode = self.stop_mode_combo.selected_item();
        let stop_mode: StartStopMode = stop_mode.into();

        let settings = self.settings.get().expect("Settings not None");

        fn settings_error(e: BoolError) {
            log::error!("Setting error {:?}", e);
        }

        let _ = settings
            .set_boolean(SAVE_CONTEXT_MASK_UNIT_RUNTIME, runtime)
            .map_err(settings_error);
        let _ = settings
            .set_boolean(SAVE_CONTEXT_MASK_UNIT_FORCE, force)
            .map_err(settings_error);
        let _ = settings
            .set_boolean(SAVE_CONTEXT_MASK_UNIT_STOP_NOW, run_now)
            .map_err(settings_error);
        let _ = settings
            .set_string(SAVE_CONTEXT_MASK_UNIT_STOP_MODE, stop_mode.as_str())
            .map_err(settings_error);

        self.parent_close_request();
        // Allow to invoke other event handlers
        glib::Propagation::Proceed
    }
}
impl AdwWindowImpl for MaskUnitDialogImp {}
