use std::string::FromUtf8Error;

#[derive(Debug)]
#[allow(unused)]
pub enum SystemdErrors {
    Custom(String),
    IoError(std::io::Error),
    Utf8Error(FromUtf8Error),
    SystemCtlError(String),
    NoSuchUnit(Option<String>),
    Malformed,
    ZBusError(zbus::Error),
    ZBusFdoError(zbus::fdo::Error),
    ZVariantError(zvariant::Error),
    CmdNoFlatpakSpawn,
    CmdNoFreedesktopFlatpakPermission(Option<String>, Option<String>),
    JournalError(String),
    NoFilePathforUnit(String),
    //FlatpakAccess(ErrorKind),
    NotAuthorized,
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

impl From<zbus::Error> for SystemdErrors {
    fn from(error: zbus::Error) -> Self {
        if let zbus::Error::MethodError(owned_error_name, ref msg, ref _message) = error {
            let err_code = zvariant::Str::from(owned_error_name);

            if err_code.eq("org.freedesktop.systemd1.NoSuchUnit") {
                SystemdErrors::NoSuchUnit(msg.clone())
            } else {
                let msg = format!("MethodError Fail {:?}", err_code);
                SystemdErrors::Custom(msg)
            }
        } else {
            SystemdErrors::ZBusError(error)
        }
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
