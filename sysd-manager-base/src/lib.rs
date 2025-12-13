pub mod consts;
pub mod enums;
pub mod file;
pub mod proxy;

use std::env;

use tracing::info;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RunMode {
    Normal,
    Development,
    Both,
}

impl RunMode {
    pub fn from_flags(dev: bool, normal: bool) -> Self {
        #[cfg(not(feature = "flatpak"))]
        let cargo_in_use = env::var("CARGO");

        #[cfg(feature = "flatpak")]
        let cargo_in_use: Result<String, env::VarError> = Ok("flatpack".to_string());

        match (dev, normal, cargo_in_use) {
            (true, true, _) => RunMode::Both,
            (true, false, _) => RunMode::Development,
            (false, true, _) => RunMode::Normal,
            (false, false, Ok(_)) => {
                info!("The program is being run by cargo. --> Assume Development mode.");
                RunMode::Development
            }
            (false, false, Err(_)) => RunMode::Normal,
        }
    }

    pub fn proxy_service_name(&self) -> String {
        format!("{}.service", self.proxy_service_id())
    }

    pub fn proxy_service_id(&self) -> &str {
        #[cfg(feature = "flatpak")]
        let unit_name = crate::consts::PROXY_SERVICE;

        #[cfg(not(feature = "flatpak"))]
        let unit_name = if *self == RunMode::Development {
            crate::consts::PROXY_SERVICE_DEV
        } else {
            crate::consts::PROXY_SERVICE
        };

        unit_name
    }

    pub fn bus_name(&self) -> &str {
        #[cfg(feature = "flatpak")]
        let bus_name = crate::consts::DBUS_NAME_FLATPAK;

        #[cfg(not(feature = "flatpak"))]
        let bus_name = if *self == RunMode::Development {
            crate::consts::DBUS_NAME_DEV
        } else {
            crate::consts::DBUS_NAME
        };

        bus_name
    }
}
