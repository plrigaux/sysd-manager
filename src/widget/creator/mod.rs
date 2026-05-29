pub mod dropdown;
mod first_page;
mod imp;
mod launch_creator_page;
pub mod navigation_row;
mod service_creator_page;
mod timer_creator_page;
mod unit_file;
mod unit_file_creator_page;
use crate::widget::app_window::AppWindow;
use adw::subclass::prelude::ObjectSubclassIsExt;
use base::enums::UnitDBusLevel;
use gettextrs::pgettext;
use gtk::glib::{self};
use std::{cell::Ref, collections::HashSet};
use tracing::warn;

glib::wrapper! {

    pub struct UnitCreatorWindow(ObjectSubclass<imp::UnitCreatorWindowImp>)
    @extends adw::Window, gtk::Window, gtk::Widget,
    @implements gtk::Accessible,  gtk::Buildable,  gtk::ConstraintTarget,
    gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl UnitCreatorWindow {
    pub fn new(app_window: &AppWindow) -> Self {
        let obj: UnitCreatorWindow = glib::Object::new();
        let _ = obj.imp().app_window.set(app_window.clone());
        obj
    }

    pub fn action_group(&self) -> gio::SimpleActionGroup {
        self.imp().action_group.borrow().clone()
    }

    pub fn set_creation_unit_type(&self, unit_type: UnitCreateType) {
        self.imp().set_creation_unit_type(unit_type);
    }

    pub fn system_file_list(&self) -> Ref<'_, HashSet<String>> {
        self.imp().system_file_list.borrow()
    }

    pub fn session_file_list(&self) -> Ref<'_, HashSet<String>> {
        self.imp().session_file_list.borrow()
    }

    pub fn set_bus_level(&self, level: UnitDBusLevel) {
        self.imp().bus_level.set(level);
    }
}

pub const VALID_UNIT_NAME: &str = r"^[a-zA-Z0-9._:\-]+@?$";
pub const ACTION_CREATOR_UNIT_BUS: &str = "creator.unit_bus_selection";
pub const ACTION_CREATOR_UNIT_TYPE_SELECTION: &str = "creator.unit_type_selection";
pub const ACTION_CREATOR_NEXT: &str = "creator.next";
pub const ACTION_CREATOR_PREVIOUS: &str = "creator.previous";
#[derive(Debug, Copy, Clone, PartialEq, Eq, glib::Enum, Default, Hash)]
#[enum_type(name = "UnitCreateType")]
pub enum UnitCreateType {
    #[default]
    Service,
    Timer,
    TimerService,
}

impl UnitCreateType {
    pub fn max_sufix_len(&self) -> usize {
        match self {
            UnitCreateType::Service => ".service".len(),
            UnitCreateType::Timer => ".timer".len(),
            UnitCreateType::TimerService => ".service".len(),
        }
    }

    fn id(&self) -> &str {
        match self {
            UnitCreateType::Service => "service",
            UnitCreateType::Timer => "timer",
            UnitCreateType::TimerService => "timer_service",
        }
    }

    fn title(&self) -> String {
        match self {
            UnitCreateType::Service => pgettext("create", "Service"),
            UnitCreateType::Timer => pgettext("create", "Timer"),
            UnitCreateType::TimerService => pgettext("create", "Timer with Service"),
        }
    }
}

impl From<&glib::Variant> for UnitCreateType {
    fn from(value: &glib::Variant) -> Self {
        match value.get::<String>().as_deref() {
            Some("service") => UnitCreateType::Service,
            Some("timer") => UnitCreateType::Timer,
            Some("timer_service") => UnitCreateType::TimerService,
            other => {
                warn!("Unkown type {:?}", other);
                UnitCreateType::Service
            }
        }
    }
}

#[derive(Debug, PartialEq)]
enum CreateUnitErr {
    WrongChar,
    Limit255,
    FileExits,
    Empty,
    FileNotExits,
    NotFile,
    NotExecutable,
    NoErr,
    Malformed,
}

impl CreateUnitErr {
    fn title_err(&self, prefix: &str) -> String {
        match self {
            CreateUnitErr::WrongChar => format!("{prefix} - Wrong Char"),
            CreateUnitErr::Limit255 => format!("{prefix} - Unit File over 255 characters"),
            CreateUnitErr::FileExits => format!("{prefix} - Unit File already exists"),
            CreateUnitErr::Empty => format!("{prefix} -  Empty"),
            CreateUnitErr::FileNotExits => format!("{prefix} - File not exists"),
            CreateUnitErr::NotFile => format!("{prefix} - Not a File"),
            CreateUnitErr::NotExecutable => format!("{prefix} - Not Exec"),
            CreateUnitErr::Malformed => format!("{prefix} - Malformed"),
            CreateUnitErr::NoErr => prefix.to_owned(),
        }
    }
}
