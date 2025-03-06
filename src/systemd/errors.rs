use std::{
    fmt::{self, Display, Formatter},
    string::FromUtf8Error,
};

#[derive(Debug)]
#[allow(unused)]
pub enum SystemdErrors {
    Custom(String),
    IoError(std::io::Error),
    Utf8Error(FromUtf8Error),
    SystemCtlError(String),
    Malformed(String, String),
    ZMethodError(String, String, String),
    CmdNoFlatpakSpawn,
    CmdNoFreedesktopFlatpakPermission(Option<String>, Option<String>),
    JournalError(String),
    NoFilePathforUnit(String),
    //FlatpakAccess(ErrorKind),
    NotAuthorized,
    Tokio,
    ZBusError(zbus::Error),
    ZAccessDenied(String, String),
    ZNoSuchUnit(String, String),
    ZVariantError(zvariant::Error),
    ZBusFdoError(zbus::fdo::Error),
}

impl SystemdErrors {
    pub fn gui_description(&self) -> Option<String> {
        match self {
            SystemdErrors::CmdNoFlatpakSpawn => {
                let value = "The program <b>flatpack-spawn</b> is needed if you use the application from Flatpack.\nPlease install it to enable all features.";
                Some(value.to_owned())
            }
            SystemdErrors::CmdNoFreedesktopFlatpakPermission(_cmdl, _file_path) => {
                let msg = "Requires permission to talk to <b>org.freedesktop.Flatpak</b> D-Bus interface when the program is a Flatpak.";
                Some(msg.to_owned())
            }
            _ => None,
        }
    }

    pub fn human_error_type(&self) -> String {
        match self {
            SystemdErrors::ZAccessDenied(_, _) => "Access Denied".to_owned(),
            _ => self.to_string(),
        }
    }
}

impl Display for SystemdErrors {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
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
                        SystemdErrors::ZAccessDenied(method.to_owned(), message)
                    }
                    "org.freedesktop.systemd1.NoSuchUnit" => {
                        SystemdErrors::ZNoSuchUnit(method.to_owned(), message)
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
        let msg = format!("{}", error);
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
