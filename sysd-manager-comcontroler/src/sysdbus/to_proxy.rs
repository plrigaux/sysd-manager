//#![allow(dead_code)]

use crate::{errors::SystemdErrors, sysd_proxy_service_name, sysdbus::get_connection};
use base::{
    consts::{MAX_HEART_BEAT_ELAPSE, MIN_HEART_BEAT_ELAPSE},
    enums::UnitDBusLevel,
    proxy::{DisEnAbleUnitFiles, DisEnAbleUnitFilesResponse},
};
use std::sync::OnceLock;
use tokio::{
    task::JoinHandle,
    time::{self, Duration},
};
use tracing::{debug, info, warn};
use zbus::{OwnedObjectPath, fdo, proxy};

static HEART_BEAT_HANDLE: OnceLock<JoinHandle<Result<(), SystemdErrors>>> = OnceLock::new();

#[proxy(
    interface = "io.github.plrigaux.SysDManager",
    default_service = "io.github.plrigaux.SysDManager",
    default_path = "/io/github/plrigaux/SysDManager"
)]
pub trait SysDManagerComLink {
    fn start_unit(&self, unit_name: &str, mode: &str) -> fdo::Result<OwnedObjectPath>;
    fn stop_unit(&self, unit_name: &str, mode: &str) -> fdo::Result<OwnedObjectPath>;
    fn restart_unit(&self, unit_name: &str, mode: &str) -> fdo::Result<OwnedObjectPath>;
    fn reload_unit(&self, unit_name: &str, mode: &str) -> fdo::Result<OwnedObjectPath>;
    fn clean_unit(&self, unit_name: &str, what: &[&str]) -> zbus::Result<()>;
    fn freeze_unit(&self, unit_name: &str) -> fdo::Result<()>;
    fn thaw_unit(&self, unit_name: &str) -> fdo::Result<()>;
    fn reload(&self) -> fdo::Result<()>;

    fn create_drop_in(
        &mut self,
        runtime: bool,
        unit_name: &str,
        file_name: &str,
        content: &str,
    ) -> fdo::Result<()>;
    fn save_file(&mut self, file_name: &str, content: &str) -> fdo::Result<u64>;
    fn delete_file(&mut self, file_path: &str) -> fdo::Result<()>;
    fn create_file(&mut self, file_name: &str, content: &str) -> fdo::Result<u64>;

    fn revert_unit_files(&self, file_names: &[&str]) -> fdo::Result<Vec<DisEnAbleUnitFiles>>;

    fn enable_unit_files_with_flags(
        &mut self,
        files: &[&str],
        flags: u64,
    ) -> fdo::Result<DisEnAbleUnitFilesResponse>;

    fn disable_unit_files_with_flags(
        &mut self,
        files: &[&str],
        flags: u64,
    ) -> fdo::Result<DisEnAbleUnitFilesResponse>;

    #[zbus(signal)]
    fn hello(msg: String) -> fdo::Result<()>;

    fn heart_beat(&self) -> fdo::Result<u64>;
    fn reset_failed_unit(&self, unit_name: &str) -> fdo::Result<()>;
    fn reset_failed(&self) -> fdo::Result<()>;

    fn set_log_level(&self, log_level: &str) -> fdo::Result<()>;
    fn set_log_target(&self, log_target: &str) -> fdo::Result<()>;
}

pub(crate) async fn get_proxy_async<'a>() -> Result<SysDManagerComLinkProxy<'a>, SystemdErrors> {
    let destination = super::RUN_CONTEXT
        .get()
        .expect("Supposed to be init")
        .destination_address();
    let connection = get_connection(UnitDBusLevel::System).await?;

    info!("BusName Destination {}", destination);
    let proxy = SysDManagerComLinkProxy::builder(&connection)
        //.path(path)?
        .destination(destination)
        .build()
        .await?;

    Ok(proxy)
}

// pub async fn start_unit(unit_name: &str, mode: &str) -> Result<String, SystemdErrors> {
//     let proxy = get_proxy_async().await?;

//     let s = proxy.start_unit(unit_name, mode).await?;
//     Ok(s.to_string())
// }

// pub async fn stop_unit(unit_name: &str, mode: &str) -> Result<String, SystemdErrors> {
//     let proxy = get_proxy_async().await?;

//     let s = proxy.stop_unit(unit_name, mode).await?;
//     Ok(s.to_string())
// }

// pub async fn restart_unit(unit_name: &str, mode: &str) -> Result<String, SystemdErrors> {
//     let proxy = get_proxy_async().await?;

//     let s = proxy.restart_unit(unit_name, mode).await?;
//     Ok(s.to_string())
// }

pub async fn clean_unit(unit_name: &str, what: &[&str]) -> Result<(), SystemdErrors> {
    info!("By Proxy clean unit {:?}", what);
    get_proxy_async().await?.clean_unit(unit_name, what).await?;
    Ok(())
}

pub async fn freeze_unit(unit_name: &str) -> Result<(), SystemdErrors> {
    info!("By Proxy Freeze unit");
    get_proxy_async().await?.freeze_unit(unit_name).await?;
    Ok(())
}

pub async fn thaw_unit(unit_name: &str) -> Result<(), SystemdErrors> {
    info!("By Proxy Thaw unit ");
    get_proxy_async().await?.thaw_unit(unit_name).await?;
    Ok(())
}

pub async fn reload() -> Result<(), SystemdErrors> {
    info!("By Proxy reload ");
    get_proxy_async().await?.reload().await?;
    Ok(())
}

pub async fn lazy_start_proxy_async() -> Result<(), SystemdErrors> {
    crate::sysdbus::init_proxy_async2().await?;

    Ok(())
}

