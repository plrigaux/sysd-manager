use std::cell::OnceCell;

use adw::{prelude::*, subclass::window::AdwWindowImpl};
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

use crate::{
    systemd::{
        self,
        data::{DisEnAbleUnitFiles, UnitInfo},
        enums::{ActiveState, DisEnableFlags, StartStopMode, UnitDBusLevel},
        errors::SystemdErrors,
    },
    systemd_gui,
    widget::{
        InterPanelMessage,
        app_window::AppWindow,
        unit_control_panel::{UnitControlPanel, enums::UnitContolType},
    },
};

use super::EnableUnitDialog;

const SAVE_CONTEXT_ENABLE_UNIT_FILE_RUNTIME: &str = "save-context-enable-unit-file-runtime";
const SAVE_CONTEXT_ENABLE_UNIT_FILE_FORCE: &str = "save-context-enable-unit-file-force";
const SAVE_CONTEXT_ENABLE_UNIT_FILE_RUN_NOW: &str = "save-context-enable-unit-file-run-now";
const SAVE_CONTEXT_ENABLE_UNIT_FILE_START_MODE: &str = "save-context-enable-unit-file-start-mode";
const SAVE_CONTEXT_ENABLE_UNIT_FILE_DBUS_LEVEL: &str = "save-context-enable-unit-file-dbus-level";

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/enable_unit_dialog.ui")]
pub struct EnableUnitDialogImp {
    #[template_child]
    enable_button: TemplateChild<gtk::Button>,

    #[template_child]
    unit_file_entry: TemplateChild<adw::EntryRow>,

    #[template_child]
    runtime_switch: TemplateChild<adw::SwitchRow>,

    #[template_child]
    force_switch: TemplateChild<adw::SwitchRow>,

    #[template_child]
    portable_switch: TemplateChild<adw::SwitchRow>,

    #[template_child]
    run_now_switch: TemplateChild<adw::SwitchRow>,

    #[template_child]
    run_mode_combo: TemplateChild<adw::ComboRow>,

    #[template_child]
    dbus_level_combo: TemplateChild<adw::ComboRow>,

    app_window: OnceCell<AppWindow>,

    unit_control: OnceCell<UnitControlPanel>,

    settings: OnceCell<gio::Settings>,
}

#[gtk::template_callbacks]
impl EnableUnitDialogImp {
    #[template_callback]
    fn enable_unit_file_button_clicked(&self, button: gtk::Button) {
        let unit_file = self.unit_file_entry.text();
        let unit_file2 = unit_file.clone();

        let dbus_level = self.dbus_level_combo.selected_item();
        let dbus_level: UnitDBusLevel = dbus_level.into();
        let dialog = self.obj().clone();

        let app_window = self.app_window.get().expect("Need window set").clone();
        let lambda_out = {
            move |_method: &str,
                  _unit: Option<&UnitInfo>,
                  result: Result<Vec<DisEnAbleUnitFiles>, SystemdErrors>,
                  control: &UnitControlPanel| {
                match result {
                    Ok(vec) => {
                        info!("Enable Unit File Result: {:?}", vec); //TODO enable start
                        let unit_name = unit_file.as_str();
                        if dialog.imp().run_now_switch.is_active() {
                            //TODO Check if Reload Units needed
                            let mode = dialog.imp().run_mode_combo.selected_item();
                            let start_mode: StartStopMode = mode.into();
                            info!(
                                "Try to start {:?} level: {:?} mode: {:?}",
                                unit_name, dbus_level, start_mode
                            );

                            let start_results =
                                systemd::start_unit_name(dbus_level, unit_name, start_mode);

                            control.start_restart(
                                unit_name,
                                None,
                                start_results,
                                UnitContolType::Start,
                                ActiveState::Active,
                                start_mode,
                            );
                        }

                        let unit = match systemd::fetch_unit(dbus_level, unit_name) {
                            Ok(unit) => {
                                warn!("Active state {}", unit.active_state());
                                Some(unit)
                            }
                            Err(e) => {
                                warn!(
                                    "Enable unit fetch {:?} level {:?} Error: {:?}",
                                    unit_name, dbus_level, e
                                );
                                None
                            }
                        };

                        app_window.set_unit(unit.as_ref());
                    }
                    Err(_error) => {}
                }
            }
        };

        let mut flags = DisEnableFlags::empty();

        if self.force_switch.is_active() {
            flags |= DisEnableFlags::SD_SYSTEMD_UNIT_FORCE
        }

        if self.portable_switch.is_active() {
            flags |= DisEnableFlags::SD_SYSTEMD_UNIT_PORTABLE
        }

        if self.runtime_switch.is_active() {
            flags |= DisEnableFlags::SD_SYSTEMD_UNIT_RUNTIME
        }

        let lambda = move |_unit: Option<&UnitInfo>| {
            systemd::enable_unit_file(unit_file2.as_str(), dbus_level, flags)
        };

        self.unit_control
            .get()
            .expect("unit_control not None")
            .call_method("Enable Unit File", false, &button, lambda, lambda_out);
    }

    #[template_callback]
    fn unit_file_changed(&self, _entry: adw::EntryRow) {
        info!("unit_file_changed");

        self.set_send_button_sensitivity()
    }

