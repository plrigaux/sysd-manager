mod creator_page_mount;
mod creator_page_service;
mod creator_page_timer;
//pub mod dropdown;
mod first_page;
mod imp;
mod launch_creator_page;
pub mod mydropdown;
pub mod navigation_row;
pub mod suggestion;
mod unit_file;
mod unit_file_creator_page;

use crate::{
    consts::{ERROR_CSS, WARNING_CSS},
    format2,
    widget::app_window::AppWindow,
};
use adw::{prelude::PreferencesRowExt, subclass::prelude::ObjectSubclassIsExt};
use gettextrs::pgettext;
use glib::object::IsA;
use gtk::glib::{self};
use std::{
    cell::Ref,
    collections::HashSet,
    path::{Path, PathBuf},
};
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

    pub fn set_creation_type(&self, unit_type: UnitCreateType) {
        self.imp().set_creation_unit_type(unit_type);
    }

    pub fn creation_type(&self) -> UnitCreateType {
        self.imp().creation_type.get()
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

    fn unit_name(&self, create_type: UnitCreateType) -> Option<String> {
        self.imp().unit_name(create_type)
    }

    pub fn file_path(&self) -> Option<PathBuf> {
        self.imp().file_path(None)
    }

    pub(crate) fn update_unit_prefix(&self, escaped_prefix: String) -> CreateUnitErr {
        self.imp().update_unit_prefix(escaped_prefix)
    }
}

pub const VALID_UNIT_NAME: &str = r"^\s*[a-zA-Z0-9._:\-\\]+@?\s*$";
pub const ACTION_CREATOR_UNIT_BUS: &str = "creator.create-unit-bus-selection";
pub const ACTION_CREATOR_UNIT_TYPE_SELECTION: &str = "creator.create-unit-type-selection";
pub const ACTION_CREATOR_NEXT: &str = "creator.next";
pub const ACTION_CREATOR_FILE: &str = "creator.file";
pub const ACTION_CREATOR_CREATE: &str = "creator.create";
pub const ACTION_CREATOR_PREVIOUS: &str = "creator.previous";
pub const ACTION_CREATOR_AVANCED_MODE: &str = "creator.creator-advanced-mode";
pub const PAGE_FIRST: &str = "first-page";
pub const PAGE_LAUNCH: &str = "launch-page";
pub const PAGE_TIMER: &str = "timer-page";
pub const PAGE_SERVICE: &str = "service-page";
pub const PAGE_MOUNT: &str = "mount-page";

#[derive(Debug, Copy, Clone, PartialEq, Eq, glib::Enum, Default, Hash)]
#[enum_type(name = "UnitCreateType")]
pub enum UnitCreateType {
    #[default]
    Service,
    Timer,
    TimerService,
    Mount,
}

impl UnitCreateType {
    pub fn max_suffix_len(&self) -> usize {
        self.dot_suffix().len()
    }

    pub fn dot_suffix(&self) -> &str {
        match self {
            UnitCreateType::Service | UnitCreateType::TimerService => ".service",
            UnitCreateType::Timer => ".timer",
            UnitCreateType::Mount => ".mount",
        }
    }

    pub fn suffix(&self) -> &str {
        &self.dot_suffix()[1..]
    }

    pub fn full_name(&self, prefix: &str) -> String {
        let dot_suffix = self.dot_suffix();
        let mut s = String::with_capacity(prefix.len() + dot_suffix.len());
        s.push_str(prefix);
        s.push_str(dot_suffix);
        s
    }