pub(crate) fn start_heart_beat() {
    if let Some(join_handle) = HEART_BEAT_HANDLE.get()
        && !join_handle.is_finished()
    {
        warn!("There is already an heart beat thread running");
    } else {
        info!("Starting Heart Beat");
        let handle = tokio::spawn(send_heart_beat());
        HEART_BEAT_HANDLE.set(handle);
    }
}

async fn send_heart_beat() -> Result<(), SystemdErrors> {
    let proxy = get_proxy_async().await?;
    loop {
        match proxy.heart_beat().await {
            Ok(delay) => {
                debug!("Heath Beat delay {delay} millis");
                let delay = delay.clamp(MIN_HEART_BEAT_ELAPSE, MAX_HEART_BEAT_ELAPSE);
                time::sleep(Duration::from_millis(delay)).await;
            }
            Err(err) => {
                warn!("Send Heart Beat Error: {err:?}");
                return Err(err.into());
            }
        }
    }
}

pub fn map_unknown_service_to_proxy_error(action_err: SystemdErrors) -> SystemdErrors {
    if matches!(action_err, SystemdErrors::ZFdoServiceUnknown(..)) {
        SystemdErrors::ProxyUnknown(sysd_proxy_service_name())
    } else {
        action_err
    }
}

#[macro_export]
macro_rules! proxy_call_async {
    ($func:ident) => {
        proxy_call_async!($func,)
    };

    ($func:ident, $($param:expr),*) => {
        match $crate::to_proxy::$func($($param),*).await {
            Ok(ok) => Ok(ok),
            Err(SystemdErrors::ZFdoServiceUnknown(msg)) => {
                warn!("Async ServiceUnkown: {:?} Function: {}", msg, stringify!($func));
                $crate::to_proxy::lazy_start_proxy_async().await;
                $crate::to_proxy::$func($($param),*).await
                .map_err(to_proxy::map_unknown_service_to_proxy_error)
            },
            Err(err) => Err(err)
        }
    }
}

pub(crate) async fn create_drop_in(
    runtime: bool,
    unit_name: &str,
    file_path: &str,
    content: &str,
) -> Result<(), SystemdErrors> {
    info!("By Proxy Create Drop-in");
    let mut proxy = get_proxy_async().await?;
    proxy
        .create_drop_in(runtime, unit_name, file_path, content)
        .await
        .map_err(|e| e.into())
}

pub async fn save_file(file_path: &str, content: &str) -> Result<u64, SystemdErrors> {
    info!("By Proxy Save file {:?}", file_path);
    let mut proxy = get_proxy_async().await?;
    proxy
        .save_file(file_path, content)
        .await
        .map_err(|e| e.into())
}

pub async fn delete_file(file_path: &str) -> Result<(), SystemdErrors> {
    info!("By Proxy delete File {:?}", file_path);
    let mut proxy = get_proxy_async().await?;
    proxy.delete_file(file_path).await.map_err(|e| e.into())
}

pub async fn create_file(file_path: &str, content: &str) -> Result<u64, SystemdErrors> {
    info!("By Proxy Create File {:?}", file_path);
    let mut proxy = get_proxy_async().await?;
    proxy
        .create_file(file_path, content)
        .await
        .map_err(|e| e.into())
}

pub async fn revert_unit_files(
    unit_names: &[&str],
) -> Result<Vec<DisEnAbleUnitFiles>, SystemdErrors> {
    info!("By Proxy Revert unit File");
    let proxy = get_proxy_async().await?;
    proxy
        .revert_unit_files(unit_names)
        .await
        .map_err(|e| e.into())
}

pub async fn enable_unit_files_with_flags(
    unit_files: &[&str],
    flags: u64,
) -> Result<DisEnAbleUnitFilesResponse, SystemdErrors> {
    info!("By Proxy Enable unit file {:?} {}", unit_files, flags);
    let mut proxy: SysDManagerComLinkProxy<'_> = get_proxy_async().await?;
    proxy
        .enable_unit_files_with_flags(unit_files, flags)
        .await
        .map_err(|err| err.into())
}

pub async fn disable_unit_files_with_flags(
    unit_files: &[&str],
    flags: u64,
) -> Result<DisEnAbleUnitFilesResponse, SystemdErrors> {
    info!("By Proxy Enable Unit File {:?} {}", unit_files, flags);
    let mut proxy: SysDManagerComLinkProxy<'_> = get_proxy_async().await?;
    proxy
        .disable_unit_files_with_flags(unit_files, flags)
        .await
        .map_err(|err| err.into())
}

pub async fn reset_failed_unit(unit_name: &str) -> Result<(), SystemdErrors> {
    info!("By Proxy Reset Failed Unit {:?}", unit_name);
    let proxy: SysDManagerComLinkProxy<'_> = get_proxy_async().await?;
    proxy
        .reset_failed_unit(unit_name)
        .await
        .map_err(|err| err.into())
}

pub async fn reset_failed() -> Result<(), SystemdErrors> {
    info!("By Proxy Reset Failed");
    let proxy: SysDManagerComLinkProxy<'_> = get_proxy_async().await?;
    proxy.reset_failed().await.map_err(|err| err.into())
}

pub async fn set_log_level(log_level: &str) -> Result<(), SystemdErrors> {
    info!("By Proxy set log level {:?}", log_level);
    get_proxy_async()
        .await?
        .set_log_level(log_level)
        .await
        .map_err(|err| err.into())
}

pub async fn set_log_target(log_target: &str) -> Result<(), SystemdErrors> {
    info!("By Proxy set log target {:?}", log_target);
    get_proxy_async()
        .await?
        .set_log_target(log_target)
        .await
        .map_err(|err| err.into())
}
