#![allow(dead_code)]
use base::enums::UnitDBusLevel;
use zbus::proxy;

use crate::{errors::SystemdErrors, sysdbus::get_blocking_connection};

#[proxy(
    interface = "io.github.plrigaux.SysDManager",
    default_service = "io.github.plrigaux.SysDManager",
    default_path = "/io/github/plrigaux/SysDManager"
)]
pub trait SysDManagerComLink {
    fn clean_unit(&self, bus: u8, unit_name: &str, what: &[&str]) -> zbus::Result<()>;
    fn freeze_unit(&self, bus: u8, unit_name: &str) -> zbus::fdo::Result<()>;
    fn thaw_unit(&self, bus: u8, unit_name: &str) -> zbus::fdo::Result<()>;

    fn create_dropin(&mut self, file_name: &str, content: &str) -> zbus::fdo::Result<String>;
    fn save_file(&mut self, file_name: &str, content: &str) -> zbus::fdo::Result<String>;
}

///1 Ensure that the  proxy is up and running
///2 Tertemine mode
///2 Connect to the proxy and return a proxy object
fn ensure_proxy_up() {
    //TODO
}

fn get_proxy<'a>() -> Result<SysDManagerComLinkProxyBlocking<'a>, SystemdErrors> {
    let (path, destination) = super::RUN_CONTEXT
        .get()
        .expect("Supposed to be init")
        .path_destination();
    let connection = get_blocking_connection(UnitDBusLevel::System)?;
    let proxy = SysDManagerComLinkProxyBlocking::builder(&connection)
        .path(path)?
        .destination(destination)?
        .build()?;

    Ok(proxy)
}

pub fn clean_unit(bus: UnitDBusLevel, unit_name: &str, what: &[&str]) -> Result<(), SystemdErrors> {
    let proxy = get_proxy()?;

    proxy.clean_unit(bus.index(), unit_name, what)?;
    Ok(())
}

pub fn freeze_unit(bus: UnitDBusLevel, unit_name: &str) -> Result<(), SystemdErrors> {
    let proxy = get_proxy()?;
    proxy.freeze_unit(bus.index(), unit_name)?;
    Ok(())
}

pub fn thaw_unit(bus: UnitDBusLevel, unit_name: &str) -> Result<(), SystemdErrors> {
    let proxy = get_proxy()?;
    proxy.thaw_unit(bus.index(), unit_name)?;
    Ok(())
}
