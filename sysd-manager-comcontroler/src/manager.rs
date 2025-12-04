use zbus::proxy;
use zvariant::OwnedObjectPath;
#[proxy(
    interface = "org.freedesktop.systemd1.Manager",
    default_service = "io.github.plrigaux.SysDManager",
    default_path = "/org/freedesktop/systemd1"
)]
pub trait Manager {
    fn start_unit(&self, unit: &str, mode: &str) -> zbus::fdo::Result<OwnedObjectPath>;
}
