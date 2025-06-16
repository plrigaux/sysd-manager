pub mod imp;

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
    Link,
}

impl ControlActionType {
    pub fn code(&self) -> &str {
        match self {
            ControlActionType::EnableUnitFiles => "Enable Unit File",
            ControlActionType::MaskUnit => "Mask Unit",
            ControlActionType::Preset => "Preset Unit File",
            ControlActionType::Reenable => "Reenable Unit File",
            ControlActionType::DisableUnitFiles => "Disable Unit File",
            ControlActionType::Link => "Link Unit File",
        }
    }

    pub fn title(&self) -> String {
        match self {
            //Dialog title
            ControlActionType::EnableUnitFiles => pgettext("action unit file", "Enable Unit File"),
            //Dialog title
            ControlActionType::MaskUnit => pgettext("action unit file", "Mask Unit"),
            //Dialog title
            ControlActionType::Preset => pgettext("action unit file", "Preset Unit File"),
            //Dialog title
            ControlActionType::Reenable => pgettext("action unit file", "Reenable Unit File"),

            ControlActionType::DisableUnitFiles => {
                //Dialog title
                pgettext("action unit file", "Disable Unit File")
            }
            //Dialog title
            ControlActionType::Link => pgettext("action unit file", "Link Unit File"),
        }
    }

    pub fn first_group_title(&self) -> String {
        match self {
            //preference group title
            ControlActionType::EnableUnitFiles => pgettext("action unit file", "Enable"),
            //preference group title
            ControlActionType::DisableUnitFiles => pgettext("action unit file", "Disable"),
            //preference group title
            ControlActionType::MaskUnit => pgettext("action unit file", "Mask"),
            _ => String::new(),
        }
    }

    pub fn after_group_title(&self) -> String {
        match self {
            //second preference group title
            ControlActionType::EnableUnitFiles => pgettext("action unit file", "Start"),
            //second preference group title
            ControlActionType::DisableUnitFiles | ControlActionType::MaskUnit => {
                pgettext("action unit file", "Stop")
            }

            _ => String::new(),
        }
    }

    pub fn dialog_subtitle(&self) -> bool {
        !matches!(
            self,
            ControlActionType::EnableUnitFiles | ControlActionType::Link
        )
    }

    fn unit_file_entry_visible(&self) -> bool {
        matches!(
            self,
            ControlActionType::EnableUnitFiles | ControlActionType::Link
        )
    }

    fn dbus_level_combo_visible(&self) -> bool {
        matches!(
            self,
            ControlActionType::EnableUnitFiles | ControlActionType::Link
        )
    }

    fn portable_switch_visible(&self) -> bool {
        matches!(
            self,
            ControlActionType::EnableUnitFiles | ControlActionType::DisableUnitFiles
        )
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
            //Button
            ControlActionType::EnableUnitFiles => pgettext("action unit file", "Enable"),
            //Button
            ControlActionType::MaskUnit => pgettext("action unit file", "Mask"),
            //Button
            ControlActionType::Preset => pgettext("action unit file", "Preset"),
            //Button
            ControlActionType::DisableUnitFiles => pgettext("action unit file", "Disable"),
            //Button
            ControlActionType::Reenable => pgettext("action unit file", "Reenable"),
            //Button
            ControlActionType::Link => pgettext("action unit file", "Link"),
        }
    }

    fn run_stop_now(&self) -> (String, String) {
        match self {
            ControlActionType::EnableUnitFiles => (
                //after action title
                pgettext("action unit file", "Run now"),
                //after action subtitle
                pgettext("action unit file", "Start Unit just after being enabled"),
            ),
            ControlActionType::MaskUnit | ControlActionType::DisableUnitFiles => (
                //after action title
                pgettext("action unit file", "Stop now"),
                //after action subtitle
                pgettext(
                    "action unit file",
                    "Ensure that the unit will also be stopped",
                ),
            ),
            _ => (String::new(), String::new()),
        }
    }

    fn run_stop_now_mode(&self) -> (String, String) {
        match self {
            ControlActionType::EnableUnitFiles => (
                //starts mode title
                pgettext("action unit file", "Run mode"),
                //starts mode subtitle
                pgettext("action unit file", "Starting mode options"),
            ),
            ControlActionType::MaskUnit | ControlActionType::DisableUnitFiles => (
                //starts mode title
                pgettext("action unit file", "Stop mode"),
                //starts mode subtitle
                pgettext("action unit file", "Stoping mode options"),
            ),
            _ => (String::new(), String::new()),
        }
    }

    fn method_name(&self) -> String {
        self.title()
    }
}
