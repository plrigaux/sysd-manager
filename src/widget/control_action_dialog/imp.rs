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
use std::cell::OnceCell;
use strum::IntoEnumIterator;

use crate::{
    systemd::{
        self,
        data::{DisEnAbleUnitFiles, EnableUnitFilesReturn, UnitInfo},
        enums::{DisEnableFlags, StartStopMode, UnitDBusLevel},
        errors::SystemdErrors,
    },
    systemd_gui,
    widget::{
        app_window::AppWindow,
        control_action_dialog::ControlActionType,
        unit_control_panel::{UnitControlPanel, enums::UnitContolType},
    },
};

use super::ControlActionDialog;

const SAVE_CONTEXT_ENABLE_UNIT_FILE_RUNTIME: &str = "save-context-enable-unit-file-runtime";
const SAVE_CONTEXT_ENABLE_UNIT_FILE_FORCE: &str = "save-context-enable-unit-file-force";
const SAVE_CONTEXT_ENABLE_UNIT_FILE_RUN_NOW: &str = "save-context-enable-unit-file-run-now";
const SAVE_CONTEXT_ENABLE_UNIT_FILE_START_MODE: &str = "save-context-enable-unit-file-start-mode";
const SAVE_CONTEXT_ENABLE_UNIT_FILE_DBUS_LEVEL: &str = "save-context-enable-unit-file-dbus-level";

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/control_action_dialog.ui")]
pub struct EnableUnitDialogImp {
    #[template_child]
    send_action_button: TemplateChild<gtk::Button>,

    #[template_child]
    unit_file_entry: TemplateChild<adw::EntryRow>,

    #[template_child]
    runtime_switch: TemplateChild<adw::SwitchRow>,

    #[template_child]
    force_switch: TemplateChild<adw::SwitchRow>,

    #[template_child]
    portable_switch: TemplateChild<adw::SwitchRow>,

    #[template_child]
    run_stop_now_switch: TemplateChild<adw::SwitchRow>,

    #[template_child]
    run_stop_mode_combo: TemplateChild<adw::ComboRow>,

    #[template_child]
    dbus_level_combo: TemplateChild<adw::ComboRow>,

    #[template_child]
    use_selected_unit_button: TemplateChild<gtk::Button>,

    #[template_child]
    first_group: TemplateChild<adw::PreferencesGroup>,

    #[template_child]
    after_action_group: TemplateChild<adw::PreferencesGroup>,

    #[template_child]
    window_title: TemplateChild<adw::WindowTitle>,

    unit: OnceCell<Option<UnitInfo>>,

    action_type: OnceCell<ControlActionType>,

    app_window: OnceCell<AppWindow>,

    unit_control: OnceCell<UnitControlPanel>,

    settings: OnceCell<gio::Settings>,
}

