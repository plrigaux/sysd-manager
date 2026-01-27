use base::proxy::{DisEnAbleUnitFiles, DisEnAbleUnitFilesResponse};
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

    fn enable_unit_files_with_flags(
        &self,
        files: &[&str],
        flags: u64,
    ) -> zbus::fdo::Result<DisEnAbleUnitFilesResponse>;

    #[zbus(allow_interactive_auth)]
    fn disable_unit_files_with_flags_and_install_info(
        &self,
        files: &[&str],
        flags: u64,
    ) -> zbus::fdo::Result<DisEnAbleUnitFilesResponse>;
}
