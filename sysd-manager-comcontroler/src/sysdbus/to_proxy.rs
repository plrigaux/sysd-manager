#![allow(dead_code)]
use base::{
    enums::UnitDBusLevel,
    proxy::{DisEnAbleUnitFiles, DisEnAbleUnitFilesResponse},
};
use futures_util::stream::StreamExt;
use log::{info, warn};
use tokio::time::timeout;
use zbus::proxy;

use crate::{
    errors::SystemdErrors,
    sysdbus::{get_blocking_connection, get_connection},
};

#[proxy(
    interface = "io.github.plrigaux.SysDManager",
    default_service = "io.github.plrigaux.SysDManager",
    default_path = "/io/github/plrigaux/SysDManager"
)]
pub trait SysDManagerComLink {
    fn clean_unit(&self, unit_name: &str, what: &[&str]) -> zbus::Result<()>;
    fn freeze_unit(&self, unit_name: &str) -> zbus::fdo::Result<()>;
    fn thaw_unit(&self, unit_name: &str) -> zbus::fdo::Result<()>;
    fn reload(&self) -> zbus::fdo::Result<()>;

    fn create_drop_in(
        &mut self,
        runtime: bool,
        unit_name: &str,
        file_name: &str,
        content: &str,
    ) -> zbus::fdo::Result<()>;
    fn save_file(&mut self, file_name: &str, content: &str) -> zbus::fdo::Result<u64>;

    fn revert_unit_files(&self, file_names: &[&str]) -> zbus::fdo::Result<Vec<DisEnAbleUnitFiles>>;

    fn enable_unit_files_with_flags(
        &mut self,
        files: &[&str],
        flags: u64,
    ) -> zbus::fdo::Result<DisEnAbleUnitFilesResponse>;

    fn disable_unit_files_with_flags(
        &mut self,
        files: &[&str],
        flags: u64,
    ) -> zbus::fdo::Result<DisEnAbleUnitFilesResponse>;

    #[zbus(signal)]
    fn hello(msg: String) -> zbus::fdo::Result<()>;
}

///1 Ensure that the  proxy is up and running
///2 Tertemine mode
///2 Connect to the proxy and return a proxy object
fn ensure_proxy_up() {
    //TODO ensure_proxy_up
}

fn get_proxy<'a>() -> Result<SysDManagerComLinkProxyBlocking<'a>, SystemdErrors> {
    let destination = super::RUN_CONTEXT
        .get()
        .expect("Supposed to be init")
        .destination_address();
    let connection = get_blocking_connection(UnitDBusLevel::System)?;
    let proxy = SysDManagerComLinkProxyBlocking::builder(&connection)
        // .path(path)?
        .destination(destination)?
        .build()?;

    Ok(proxy)
}

async fn get_proxy_async<'a>() -> Result<SysDManagerComLinkProxy<'a>, SystemdErrors> {
    let destination = super::RUN_CONTEXT
        .get()
        .expect("Supposed to be init")
        .destination_address();
    let connection = get_connection(UnitDBusLevel::System).await?;
    let proxy = SysDManagerComLinkProxy::builder(&connection)
        //.path(path)?
        .destination(destination)?
        .build()
        .await?;

    Ok(proxy)
}

pub fn clean_unit(unit_name: &str, what: &[&str]) -> Result<(), SystemdErrors> {
    let proxy = get_proxy()?;

    proxy.clean_unit(unit_name, what)?;
    Ok(())
}

pub fn freeze_unit(unit_name: &str) -> Result<(), SystemdErrors> {
    let proxy = get_proxy()?;
    proxy.freeze_unit(unit_name)?;
    Ok(())
}

pub fn thaw_unit(unit_name: &str) -> Result<(), SystemdErrors> {
    let proxy: SysDManagerComLinkProxyBlocking<'_> = get_proxy()?;
    proxy.thaw_unit(unit_name)?;
    Ok(())
}

pub async fn reload() -> Result<(), SystemdErrors> {
    let proxy = get_proxy_async().await?;
    proxy.reload().await?;
    Ok(())
}

