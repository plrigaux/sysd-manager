use constcat::concat;

pub const APP_ID: &str = "io.github.plrigaux.sysd-manager";

#[cfg(feature = "flatpak")]
pub const PROXY_SERVICE: &str = "sysd-manager-proxy-flatpak";

#[cfg(not(feature = "flatpak"))]
pub const PROXY_SERVICE: &str = "sysd-manager-proxy";

pub const PROXY_SERVICE_DEV: &str = "sysd-manager-proxy-dev";

pub const DBUS_NAME: &str = "io.github.plrigaux.SysDManager";
pub const DBUS_NAME_DEV: &str = concat!(DBUS_NAME, "Dev");
pub const DBUS_NAME_FLATPAK: &str = concat!(DBUS_NAME, "Flatpak");
pub const DBUS_INTERFACE: &str = DBUS_NAME;
pub const DBUS_DESTINATION: &str = DBUS_NAME;
pub const DBUS_DESTINATION_DEV: &str = DBUS_NAME_DEV;
pub const DBUS_PATH: &str = "/io/github/plrigaux/SysDManager";
pub const DBUS_PATH_DEV: &str = concat!(DBUS_PATH, "Dev");