    fn title(&self) -> String {
        match self {
            //Create Unit title
            UnitCreateType::Service => pgettext("create", "Service"),
            //Create Unit title
            UnitCreateType::Timer => pgettext("create", "Timer"),
            //Create Unit title
            UnitCreateType::TimerService => pgettext("create", "Timer with Service"),
            //Create Unit title
            UnitCreateType::Mount => pgettext("create", "Mount"),
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
            "mount" => UnitCreateType::Mount,

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
pub(crate) enum CreateUnitErr {
    NoErr,
    WrongChar,
    FileExits,
    Empty,
    FileNotExits,
    NotFile,
    NotExecutable,
    Malformed,
    NotAbsolute,
    NotDir,
    NoPath,
    TooShort(usize),
    TooLong(usize),
    Mandatory,
    Unknown,
}

impl CreateUnitErr {
    fn title_err(&self, prefix: &str) -> String {
        match self {
            CreateUnitErr::WrongChar => format!("{prefix} - Invalid character"),
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
            CreateUnitErr::TooShort(limit) => format2!(
                pgettext(
                    "validator",
                    "{} - Too short, under the limit of {} characters"
                ),
                prefix,
                limit
            ),
            CreateUnitErr::TooLong(limit) => format2!(
                pgettext(
                    "validator",
                    "{} - Too long, over the limit of {} characters"
                ),
                prefix,
                limit
            ),
            CreateUnitErr::Mandatory => format2!(pgettext("validator", "{} - Mandatory"), prefix),
            CreateUnitErr::Unknown => format2!(pgettext("validator", "{} - Unknown"), prefix),
        }
    }

    fn apply_validation_result(
        entry: &(impl IsA<adw::PreferencesRow> + gtk::prelude::WidgetExt),
        name_err: &CreateUnitErr,
        prefix: &str,
    ) {
        Self::apply_validation_result_error(entry, name_err, prefix, false);
    }

    fn apply_validation_result_error(
        entry: &(impl IsA<adw::PreferencesRow> + gtk::prelude::WidgetExt),
        name_err: &CreateUnitErr,
        prefix: &str,
        error: bool,
    ) -> bool {
        entry.set_title(&name_err.title_err(prefix));
        match name_err {
            CreateUnitErr::NoErr => {
                entry.remove_css_class(WARNING_CSS);
                entry.remove_css_class(ERROR_CSS);
                true
            }
            _ => {
                if error {
                    entry.add_css_class(ERROR_CSS);
                } else {
                    entry.add_css_class(WARNING_CSS);
                }
                false
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
    Mount,
    MountFile,
}

const FILE_PAGE_SERVICE: &str = "service-file-page";
const FILE_PAGE_MOUNT: &str = "mount-file-page";
const FILE_PAGE_TIMER: &str = "timer-file-page";

impl PageType {
    fn id(&self) -> &str {
        match self {
            PageType::Start => PAGE_FIRST,
            PageType::Service => PAGE_SERVICE,
            PageType::ServiceFile => FILE_PAGE_SERVICE,
            PageType::Timer => PAGE_TIMER,
            PageType::TimerFile => FILE_PAGE_TIMER,
            PageType::Launch => PAGE_LAUNCH,
            PageType::Mount => PAGE_MOUNT,
            PageType::MountFile => FILE_PAGE_MOUNT,
        }
    }

    fn next(&self, creation_type: UnitCreateType) -> Option<&'static str> {
        match (self, creation_type) {
            (PageType::Start, UnitCreateType::Timer) => Some(PAGE_TIMER),
            (PageType::Start, UnitCreateType::Mount) => Some(PAGE_MOUNT),
            (PageType::Start, _) => Some(PAGE_SERVICE),
            (PageType::Service, UnitCreateType::TimerService) => Some(PAGE_TIMER),
            (PageType::Service, _) => Some(PageType::Launch.id()),
            (PageType::ServiceFile, UnitCreateType::TimerService) => Some(PAGE_TIMER),
            (PageType::ServiceFile, _) => Some(PageType::Launch.id()),
            (PageType::Timer, _) => Some(PageType::Launch.id()),
            (PageType::TimerFile, _) => Some(PageType::Launch.id()),
            (PageType::Mount, _) => Some(PageType::Launch.id()),
            (PageType::MountFile, _) => Some(PageType::Launch.id()),
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
            Some(PAGE_MOUNT) => PageType::Mount,
            Some(FILE_PAGE_SERVICE) => PageType::ServiceFile,
            Some(FILE_PAGE_TIMER) => PageType::TimerFile,
            Some(FILE_PAGE_MOUNT) => PageType::MountFile,
            Some(PAGE_LAUNCH) => PageType::Launch,
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

pub fn set_initial_folder(file_dialog: &gtk::FileDialog) {
    if let Ok(home) = std::env::var("HOME") {
        let path = Path::new(&home);
        let dir = gio::File::for_path(path);
        file_dialog.set_initial_folder(Some(&dir));
    }
}

fn get_file_path(text: &str) -> Result<&str, CreateUnitErr> {
    let text = text.trim_start();
    let mut begin = 0;
    let mut end = text.len();
    let mut in_quotes = false;

    for (idx, char) in text.char_indices() {
        if char.is_whitespace() && !in_quotes {
            end = idx;
            break;
        } else if char == '"' {
            if idx == 0 {
                in_quotes = true;
                begin = 1;
            } else {
                end = idx;
                in_quotes = false;
                break;
            }
        }
    }
    if in_quotes {
        return Err(CreateUnitErr::Malformed);
    }
    Ok(&text[begin..end])
}

#[cfg(test)]
mod tests {
    use test_base::init_logs;

    use super::*;

    #[test]
    fn test_get_file() {
        init_logs();
        assert_eq!(get_file_path("text"), Ok("text"));
        assert_eq!(get_file_path("  text"), Ok("text"));
        assert_eq!(get_file_path("  text   "), Ok("text"));
        assert_eq!(get_file_path("  text -f  "), Ok("text"));
        assert_eq!(get_file_path(r#""text asdf" xxx"#), Ok("text asdf"));
        assert_eq!(get_file_path("\"\"text"), Ok(""));

        assert_eq!(
            get_file_path("/home/plr/bin/AppDir/etc"),
            Ok("/home/plr/bin/AppDir/etc")
        );
    }
}