#[gtk::template_callbacks]
impl EnableUnitDialogImp {
    #[template_callback]
    fn send_action_button_clicked(&self, button: gtk::Button) {
        let unit_file = self.unit_file_entry.text();
        let unit_file2 = unit_file.clone();

        let dialog = self.obj().clone();

        let app_window = self.app_window.get().expect("Need window set").clone();

        let runtime = self.runtime_switch.is_active();
        let force = self.force_switch.is_active();

        let action_type = *self.action_type.get().expect("Value need to be set");
        match action_type {
            ControlActionType::EnableUnitFiles => {
                let dbus_level = self.dbus_level_combo.selected();
                let dbus_level: UnitDBusLevel = dbus_level.into();
                let handling_response_callback = {
                    move |_method: &str,
                          _unit: Option<&UnitInfo>,
                          result: Result<EnableUnitFilesReturn, SystemdErrors>,
                          control: &UnitControlPanel| {
                        match result {
                            Ok(vec) => {
                                info!("{} Result: {:?}", action_type.code(), vec);
                                let unit_name = unit_file.as_str();
                                if dialog.imp().run_stop_now_switch.is_active() {
                                    //TODO Check if Reload Units needed
                                    let mode = dialog.imp().run_stop_mode_combo.selected_item();
                                    let start_mode: StartStopMode = mode.into();
                                    info!(
                                        "Try to start {unit_name:?} level: {dbus_level:?} mode: {start_mode:?}"
                                    );

                                    let start_results =
                                        systemd::start_unit_name(dbus_level, unit_name, start_mode);

                                    control.start_restart(
                                        unit_name,
                                        None,
                                        start_results,
                                        UnitContolType::Start,
                                        start_mode,
                                    );
                                }

                                match systemd::fetch_unit(dbus_level, unit_name) {
                                    Ok(unit) => {
                                        let returned_unit = app_window.set_unit(Some(&unit));
                                        after_unit_file_action(
                                            "",
                                            returned_unit.as_ref(),
                                            Ok(()),
                                            control,
                                        );
                                    }
                                    Err(e) => {
                                        warn!(
                                            "Enable unit fetch {unit_name:?} level {dbus_level:?} Error: {e:?}"
                                        );
                                    }
                                }
                            }
                            Err(_error) => {
                                //handle by caller function
                            }
                        }
                    }
                };

                let mut flags = DisEnableFlags::empty();

                if force {
                    flags |= DisEnableFlags::SD_SYSTEMD_UNIT_FORCE
                }

                if self.portable_switch.is_active() {
                    flags |= DisEnableFlags::SD_SYSTEMD_UNIT_PORTABLE
                }

                if runtime {
                    flags |= DisEnableFlags::SD_SYSTEMD_UNIT_RUNTIME
                }

                let lambda = move |_params: Option<(UnitDBusLevel, String)>| {
                    systemd::enable_unit_file(dbus_level, unit_file2.as_str(), flags)
                };

                self.unit_control
                    .get()
                    .expect("unit_control not None")
                    .call_method(
                        &action_type.method_name(),
                        false,
                        &button,
                        lambda,
                        handling_response_callback,
                    );
            }
            ControlActionType::DisableUnitFiles => {
                let handling_response_callback = {
                    move |_method: &str,
                          unit: Option<&UnitInfo>,
                          result: Result<Vec<DisEnAbleUnitFiles>, SystemdErrors>,
                          control: &UnitControlPanel| {
                        match result {
                            Ok(ref vec) => {
                                info!("{} Result: {:?}", action_type.code(), vec);

                                if let Some(unit) = unit
                                    && dialog.imp().run_stop_now_switch.is_active()
                                {
                                    let mode = dialog.imp().run_stop_mode_combo.selected_item();
                                    let mode: StartStopMode = mode.into();
                                    info!("Stop Unit {:?} mode {:?}", unit.primary(), mode);
                                    let stop_results = systemd::stop_unit(
                                        unit.dbus_level(),
                                        &unit.primary(),
                                        mode,
                                    );

                                    control.start_restart(
                                        &unit.primary(),
                                        Some(unit),
                                        stop_results,
                                        UnitContolType::Stop,
                                        mode,
                                    );
                                }

                                let result = result.map(|_arg| ());
                                after_unit_file_action(
                                    &action_type.method_name(),
                                    unit,
                                    result,
                                    control,
                                );
                            }
                            Err(_error) => {}
                        }
                    }
                };

                let mut flags = DisEnableFlags::empty();

                if force {
                    flags |= DisEnableFlags::SD_SYSTEMD_UNIT_FORCE
                }

                if self.portable_switch.is_active() {
                    flags |= DisEnableFlags::SD_SYSTEMD_UNIT_PORTABLE
                }

                if runtime {
                    flags |= DisEnableFlags::SD_SYSTEMD_UNIT_RUNTIME
                }

                let lambda = move |params: Option<(UnitDBusLevel, String)>| {
                    if let Some((level, primary_name)) = params {
                        systemd::disable_unit_files(level, &primary_name, flags)
                    } else {
                        Err(SystemdErrors::NoUnit)
                    }
                };

                self.unit_control
                    .get()
                    .expect("unit_control not None")
                    .call_method(
                        &action_type.method_name(),
                        false,
                        &button,
                        lambda,
                        handling_response_callback,
                    );
            }
            ControlActionType::MaskUnit => {
                let handling_response_callback = {
                    move |_method: &str,
                          unit: Option<&UnitInfo>,
                          result: Result<Vec<DisEnAbleUnitFiles>, SystemdErrors>,
                          control: &UnitControlPanel| {
                        match result {
                            Ok(ref vec) => {
                                info!("{} Result: {:?}", action_type.code(), vec);

                                if let Some(unit) = unit
                                    && dialog.imp().run_stop_now_switch.is_active()
                                {
                                    let mode = dialog.imp().run_stop_mode_combo.selected_item();
                                    let mode: StartStopMode = mode.into();
                                    info!("Stop Unit {:?} mode {:?}", unit.primary(), mode);
                                    let stop_results = systemd::stop_unit(
                                        unit.dbus_level(),
                                        &unit.primary(),
                                        mode,
                                    );

                                    control.start_restart(
                                        &unit.primary(),
                                        Some(unit),
                                        stop_results,
                                        UnitContolType::Stop,
                                        mode,
                                    );
                                }

                                let result = result.map(|_arg| ());
                                after_unit_file_action("Mask", unit, result, control);
                            }
                            Err(_error) => {}
                        }
                    }
                };

                let lambda = move |params: Option<(UnitDBusLevel, String)>| {
                    if let Some((level, primary_name)) = params {
                        systemd::mask_unit_files(level, &primary_name, runtime, force)
                    } else {
                        Err(SystemdErrors::NoUnit)
                    }
                };

                self.unit_control
                    .get()
                    .expect("unit_control not None")
                    .call_method(
                        &action_type.method_name(),
                        false,
                        &button,
                        lambda,
                        handling_response_callback,
                    );
            }
            ControlActionType::Preset => {
                let handling_response_callback = {
                    move |_method: &str,
                          unit: Option<&UnitInfo>,
                          result: Result<EnableUnitFilesReturn, SystemdErrors>,
                          control: &UnitControlPanel| {
                        match result {
                            Ok(ref vec) => {
                                info!("{} Result: {:?}", action_type.code(), vec);

                                let result = result.map(|_arg| ());
                                after_unit_file_action("Preset", unit, result, control);
                            }
                            Err(_error) => {}
                        }
                    }
                };

                let lambda = move |params: Option<(UnitDBusLevel, String)>| {
                    if let Some((level, primary_name)) = params {
                        systemd::preset_unit_files(level, &primary_name, runtime, force)
                    } else {
                        Err(SystemdErrors::NoUnit)
                    }
                };

                self.unit_control
                    .get()
                    .expect("unit_control not None")
                    .call_method(
                        &action_type.method_name(),
                        false,
                        &button,
                        lambda,
                        handling_response_callback,
                    );
            }

            ControlActionType::Reenable => {
                let handling_response_callback = {
                    move |_method: &str,
                          unit: Option<&UnitInfo>,
                          result: Result<EnableUnitFilesReturn, SystemdErrors>,
                          control: &UnitControlPanel| {
                        match result {
                            Ok(ref vec) => {
                                info!("{} Result: {:?}", action_type.code(), vec);

                                let result = result.map(|_arg| ());
                                after_unit_file_action(
                                    &action_type.method_name(),
                                    unit,
                                    result,
                                    control,
                                );
                            }
                            Err(_error) => {}
                        }
                    }
                };

                let lambda = move |params: Option<(UnitDBusLevel, String)>| {
                    if let Some((level, primary_name)) = params {
                        systemd::reenable_unit_file(level, &primary_name, runtime, force)
                    } else {
                        Err(SystemdErrors::NoUnit)
                    }
                };

                self.unit_control
                    .get()
                    .expect("unit_control not None")
                    .call_method(
                        &action_type.method_name(),
                        false,
                        &button,
                        lambda,
                        handling_response_callback,
                    );
            }

            ControlActionType::Link => {
                let dbus_level = self.dbus_level_combo.selected();
                let dbus_level: UnitDBusLevel = dbus_level.into();
                let handling_response_callback = {
                    move |_method: &str,
                          _unit: Option<&UnitInfo>,
                          result: Result<Vec<DisEnAbleUnitFiles>, SystemdErrors>,
                          control: &UnitControlPanel| {
                        match result {
                            Ok(vec) => {
                                info!("{} Result: {:?}", action_type.code(), vec);
                                let unit_name = unit_file.as_str();

                                match systemd::fetch_unit(dbus_level, unit_name) {
                                    Ok(unit) => {
                                        let returned_unit = app_window.set_unit(Some(&unit));
                                        after_unit_file_action(
                                            "",
                                            returned_unit.as_ref(),
                                            Ok(()),
                                            control,
                                        );
                                    }
                                    Err(e) => {
                                        warn!(
                                            "{} fetch {:?} level {:?} Error: {:?}",
                                            action_type.code(),
                                            unit_name,
                                            dbus_level,
                                            e
                                        );
                                    }
                                }
                            }
                            Err(_error) => {
                                //handle by caller function
                            }
                        }
                    }
                };

                let lambda = move |_params: Option<(UnitDBusLevel, String)>| {
                    systemd::link_unit_files(dbus_level, unit_file2.as_str(), runtime, force)
                };

                self.unit_control
                    .get()
                    .expect("unit_control not None")
                    .call_method(
                        &action_type.method_name(),
                        false,
                        &button,
                        lambda,
                        handling_response_callback,
                    );
            }
        }

        /*   self.unit_control
        .get()
        .expect("unit_control not None")
        .call_method(
            /*Message answer*/ &pgettext("enable unit file", "Enable Unit File"),
            false,
            &button,
            lambda,
            handling_response_callback,
        ); */
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
        info!("unit_file_insert_text {text:?} {position:?} {pointer:?}");
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
        self.run_stop_now_switch.set_active(run_now);

