use zbus::proxy;

use crate::{enums::UnitDBusLevel, errors::SystemdErrors, sysdbus::get_connection};

macro_rules! pizza {
    () => {
        "pizza"
    };
}

#[proxy(
    interface = "io.github.plrigaux.SysDManager",
    default_service = "io.github.plrigaux.SysDManager",
    default_path = "/io/github/plrigaux/SysDManager"
)]
pub trait SysDManagerComLink {
    fn clean_unit(&self, unit_name: &str, what: &[&str]) -> zbus::Result<()>;
    fn freeze_unit(&self, unit_name: &str) -> zbus::fdo::Result<()>;
    fn thaw_unit(&self, unit_name: &str) -> zbus::fdo::Result<()>;
}

///1 Ensure that the  proxy is up and running
///2 Tertemine mode
///2 Connect to the proxy and return a proxy object
fn ensure_proxy_up() {
    //TODO
}

async fn clean_unit(unit_name: &str, what: &[&str]) -> Result<(), SystemdErrors> {
    let connection = get_connection(UnitDBusLevel::System).await?;
    let proxy = SysDManagerComLinkProxy::builder(&connection)
        .build()
        .await?;
    proxy.clean_unit(unit_name, what).await?;
    Ok(())
}
