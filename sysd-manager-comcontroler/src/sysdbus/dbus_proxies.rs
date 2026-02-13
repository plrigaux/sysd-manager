use std::{collections::HashMap, sync::LazyLock};

use base::{
    enums::UnitDBusLevel,
    proxy::{DisEnAbleUnitFiles, DisEnAbleUnitFilesResponse, QueuedJobs},
};
use log::error;
use tokio::sync::OnceCell;
use zbus::proxy;
use zvariant::{OwnedObjectPath, OwnedValue};

use crate::{
    data::ListedLoadedUnit,
    errors::SystemdErrors,
    sysdbus::{ListedUnitFile, get_blocking_connection, get_connection},
};

#[proxy(
    interface = "org.freedesktop.DBus.Properties",
    default_service = "org.freedesktop.systemd1"
)]
pub(crate) trait ZProperties {
    fn get(&self, interface: &str, property_name: &str) -> Result<OwnedValue, zbus::Error>;

    fn get_all(&self, interface: &str) -> Result<HashMap<String, OwnedValue>, zbus::Error>;
}

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

    #[zbus(allow_interactive_auth)]
    fn reload(&self) -> zbus::fdo::Result<()>;

    fn clean_unit(&self, unit_name: &str, what: &[&str]) -> zbus::Result<()>;
    fn freeze_unit(&self, unit_name: &str) -> zbus::fdo::Result<()>;
    fn thaw_unit(&self, unit_name: &str) -> zbus::fdo::Result<()>;
    fn start_unit(&self, unit: &str, mode: &str) -> zbus::fdo::Result<OwnedObjectPath>;
    ///returns an array with all currently queued jobs.
    fn list_jobs(&self) -> zbus::fdo::Result<QueuedJobs>;

    fn create_drop_in(
        &mut self,
        runtime: bool,
        unit_name: &str,
        file_name: &str,
        content: &str,
    ) -> zbus::fdo::Result<()>;
    fn save_file(&mut self, file_name: &str, content: &str) -> zbus::fdo::Result<u64>;

    #[zbus(allow_interactive_auth)]
    fn revert_unit_files(&self, file_names: &[&str]) -> zbus::fdo::Result<Vec<DisEnAbleUnitFiles>>;

    #[zbus(allow_interactive_auth)]
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

    fn list_units(&self) -> Result<Vec<ListedLoadedUnit>, zbus::Error>;
    fn list_units_filtered(&self, states: &[&str]) -> Result<Vec<ListedLoadedUnit>, zbus::Error>;
    fn list_units_by_patterns(
        &self,
        states: &[&str],
        patterns: &[&str],
    ) -> Result<Vec<ListedLoadedUnit>, zbus::Error>;
    fn list_units_by_names(&self, names: &[&str]) -> Result<Vec<ListedLoadedUnit>, zbus::Error>;
    fn list_unit_files(&self) -> Result<Vec<ListedUnitFile>, zbus::Error>;
    fn list_unit_files_by_patterns(
        &self,
        states: &[&str],
        patterns: &[&str],
    ) -> Result<Vec<ListedUnitFile>, zbus::Error>;

    fn get_unit_file_state(&self, file: &str) -> Result<String, zbus::Error>;
}

static SYSTEM_MANAGER: OnceCell<Systemd1ManagerProxy> = OnceCell::const_new();
static SYSTEM_MANAGER_USER_SESSION: OnceCell<Systemd1ManagerProxy> = OnceCell::const_new();

fn systemd_manager_blocking_() -> Result<Systemd1ManagerProxyBlocking<'static>, SystemdErrors> {
    let conn = get_blocking_connection(base::enums::UnitDBusLevel::System)?;
    let proxy = Systemd1ManagerProxyBlocking::builder(&conn).build()?;
    Ok(proxy)
}

fn systemd_manager_session_blocking() -> Result<Systemd1ManagerProxyBlocking<'static>, SystemdErrors>
{
    let conn = get_blocking_connection(base::enums::UnitDBusLevel::UserSession)?;
    let proxy = Systemd1ManagerProxyBlocking::builder(&conn).build()?;
    Ok(proxy)
}

static SYSTEM_MANAGER_BLOCKING: LazyLock<Systemd1ManagerProxyBlocking> = LazyLock::new(|| {
    systemd_manager_blocking_()
        .inspect_err(|e| error!("{e:?}"))
        .unwrap()
});

static SYSTEM_MANAGER_SESSION_BLOCKING: LazyLock<Systemd1ManagerProxyBlocking> =
    LazyLock::new(|| {
        systemd_manager_session_blocking()
            .inspect_err(|e| error!("{e:?}"))
            .unwrap()
    });

#[cfg(feature = "flatpak")]
pub fn systemd_manager_blocking<'a>(level: UnitDBusLevel) -> &'a Systemd1ManagerProxyBlocking<'a> {
    match level {
        UnitDBusLevel::System | UnitDBusLevel::Both => systemd_manager(),
        UnitDBusLevel::UserSession => systemd_manager_session(),
    }
}

pub fn systemd_manager<'a>() -> &'a Systemd1ManagerProxyBlocking<'a> {
    (&*SYSTEM_MANAGER_BLOCKING) as _
}

pub fn systemd_manager_session<'a>() -> &'a Systemd1ManagerProxyBlocking<'a> {
    (&*SYSTEM_MANAGER_SESSION_BLOCKING) as _
}

pub async fn systemd_manager_async(
    level: UnitDBusLevel,
) -> Result<&'static Systemd1ManagerProxy<'static>, SystemdErrors> {
    match level {
        UnitDBusLevel::System | UnitDBusLevel::Both => {
            SYSTEM_MANAGER
                .get_or_try_init(async || -> Result<Systemd1ManagerProxy, SystemdErrors> {
                    let conn = get_connection(UnitDBusLevel::System).await?;
                    let proxy = Systemd1ManagerProxy::builder(&conn).build().await?;
                    Ok(proxy)
                })
                .await
        }
        UnitDBusLevel::UserSession => {
            SYSTEM_MANAGER_USER_SESSION
                .get_or_try_init(async || -> Result<Systemd1ManagerProxy, SystemdErrors> {
                    let conn = get_connection(UnitDBusLevel::UserSession).await?;
                    let proxy = Systemd1ManagerProxy::builder(&conn).build().await?;
                    Ok(proxy)
                })
                .await
        }
    }
}
