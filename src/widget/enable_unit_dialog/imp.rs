use std::cell::OnceCell;

use adw::{prelude::*, subclass::window::AdwWindowImpl};
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
        data::UnitInfo,
        enums::{DisEnableFlags, StartStopMode, UnitDBusLevel},
        errors::SystemdErrors,
    },
    widget::{
        InterPanelMessage,
        app_window::AppWindow,
        unit_control_panel::{UnitControlPanel, work_around_dialog},
    },
};

use super::EnableUnitDialog;

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

    app_window: OnceCell<AppWindow>,

    unit_control: OnceCell<UnitControlPanel>,
}

#[gtk::template_callbacks]
impl EnableUnitDialogImp {
    #[template_callback]
    fn enable_unit_file_button_clicked(&self, button: gtk::Button) {
        let lambda_out = {
            let this = self.obj().clone();
            move |method: &str,
                  unit: &UnitInfo,
                  result: Result<(), SystemdErrors>,
                  _control: &UnitControlPanel| {
                if let Err(error) = result {
                    if let SystemdErrors::ZAccessDenied(_, _) = error {
                        let mut cmd = "sudo systemctl clean ".to_owned();

                        cmd.push_str(&unit.primary());
                        work_around_dialog(&cmd, &error, method, &this.into())
                    }
                }
            }
        };

        let unit_file = self.unit_file_entry.text();

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

        let lambda = move |_unit: &UnitInfo| match systemd::enable_unit_file(
            unit_file.as_str(),
            UnitDBusLevel::System,
            flags,
        ) {
            Ok(a) => {
                info!("Enable Response {:?}", a);
                Ok(())
            }
            Err(e) => Err(e),
        };

        self.unit_control
            .get()
            .expect("unit_control not None")
            .call_method("Enable Unit File", &button, lambda, lambda_out);
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
    fn reset_button_clicked(&self, _button: gtk::Button) {
        info!("reset_button_clicked");
        self.unit_file_entry.set_text("");
        self.runtime_switch.set_active(false);
        self.force_switch.set_active(false);
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

        let model = adw::EnumListModel::new(StartStopMode::static_type());

        let expression = gtk::PropertyExpression::new(
            adw::EnumListItem::static_type(),
            None::<gtk::Expression>,
            "name",
        );

        self.run_mode_combo.set_expression(Some(expression));
        self.run_mode_combo.set_model(Some(&model));
    }
}

impl WidgetImpl for EnableUnitDialogImp {}
impl WindowImpl for EnableUnitDialogImp {}
impl AdwWindowImpl for EnableUnitDialogImp {}
