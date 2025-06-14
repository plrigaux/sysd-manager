mod imp;

use gettextrs::pgettext;
use gtk::{
    glib::{self},
    subclass::prelude::ObjectSubclassIsExt,
};

use crate::systemd::data::UnitInfo;

use super::{app_window::AppWindow, unit_control_panel::UnitControlPanel};

// ANCHOR: mod
glib::wrapper! {
    pub struct ControlActionDialog(ObjectSubclass<imp::EnableUnitDialogImp>)
    @extends adw::Window, gtk::Window, gtk::Widget,
    @implements gtk::Accessible,  gtk::Buildable,  gtk::ConstraintTarget,
    gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl ControlActionDialog {
    pub fn new(
        unit: Option<&UnitInfo>,
        app_window: Option<&AppWindow>,
        unit_control: &UnitControlPanel,
        action_type: ControlActionType,
    ) -> Self {
        let obj: ControlActionDialog = glib::Object::new();
        let imp = obj.imp();
        imp.set_app_window(app_window, unit_control);
        imp.set_action_type(action_type);
        imp.set_unit(unit);

        obj
    }
}
#[derive(Debug, Copy, Clone)]
pub enum ControlActionType {
    EnableUnitFiles,
    MaskUnit,
    Preset,
    DisableUnitFiles,
    Reenable,
}

impl ControlActionType {
    pub fn code(&self) -> &str {
        match self {
            ControlActionType::EnableUnitFiles => "Enable Unit File",
            ControlActionType::MaskUnit => "Mask Unit",
            ControlActionType::Preset => "Preset Unit File",
            ControlActionType::Reenable => "Reenable Unit File",
            ControlActionType::DisableUnitFiles => "Disable Unit File",
        }
    }

    pub fn title(&self) -> String {
        match self {
            ControlActionType::EnableUnitFiles => pgettext("action unit file", "Enable Unit File"),
            ControlActionType::MaskUnit => pgettext("action unit file", "Mask Unit"),
            ControlActionType::Preset => pgettext("action unit file", "Preset Unit File"),
            ControlActionType::Reenable => pgettext("action unit file", "Reenable Unit File"),
            ControlActionType::DisableUnitFiles => {
                pgettext("action unit file", "Disable Unit File")
            }
        }
    }

    pub fn first_group_title(&self) -> String {
        match self {
            ControlActionType::EnableUnitFiles => pgettext("action unit file", "Enable"),
            ControlActionType::DisableUnitFiles => pgettext("action unit file", "Disable"),
            ControlActionType::MaskUnit => pgettext("action unit file", "Mask"),
            _ => String::new(),
        }
    }

    pub fn after_group_title(&self) -> String {
        match self {
            ControlActionType::EnableUnitFiles => pgettext("action unit file", "Start"),
            ControlActionType::DisableUnitFiles => pgettext("action unit file", "Stop"),
            ControlActionType::MaskUnit => pgettext("action unit file", "Stop"),
            _ => String::new(),
        }
    }

    pub fn dialog_subtitle(&self) -> bool {
        !matches!(self, ControlActionType::EnableUnitFiles)
    }

    fn unit_file_entry_visible(&self) -> bool {
        matches!(self, ControlActionType::EnableUnitFiles)
    }

    fn dbus_level_combo_visible(&self) -> bool {
        matches!(self, ControlActionType::EnableUnitFiles)
    }

    fn portable_switch_visible(&self) -> bool {
        matches!(self, ControlActionType::EnableUnitFiles)
    }

    fn after_action_group_visible(&self) -> bool {
        matches!(
            self,
            ControlActionType::EnableUnitFiles
                | ControlActionType::MaskUnit
                | ControlActionType::DisableUnitFiles
        )
    }

    fn send_action_label(&self) -> String {
        match self {
            ControlActionType::EnableUnitFiles => pgettext("action unit file", "Enable"),
            ControlActionType::MaskUnit => pgettext("action unit file", "Mask"),
            ControlActionType::Preset => pgettext("action unit file", "Preset"),
            ControlActionType::DisableUnitFiles => pgettext("action unit file", "Enable"),
            ControlActionType::Reenable => pgettext("action unit file", "Enable"),
        }
    }

    fn run_stop_now(&self) -> (String, String) {
        match self {
            ControlActionType::EnableUnitFiles => (
                pgettext("action unit file", "Run now"),
                pgettext("action unit file", "Start Unit just after being enabled"),
            ),
            ControlActionType::MaskUnit | ControlActionType::DisableUnitFiles => (
                pgettext("action unit file", "Stop now"),
                pgettext(
                    "action unit file",
                    "Ensure that the unit will also be stopped",
                ),
            ),
            _ => (String::new(), String::new()),
        }
    }

    fn method_name(&self) -> String {
        self.title()
    }
}