        let start_mode: StartStopMode = start_mode.as_str().into();
        let position = start_mode.discriminant();
        self.run_stop_mode_combo.set_selected(position);
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
                Err(e) => warn!("Unit File Selection Error {e:?}"),
            },
        );
    }

    #[template_callback]
    fn use_selected_unit_clicked(&self, _button: gtk::Button) {
        if let Some(Some(selected_unit)) = self.unit.get() {
            self.unit_file_entry.set_text(&selected_unit.primary());
        }
    }

    fn set_send_button_sensitivity(&self) {
        let unit_file = self.unit_file_entry.text();

        //  let enable_button = if unit_file.is_empty() { false } else { true };
        self.send_action_button.set_sensitive(!unit_file.is_empty());
    }

    pub fn set_unit(&self, unit: Option<&UnitInfo>) {
        self.unit.set(unit.cloned()).expect("Unit set Once Only");

        if let Some(unit) = unit {
            if self.action_type.get().expect("Not None").dialog_subtitle() {
                self.window_title.set_subtitle(&unit.primary());
            }

            if !matches!(
                self.action_type.get().expect("Not none"),
                ControlActionType::EnableUnitFiles
            ) {
                self.send_action_button.set_sensitive(true);
            }
        }

        self.use_selected_unit_button.set_sensitive(unit.is_some());
    }

    pub(crate) fn set_action_type(&self, action_type: super::ControlActionType) {
        self.action_type.set(action_type).expect("Only set once");

        self.window_title.set_title(&action_type.title());

        self.first_group.set_title(&action_type.first_group_title());

        self.after_action_group
            .set_title(&action_type.after_group_title());

        self.unit_file_entry
            .set_visible(action_type.unit_file_entry_visible());

        self.dbus_level_combo
            .set_visible(action_type.dbus_level_combo_visible());

        self.portable_switch
            .set_visible(action_type.portable_switch_visible());

        self.after_action_group
            .set_visible(action_type.after_action_group_visible());

        self.send_action_button
            .set_label(&action_type.send_action_label());

        let (title, subtitle) = action_type.run_stop_now();
        self.run_stop_now_switch.set_title(&title);
        self.run_stop_now_switch.set_subtitle(&subtitle);

        let (title, subtitle) = action_type.run_stop_now_mode();
        self.run_stop_mode_combo.set_title(&title);
        self.run_stop_mode_combo.set_subtitle(&subtitle);
    }
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for EnableUnitDialogImp {
    const NAME: &'static str = "ControlAction_DIALOG";
    type Type = ControlActionDialog;
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

        self.run_stop_mode_combo.set_expression(Some(expression));
        self.run_stop_mode_combo.set_model(Some(&model));

        let mut levels_string = Vec::new();
        for level in UnitDBusLevel::iter() {
            levels_string.push(level.nice_label());
        }

        let level_str: Vec<&str> = levels_string.iter().map(|x| &**x).collect();
        let string_list = gtk::StringList::new(&level_str);
        self.dbus_level_combo.set_model(Some(&string_list));

        self.reset_button_clicked();
    }
}

