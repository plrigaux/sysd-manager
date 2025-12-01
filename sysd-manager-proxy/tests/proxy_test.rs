use base::consts::DBUS_DESTINATION_DEV;
use log::info;

use test_base::init_logs;
use zbus::proxy;

/* pub const DBUS_NAME: &str = "io.github.plrigaux.SysDManager";
pub const DBUS_NAME_DEV: &str = concat!(DBUS_NAME, "Dev");
pub const DBUS_INTERFACE: &str = DBUS_NAME;
pub const DBUS_DESTINATION: &str = DBUS_NAME;
pub const DBUS_DESTINATION_DEV: &str = DBUS_NAME_DEV;
pub const DBUS_PATH: &str = "/io/github/plrigaux/SysDManager";
pub const DBUS_PATH_DEV: &str = concat!(DBUS_PATH, "Dev"); */

#[proxy(
    interface = "io.github.plrigaux.SysDManager",
    default_service = "io.github.plrigaux.SysDManagerDev",
    default_path = "/io/github/plrigaux/SysDManagerDev"
)]
pub trait SysDProxyTester {
    fn clean_unit(&self, unit_name: &str, what: &[&str]) -> zbus::Result<()>;

    fn my_user_id(&self) -> zbus::Result<u32>;

    fn even_ping(&self, val: u32) -> zbus::Result<u32>;
}

async fn system() -> zbus::Result<zbus::Connection> {
    zbus::Connection::system().await
}

async fn system_proxy<'a>() -> zbus::Result<SysDProxyTesterProxy<'a>> {
    let conn = system().await?;
    let proxy = SysDProxyTesterProxy::builder(&conn)
        .destination(DBUS_DESTINATION_DEV)?
        .build()
        .await?;

    Ok(proxy)
}

/* async fn session() -> zbus::Result<zbus::Connection> {
    zbus::Connection::session().await
} */

#[ignore = "need a connection to a service"]
#[tokio::test]
async fn test_sysd_proxy_tester() -> zbus::Result<()> {
    init_logs();
    let conn = system().await?;
    let _proxy = SysDProxyTesterProxy::builder(&conn).build().await?;
    Ok(())
}

#[ignore = "need a connection to a service"]
#[tokio::test]
async fn test_get_user_id() -> zbus::Result<()> {
    init_logs();
    let proxy = system_proxy().await?;

    let uid = proxy.my_user_id().await?;

    info!("User id from proxy: {}", uid);
    Ok(())
}

#[ignore = "need a connection to a service"]
#[tokio::test]
async fn test_even_ping() -> zbus::Result<()> {
    init_logs();
    let proxy = system_proxy().await?;

    let val = proxy.even_ping(42).await?;

    info!("Value returned {}", val);
    Ok(())
}

#[ignore = "need a connection to a service"]
#[tokio::test]
async fn test_even_ping_fail() -> zbus::Result<()> {
    init_logs();
    let proxy = system_proxy().await?;

    match proxy.even_ping(43).await {
        Ok(val) => {
            panic!("Should not succeed, got {}", val);
        }
        Err(zbus::Error::MethodError(a, _b, _c)) => {
            let inner: &zbus::names::ErrorName<'_> = a.inner();
            info!("Expected error received: {}", inner);
        }
        Err(e) => {
            info!("Expected error received: {}", e);
        }
    }

    Ok(())
}
