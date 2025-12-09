use base::proxy::DisEnAbleUnitFiles;
use zbus::proxy;

#[proxy(
    interface = "org.freedesktop.systemd1.Manager",
    default_service = "org.freedesktop.systemd1",
    default_path = "/org/freedesktop/systemd1"
)]
pub trait SysDManagerComLink {
    fn clean_unit(&self, unit_name: &str, what: &[&str]) -> zbus::fdo::Result<()>;
    fn freeze_unit(&self, unit_name: &str) -> zbus::fdo::Result<()>;
    fn thaw_unit(&self, unit_name: &str) -> zbus::fdo::Result<()>;
    fn revert_unit_files(
        &self,
        unit_names: &Vec<String>,
    ) -> zbus::fdo::Result<Vec<DisEnAbleUnitFiles>>;
    fn reload(&self) -> zbus::fdo::Result<()>;
}
