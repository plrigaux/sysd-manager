use zbus::proxy;

#[proxy(
    interface = "org.freedesktop.systemd1.Unit",
    default_service = "org.freedesktop.systemd1"
)]
pub(super) trait ZUnitInfo {
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
