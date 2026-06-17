pub mod dropdown;
mod first_page;
mod imp;
mod launch_creator_page;
pub mod navigation_row;
pub mod suggestion;
mod timer_creator_page;
mod unit_file;
mod unit_file_creator_page;

mod service_creator_page;

use crate::{format2, widget::app_window::AppWindow};
use adw::subclass::prelude::ObjectSubclassIsExt;
use gettextrs::pgettext;
use gtk::glib::{self};
use std::{cell::Ref, collections::HashSet};
use systemd::errors::SystemdErrors;
use tracing::{error, warn};

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

    pub fn add_toast_message(
        &self,
        message: &str,
        markup: bool,
        action: Option<(&str, String, bool)>,
    ) {
        self.imp().add_toast_message(message, markup, action);
    }

    pub fn app_window(&self) -> Option<&AppWindow> {
        self.imp().app_window.get()
    }
}

pub const VALID_UNIT_NAME: &str = r"^[a-zA-Z0-9._:\-]+@?$";
pub const ACTION_CREATOR_UNIT_BUS: &str = "creator.create-unit-bus-selection";
pub const ACTION_CREATOR_UNIT_TYPE_SELECTION: &str = "creator.create-unit-type-selection";
pub const ACTION_CREATOR_NEXT: &str = "creator.next";
pub const ACTION_CREATOR_FILE: &str = "creator.file";
pub const ACTION_CREATOR_CREATE: &str = "creator.create";
pub const ACTION_CREATOR_PREVIOUS: &str = "creator.previous";
pub const PAGE_FIRST: &str = "first-page";
pub const PAGE_LAUNCH: &str = "launch-page";
pub const PAGE_TIMER: &str = "timer-page";
pub const PAGE_SERVICE: &str = "service-page";

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
        match value.get::<String>() {
            Some(s) => s.into(),
            None => {
                warn!("Unkown type None",);
                UnitCreateType::Service
            }
        }
    }
}

impl From<glib::GString> for UnitCreateType {
    fn from(value: glib::GString) -> Self {
        value.as_str().into()
    }
}

impl From<&str> for UnitCreateType {
    fn from(value: &str) -> Self {
        match value {
            "service" => UnitCreateType::Service,
            "timer" => UnitCreateType::Timer,
            "timer_service" => UnitCreateType::TimerService,
            other => {
                warn!("Unkown type {:?}", other);
                UnitCreateType::Service
            }
        }
    }
}

impl From<String> for UnitCreateType {
    fn from(value: String) -> Self {
        value.as_str().into()
    }
}

#[derive(Debug)]
pub enum SaveUnit {
    Created,
    CreateError(SystemdErrors),
}

#[derive(Debug, PartialEq)]
enum CreateUnitErr {
    NoErr,
    WrongChar,
    Limit255,
    FileExits,
    Empty,
    FileNotExits,
    NotFile,
    NotExecutable,
    Malformed,
    NotAbsolute,
    NotDir,
    NoPath,
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
            CreateUnitErr::NotExecutable => format!("{prefix} - Not an executable"),
            CreateUnitErr::Malformed => format!("{prefix} - Malformed"),
            CreateUnitErr::NoErr => prefix.to_owned(),
            CreateUnitErr::NotAbsolute => {
                format2!(pgettext("validator", "{} - Not absolute path"), prefix)
            }
            CreateUnitErr::NotDir => {
                format2!(pgettext("validator", "{} - Not a directory"), prefix)
            }
            CreateUnitErr::NoPath => {
                format2!(pgettext("validator", "{} - No path specified"), prefix)
            }
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default, glib::Enum)]
#[enum_type(name = "PageType")]
pub enum PageType {
    #[default]
    Start,
    Service,
    ServiceFile,
    Timer,
    TimerFile,
    Launch,
}

const SERVICE_FILE_PAGE: &str = "service-file-page";
const TIMER_FILE_PAGE: &str = "timer-file-page";

impl PageType {
    fn id(&self) -> &str {
        match self {
            PageType::Start => PAGE_FIRST,
            PageType::Service => PAGE_SERVICE,
            PageType::ServiceFile => SERVICE_FILE_PAGE,
            PageType::Timer => PAGE_TIMER,
            PageType::TimerFile => TIMER_FILE_PAGE,
            PageType::Launch => PAGE_LAUNCH,
        }
    }

    fn next(&self, creation_type: UnitCreateType) -> Option<&'static str> {
        match (self, creation_type) {
            (PageType::Start, UnitCreateType::Timer) => Some(PAGE_TIMER),
            (PageType::Start, _) => Some(PAGE_SERVICE),
            (PageType::Service, UnitCreateType::TimerService) => Some(PAGE_TIMER),
            (PageType::Service, _) => Some(PageType::Launch.id()),
            (PageType::ServiceFile, UnitCreateType::TimerService) => Some(PAGE_TIMER),
            (PageType::ServiceFile, _) => Some(PageType::Launch.id()),
            (PageType::Timer, _) => Some(PageType::Launch.id()),
            (PageType::TimerFile, _) => Some(PageType::Launch.id()),
            (PageType::Launch, _) => None,
        }
    }
}

impl From<Option<&str>> for PageType {
    fn from(value: Option<&str>) -> Self {
        match value {
            Some(PAGE_FIRST) => PageType::Start,
            Some(PAGE_TIMER) => PageType::Timer,
            Some(PAGE_SERVICE) => PageType::Service,
            Some(PAGE_LAUNCH) => PageType::Launch,
            Some(SERVICE_FILE_PAGE) => PageType::ServiceFile,
            Some(TIMER_FILE_PAGE) => PageType::TimerFile,
            Some(tag) => {
                warn!("Unkown TAG {tag}");
                PageType::Launch
            }
            None => {
                error!("Missing Tag");
                PageType::Start
            }
        }
    }
}
