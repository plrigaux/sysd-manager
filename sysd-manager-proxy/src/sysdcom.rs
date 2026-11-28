use base::proxy::{DisEnAbleUnitFiles, DisEnAbleUnitFilesResponse};
use zbus::{OwnedObjectPath, fdo, proxy};

#[proxy(
    interface = "org.freedesktop.systemd1.Manager",
    default_service = "org.freedesktop.systemd1",
    default_path = "/org/freedesktop/systemd1"
)]
pub trait SysDManagerComLink {
    fn start_unit(&self, unit_name: &str, mode: &str) -> fdo::Result<OwnedObjectPath>;
    fn stop_unit(&self, unit_name: &str, mode: &str) -> fdo::Result<OwnedObjectPath>;
    fn restart_unit(&self, unit_name: &str, mode: &str) -> fdo::Result<OwnedObjectPath>;
    fn reload_unit(&self, unit_name: &str, mode: &str) -> fdo::Result<OwnedObjectPath>;
    fn clean_unit(&self, unit_name: &str, what: &[&str]) -> fdo::Result<()>;
    fn freeze_unit(&self, unit_name: &str) -> fdo::Result<()>;
    fn thaw_unit(&self, unit_name: &str) -> fdo::Result<()>;
    fn revert_unit_files(&self, unit_names: &Vec<String>) -> fdo::Result<Vec<DisEnAbleUnitFiles>>;
    fn reload(&self) -> fdo::Result<()>;

    fn enable_unit_files_with_flags(
        &self,
        files: &[&str],
        flags: u64,
    ) -> fdo::Result<DisEnAbleUnitFilesResponse>;

    #[zbus(allow_interactive_auth)]
    fn disable_unit_files_with_flags_and_install_info(
        &self,
        files: &[&str],
        flags: u64,
    ) -> fdo::Result<DisEnAbleUnitFilesResponse>;

    fn reset_failed_unit(&self, unit_name: &str) -> fdo::Result<()>;
    fn reset_failed(&self) -> fdo::Result<()>;

    #[zbus(property)]
    fn set_log_level(&self, log_level: &str) -> fdo::Result<()>;

    #[zbus(property)]
    fn set_log_target(&self, log_target: &str) -> fdo::Result<()>;
}