fn extract_job_id(job: &str) -> Option<u32> {
    job.rsplit_once('/')
        .and_then(|(_, id)| id.parse::<u32>().ok())
}

pub async fn lazy_start_proxy_async() -> Result<(), SystemdErrors> {
    let proxy = get_proxy_async().await?;
    let hello_stream = proxy.receive_hello().await?;
    crate::sysdbus::init_proxy_async2().await?;

    let r = timeout(
        tokio::time::Duration::from_secs(2),
        wait_hello(hello_stream),
    )
    .await;

    match r {
        Ok(rr) => rr?,
        Err(e) => warn!("Proxy start time up : {}", e),
    }
    Ok(())
}

async fn wait_hello(mut hello_stream: HelloStream) -> Result<(), SystemdErrors> {
    if let Some(msg) = hello_stream.next().await {
        let args = msg.args()?;
        info!("Hello Proxy Args {:?}", args);
    }
    Ok(())
}

pub fn lazy_start_proxy_block() -> Result<(), SystemdErrors> {
    crate::runtime().block_on(async move {
        warn!("lazy 1");
        lazy_start_proxy_async().await;
        warn!("lazy 2");
    });
    Ok(())
}

#[macro_export]
macro_rules! proxy_call {
    ($f:ident,$($p:expr),+) => {
        match $crate::to_proxy::$f($($p),+) {
            Ok(ok) => Ok(ok),
            Err(SystemdErrors::ZFdoServiceUnknowm(s)) => {
                warn!("ServiceUnkown: {}", s);
                $crate::to_proxy::lazy_start_proxy_block();

                $crate::to_proxy::$f($($p),+)
            },
            Err(err) => Err(err)
        }
    }
}

#[macro_export]
macro_rules! proxy_call_async {
    ($f:ident) => {
        proxy_call_async!($f,)
    };

    ($f:ident, $($p:expr),*) => {
        match $crate::to_proxy::$f($($p),*).await {
            Ok(ok) => Ok(ok),
            Err(SystemdErrors::ZFdoServiceUnknowm(s)) => {
                warn!("ServiceUnkown: {}", s);
                $crate::to_proxy::lazy_start_proxy_async();

                $crate::to_proxy::$f($($p),*).await
            },
            Err(err) => Err(err)
        }
    }
}

pub(crate) async fn create_drop_in(
    runtime: bool,
    unit_name: &str,
    file_name: &str,
    content: &str,
) -> Result<(), SystemdErrors> {
    let mut proxy = get_proxy_async().await?;
    proxy
        .create_drop_in(runtime, unit_name, file_name, content)
        .await
        .map_err(|e| e.into())
}

pub async fn save_file(file_path: &str, content: &str) -> Result<u64, SystemdErrors> {
    let mut proxy = get_proxy_async().await?;
    proxy
        .save_file(file_path, content)
        .await
        .map_err(|e| e.into())
}

pub async fn revert_unit_files(
    unit_names: &[&str],
) -> Result<Vec<DisEnAbleUnitFiles>, SystemdErrors> {
    let proxy = get_proxy_async().await?;
    proxy
        .revert_unit_files(unit_names)
        .await
        .map_err(|e| e.into())
}

pub fn enable_unit_files_with_flags(
    unit_files: &[&str],
    flags: u64,
) -> Result<DisEnAbleUnitFilesResponse, SystemdErrors> {
    let mut proxy: SysDManagerComLinkProxyBlocking<'_> = get_proxy()?;
    proxy
        .enable_unit_files_with_flags(unit_files, flags)
        .map_err(|err| err.into())
}

pub fn disable_unit_files_with_flags(
    unit_files: &[&str],
    flags: u64,
) -> Result<DisEnAbleUnitFilesResponse, SystemdErrors> {
    let mut proxy: SysDManagerComLinkProxyBlocking<'_> = get_proxy()?;
    proxy
        .disable_unit_files_with_flags(unit_files, flags)
        .map_err(|err| err.into())
}