    #[template_callback]
    fn unit_file_apply(&self, _entry: adw::EntryRow) {
        info!("unit_file_apply");
    }

    #[template_callback]
    fn unit_file_entry_activated(&self, _entry: adw::EntryRow) {
        info!("unit_file_entry_activated");
    }

    #[template_callback]
    fn unit_file_delete_text(&self, a: i32, b: i32, _entry: adw::EntryRow) {
        info!("unit_file_delete_text {a} {b}");
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

        self.unit_file_entry.set_text("");

        let dbus_level = settings.string(SAVE_CONTEXT_ENABLE_UNIT_FILE_DBUS_LEVEL);
        let runtime = settings.boolean(SAVE_CONTEXT_ENABLE_UNIT_FILE_RUNTIME);
        let force = settings.boolean(SAVE_CONTEXT_ENABLE_UNIT_FILE_FORCE);
        let run_now = settings.boolean(SAVE_CONTEXT_ENABLE_UNIT_FILE_RUN_NOW);
        let start_mode = settings.string(SAVE_CONTEXT_ENABLE_UNIT_FILE_START_MODE);

        let dbus_level: UnitDBusLevel = dbus_level.as_str().into();
        let position = dbus_level.value() as u32;
        self.dbus_level_combo.set_selected(position);

        self.runtime_switch.set_active(runtime);
        self.force_switch.set_active(force);
        self.run_now_switch.set_active(run_now);

        let start_mode: StartStopMode = start_mode.as_str().into();
        let position = start_mode.discriminant();
        self.run_mode_combo.set_selected(position);
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

    #[template_callback]
    fn file_bowser_clicked(&self, _button: gtk::Button) {
        let file_dialog = gtk::FileDialog::builder()
            .title("Select a unit file")
            .accept_label("Select")
            .build();

        let enable_unit_dialog = self.obj().clone();
        let window: gtk::Window = enable_unit_dialog.clone().into();

        file_dialog.open(
            Some(&window),
            None::<&gio::Cancellable>,
            move |result| match result {
                Ok(file) => {
                    if let Some(path) = file.path() {
                        let file_path_str = path.display().to_string();
                        enable_unit_dialog
                            .imp()
                            .unit_file_entry
                            .set_text(&file_path_str);
                    }
                }
                Err(e) => warn!("Unit File Selection Error {:?}", e),
            },
        );
    }

    #[template_callback]
    fn unit_bowser_clicked(&self, _button: gtk::Button) {}

    pub(super) fn set_inter_message(&self, _action: &InterPanelMessage) {}

    fn set_send_button_sensitivity(&self) {
        let unit_file = self.unit_file_entry.text();

        //  let enable_button = if unit_file.is_empty() { false } else { true };
        self.enable_button.set_sensitive(!unit_file.is_empty());
    }
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for EnableUnitDialogImp {
    const NAME: &'static str = "ENABLE_UNIT_DIALOG";
    type Type = EnableUnitDialog;
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

impl ObjectImpl for EnableUnitDialogImp {
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

        self.run_mode_combo.set_expression(Some(expression));
        self.run_mode_combo.set_model(Some(&model));

        let model = adw::EnumListModel::new(UnitDBusLevel::static_type());

        let expression = gtk::PropertyExpression::new(
            adw::EnumListItem::static_type(),
            None::<gtk::Expression>,
            "nick",
        );

        self.dbus_level_combo.set_expression(Some(expression));
        self.dbus_level_combo.set_model(Some(&model));

        self.reset_button_clicked();
    }
}

impl WidgetImpl for EnableUnitDialogImp {}
impl WindowImpl for EnableUnitDialogImp {
    fn close_request(&self) -> glib::Propagation {
        // Save window size
        info!("Close window");

        let dbus_level = self.dbus_level_combo.selected_item();
        let dbus_level: UnitDBusLevel = dbus_level.into();
        let runtime = self.runtime_switch.is_active();
        let force = self.force_switch.is_active();
        let run_now = self.run_now_switch.is_active();
        let start_mode = self.run_mode_combo.selected_item();
        let start_mode: StartStopMode = start_mode.into();

        let settings = self.settings.get().expect("Settings not None");

        fn settings_error(e: BoolError) {
            log::error!("Setting error {:?}", e);
        }

        let _ = settings
            .set_string(
                SAVE_CONTEXT_ENABLE_UNIT_FILE_DBUS_LEVEL,
                dbus_level.as_str(),
            )
            .map_err(settings_error);
        let _ = settings
            .set_boolean(SAVE_CONTEXT_ENABLE_UNIT_FILE_RUNTIME, runtime)
            .map_err(settings_error);
        let _ = settings
            .set_boolean(SAVE_CONTEXT_ENABLE_UNIT_FILE_FORCE, force)
            .map_err(settings_error);
        let _ = settings
            .set_boolean(SAVE_CONTEXT_ENABLE_UNIT_FILE_RUN_NOW, run_now)
            .map_err(settings_error);
        let _ = settings
            .set_string(
                SAVE_CONTEXT_ENABLE_UNIT_FILE_START_MODE,
                start_mode.as_str(),
            )
            .map_err(settings_error);

        self.parent_close_request();
        // Allow to invoke other event handlers
        glib::Propagation::Proceed
    }
}
impl AdwWindowImpl for EnableUnitDialogImp {}
