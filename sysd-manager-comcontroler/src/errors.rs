use std::{
    ffi::OsString,
    fmt::{self, Display, Formatter},
    process::Command,
    string::FromUtf8Error,
};

use gettextrs::pgettext;

#[derive(Debug)]
#[allow(unused)]
pub enum SystemdErrors {
    Command(
        OsString,
        Vec<OsString>,
        Vec<(OsString, Option<OsString>)>,
        std::io::Error,
    ),
    Custom(String),
    IoError(std::io::Error),
    Utf8Error(FromUtf8Error),
    Fmt(std::fmt::Error),
    ZMethodError(String, String, String),
    CmdNoFlatpakSpawn,
    CmdNoFreedesktopFlatpakPermission(Option<String>, Option<String>),
    JournalError(String),
    NoFilePathforUnit(String),
    Malformed(String, String),
    NotAuthorized,
    NoUnit,
    SystemCtlError(String),
    Tokio,
    ZBusError(zbus::Error),
    ZAccessDenied(String, String),
    ZNoSuchUnit(String, String),
    ZNoSuchUnitProxy(String, String),
    ZJobTypeNotApplicable(String, String),
    ZUnitMasked(String, String),
    ZVariantError(zvariant::Error),
    ZBusFdoError(zbus::fdo::Error),
    ZXml(zbus_xml::Error),
}

impl SystemdErrors {
    pub fn gui_description(&self) -> Option<String> {
        match self {
            SystemdErrors::CmdNoFlatpakSpawn => {
                //error message flatpak permission
                Some(pgettext(
                    "error",
                    "The program <b>flatpack-spawn</b> is needed if you use the application from Flatpack.\nPlease install it to enable all features.",
                ))
            }
            SystemdErrors::CmdNoFreedesktopFlatpakPermission(_cmdl, _file_path) => {
                //error message flatpak permission
                Some(pgettext(
                    "error",
                    "It requires permission to talk to <b>org.freedesktop.Flatpak</b> D-Bus interface when the program is a Flatpak.",
                ))
            }
            _ => None,
        }
    }

    pub fn human_error_type(&self) -> String {
        match self {
            SystemdErrors::ZAccessDenied(_, detail) => detail.clone(),
            SystemdErrors::ZJobTypeNotApplicable(_, detail) => detail.clone(),
            SystemdErrors::ZNoSuchUnit(_, detail) => detail.clone(),
            SystemdErrors::ZNoSuchUnitProxy(_, detail) => detail.clone(),
            SystemdErrors::ZUnitMasked(_, detail) => detail.clone(),
            _ => self.to_string(),
        }
    }

    pub(crate) fn create_command_error(command: &Command, error: std::io::Error) -> Self {
        let program = command.get_program().to_os_string();
        let envs: Vec<(OsString, Option<OsString>)> = command
            .get_envs()
            .map(|(k, v)| (k.to_os_string(), v.map(|s| s.to_os_string())))
            .collect();
        let arg: Vec<OsString> = command.get_args().map(|s| s.to_os_string()).collect();

        SystemdErrors::Command(program, arg, envs, error)
    }
}

impl Display for SystemdErrors {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl From<std::io::Error> for SystemdErrors {
    fn from(error: std::io::Error) -> Self {
        SystemdErrors::IoError(error)
    }
}

impl From<FromUtf8Error> for SystemdErrors {
    fn from(error: FromUtf8Error) -> Self {
        SystemdErrors::Utf8Error(error)
    }
}

impl From<(zbus::Error, &str)> for SystemdErrors {
    fn from(value: (zbus::Error, &str)) -> Self {
        let (zb_error, method) = value;

        match zb_error {
            zbus::Error::MethodError(owned_error_name, ref msg, _message) => {
                let err_code = zvariant::Str::from(owned_error_name);

                let err_code = err_code.as_str();
                let message = msg.clone().unwrap_or_default();

                match err_code {
                    "org.freedesktop.DBus.Error.AccessDenied" => {
                        let method = if method.is_empty() {
                            "AccessDenied"
                        } else {
                            method
                        };
                        SystemdErrors::ZAccessDenied(method.to_owned(), message)
                    }
                    "org.freedesktop.systemd1.NoSuchUnit" => {
                        let method = if method.is_empty() {
                            "NoSuchUnit"
                        } else {
                            method
                        };
                        SystemdErrors::ZNoSuchUnit(method.to_owned(), message)
                    }
                    "org.freedesktop.DBus.Error.InvalidArgs" => {
                        let method = if method.is_empty() {
                            "InvalidArgs"
                        } else {
                            method
                        };
                        SystemdErrors::ZNoSuchUnitProxy(method.to_owned(), message)
                    }
                    "org.freedesktop.systemd1.JobTypeNotApplicable" => {
                        let method = if method.is_empty() {
                            "JobTypeNotApplicable"
                        } else {
                            method
                        };
                        SystemdErrors::ZJobTypeNotApplicable(method.to_owned(), message)
                    }
                    "org.freedesktop.systemd1.UnitMasked" => {
                        let method = if method.is_empty() {
                            "UnitMasked"
                        } else {
                            method
                        };
                        SystemdErrors::ZUnitMasked(method.to_owned(), message)
                    }
                    _ => {
                        SystemdErrors::ZMethodError(method.to_owned(), err_code.to_owned(), message)
                    }
                }
            }

            _ => SystemdErrors::ZBusError(zb_error),
        }
    }
}

impl From<zbus::Error> for SystemdErrors {
    fn from(error: zbus::Error) -> Self {
        //log::info!("TS {:?}", error);
        SystemdErrors::from((error, ""))
    }
}

impl From<zbus::fdo::Error> for SystemdErrors {
    fn from(error: zbus::fdo::Error) -> Self {
        SystemdErrors::ZBusFdoError(error)
    }
}

impl From<Box<dyn std::error::Error>> for SystemdErrors {
    fn from(error: Box<dyn std::error::Error>) -> Self {
        let msg = format!("{error}");
        SystemdErrors::JournalError(msg)
    }
}

impl From<zvariant::Error> for SystemdErrors {
    fn from(value: zvariant::Error) -> Self {
        SystemdErrors::ZVariantError(value)
    }
}

impl From<tokio::task::JoinError> for SystemdErrors {
    fn from(_value: tokio::task::JoinError) -> Self {
        SystemdErrors::Tokio
    }
}

impl From<zbus_xml::Error> for SystemdErrors {
    fn from(value: zbus_xml::Error) -> Self {
        SystemdErrors::ZXml(value)
    }
}

impl From<std::fmt::Error> for SystemdErrors {
    fn from(value: std::fmt::Error) -> Self {
        SystemdErrors::Fmt(value)
    }
}

impl From<String> for SystemdErrors {
    fn from(value: String) -> Self {
        SystemdErrors::Custom(value)
    }
}

impl From<&str> for SystemdErrors {
    fn from(value: &str) -> Self {
        value.to_owned().into()
    }
}
