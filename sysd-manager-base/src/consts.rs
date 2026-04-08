use constcat::concat;

pub const APP_ID: &str = "io.github.plrigaux.sysd-manager";

pub const PROXY_SERVICE: &str = "sysd-manager-proxy";

pub const PROXY_SERVICE_DEV: &str = "sysd-manager-proxy-dev";

pub const DBUS_NAME: &str = "io.github.plrigaux.SysDManager";
pub const DBUS_NAME_DEV: &str = concat!(DBUS_NAME, "Dev");

#[cfg(feature = "flatpak")]
pub const DBUS_NAME_FLATPAK: &str = concat!(DBUS_NAME, "Flatpak");
#[cfg(feature = "appimage")]
pub const DBUS_NAME_APPIMAGE: &str = concat!(DBUS_NAME, "AppImage");
pub const DBUS_INTERFACE: &str = DBUS_NAME;
pub const DBUS_DESTINATION: &str = DBUS_NAME;
pub const DBUS_DESTINATION_DEV: &str = DBUS_NAME_DEV;
pub const DBUS_PATH: &str = "/io/github/plrigaux/SysDManager";
pub const DBUS_PATH_DEV: &str = concat!(DBUS_PATH, "Dev");

pub const MIN_HEART_BEAT_ELAPSE: u64 = 500;
pub const MAX_HEART_BEAT_ELAPSE: u64 = 300_000;
pub const FAVORITE_ICON_FILLED: &str = "bookmark-filled-symbolic";
pub const FAVORITE_ICON_OUTLINE: &str = "bookmark-outline-symbolic";
