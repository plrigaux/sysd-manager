use base::proxy::DisEnAbleUnitFiles;
use zbus::proxy;
use zvariant::OwnedObjectPath;

#[proxy(
    interface = "org.freedesktop.systemd1.Unit",
    default_service = "org.freedesktop.systemd1"
)]
pub(crate) trait ZUnitInfo {
    #[zbus(property)]
    fn id(&self) -> Result<String, zbus::Error>;

    #[zbus(property)]
    fn description(&self) -> Result<String, zbus::Error>;

    #[zbus(property)]
    fn load_state(&self) -> Result<String, zbus::Error>;

    #[zbus(property)]
    fn active_state(&self) -> Result<String, zbus::Error>;

    #[zbus(property)]
    fn sub_state(&self) -> Result<String, zbus::Error>;

    #[zbus(property)]
    fn following(&self) -> Result<String, zbus::Error>;

    #[zbus(property)]
    fn fragment_path(&self) -> Result<String, zbus::Error>;

    #[zbus(property)]
    fn unit_file_state(&self) -> Result<String, zbus::Error>;

    #[zbus(property)]
    fn unit_file_preset(&self) -> Result<String, zbus::Error>;

    #[zbus(property)]
    fn drop_in_paths(&self) -> Result<Vec<String>, zbus::Error>;
}

#[proxy(
    default_service = "org.freedesktop.systemd1",
    default_path = "/org/freedesktop/systemd1",
    interface = "org.freedesktop.systemd1.Manager"
)]
pub(crate) trait Systemd1Manager {
    // Defines signature for D-Bus signal named `JobNew`
    #[zbus(signal)]
    fn unit_new(&self, id: String, unit: OwnedObjectPath) -> zbus::Result<()>;

    #[zbus(signal)]
    fn unit_removed(&self, id: String, unit: OwnedObjectPath) -> zbus::Result<()>;

    // Defines signature for D-Bus signal named `JobNew`
    #[zbus(signal)]
    fn job_new(&self, id: u32, job: OwnedObjectPath, unit: String) -> zbus::Result<()>;

    #[zbus(signal)]
    fn job_removed(
        &self,
        id: u32,
        job: OwnedObjectPath,
        unit: String,
        result: String,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    fn startup_finished(
        &self,
        firmware: u64,
        loader: u64,
        kernel: u64,
        initrd: u64,
        userspace: u64,
        total: u64,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    fn unit_files_changed(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    fn reloading(&self, active: bool) -> zbus::Result<()>;

    fn clean_unit(&self, unit_name: &str, what: &[&str]) -> zbus::Result<()>;
    fn freeze_unit(&self, unit_name: &str) -> zbus::fdo::Result<()>;
    fn thaw_unit(&self, unit_name: &str) -> zbus::fdo::Result<()>;

    fn create_drop_in(
        &mut self,
        runtime: bool,
        unit_name: &str,
        file_name: &str,
        content: &str,
    ) -> zbus::fdo::Result<()>;
    fn save_file(&mut self, file_name: &str, content: &str) -> zbus::fdo::Result<u64>;

    fn revert_unit_files(
        &mut self,
        file_names: &[&str],
    ) -> zbus::fdo::Result<Vec<DisEnAbleUnitFiles>>;
}