impl WidgetImpl for EnableUnitDialogImp {}
impl WindowImpl for EnableUnitDialogImp {
    fn close_request(&self) -> glib::Propagation {
        // Save window size
        info!("Close window");

        let dbus_level: UnitDBusLevel = self.dbus_level_combo.selected().into();
        let runtime = self.runtime_switch.is_active();
        let force = self.force_switch.is_active();
        let run_now = self.run_stop_now_switch.is_active();
        let start_mode = self.run_stop_mode_combo.selected_item();
        let start_mode: StartStopMode = start_mode.into();

        let settings = self.settings.get().expect("Settings not None");

        fn settings_error(e: BoolError) {
            log::error!("Setting error {e:?}");
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

pub fn after_unit_file_action(
    _method_name: &str,
    unit: Option<&UnitInfo>,
    result: Result<(), SystemdErrors>,
    control: &UnitControlPanel,
) {
    if result.is_err() {
        return;
    }

    let Some(unit) = unit else {
        warn!("Unit None");
        return;
    };

    let unit = unit.clone();
    let control = control.clone();
    glib::spawn_future_local(async move {
        let (sender, receiver) = tokio::sync::oneshot::channel();

        let primary_name = unit.primary();
        let level = unit.dbus_level();
        let object_path = unit.object_path();
        crate::systemd::runtime().spawn(async move {
            let response =
                systemd::complete_single_unit_information(primary_name, level, object_path).await;

            sender
                .send(response)
                .expect("The channel needs to be open.");
        });

        let vec_unit_info = match receiver.await.expect("Tokio receiver works") {
            Ok(unit_files) => unit_files,
            Err(err) => {
                warn!("Fail to update Unit info {err:?}");
                return Err(err);
            }
        };

        if let Some(update) = vec_unit_info.into_iter().next() {
            unit.update_from_unit_info(update);
        }

        control.selection_change(Some(&unit));
        Ok::<(), SystemdErrors>(())
    });
}
