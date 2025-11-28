use zbus::proxy;

#[proxy(
    interface = "org.freedesktop.systemd1.Manager",
    default_service = "org.freedesktop.systemd1",
    default_path = "/org/freedesktop/systemd1"
)]
pub trait SysDManagerComLink {
    fn clean_unit(&self, unit_name: &str, what: &[&str]) -> zbus::Result<()>;
}
