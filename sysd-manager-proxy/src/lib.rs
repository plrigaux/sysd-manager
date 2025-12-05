mod file;
mod sysdcom;
use base::{RunMode, consts::*, enums::UnitDBusLevel};

use log::{debug, info, warn};
use std::{borrow::Cow, collections::HashMap, env, error::Error, sync::OnceLock};
use tokio::sync::OnceCell;
use zbus::{
    Connection, ObjectServer, connection, interface, message::Header, object_server::SignalEmitter,
};
use zbus_polkit::policykit1::{AuthorityProxy, CheckAuthorizationFlags, Subject};
static AUTHORITY: OnceLock<AuthorityProxy> = OnceLock::new();
static CONNECTION: OnceLock<Connection> = OnceLock::new();

pub async fn init_authority() -> Result<(), zbus::Error> {
    let connection = Connection::system().await?;
    let proxy = AuthorityProxy::new(&connection).await?;

    info!("backend name {}", proxy.backend_name().await?);
    info!("backend version {}", proxy.backend_version().await?);
    info!("backend feature {:?}", proxy.backend_features().await?);

    AUTHORITY.get_or_init(|| proxy);
    Ok(())
}
fn get_env<'a>(key: &str, default: &'a str) -> Cow<'a, str> {
    match env::var(key) {
        Ok(val) => {
            info!("Key {key}, Value {val}");
            Cow::Owned(val)
        }
        Err(e) => {
            debug!("Env error {e:?}");
            info!("Key {key}, Use default value {default}");
            Cow::Borrowed(default)
        }
    }
}
pub async fn init_connection(run_mode: RunMode) -> Result<(), Box<dyn Error>> {
    let proxy = SysDManagerProxy::new()?;

    let id = unsafe { libc::getegid() };
    info!("User id {id}");

    let (default_name, default_path) = if run_mode == RunMode::Development {
        (DBUS_NAME_DEV, DBUS_PATH_DEV)
    } else {
        (DBUS_NAME, DBUS_PATH)
    };

    let dbus_name = get_env("DBUS_NAME", default_name);
    let dbus_path = get_env("DBUS_PATH", default_path);

    info!("DBus name {dbus_name}");
    info!("DBus path {dbus_path}");

    let connection = connection::Builder::system()?
        .name(dbus_name)?
        .serve_at(dbus_path, proxy)?
        .build()
        .await?;

    CONNECTION.get_or_init(|| connection);
    Ok(())
}

pub fn auth() -> &'static AuthorityProxy<'static> {
    AUTHORITY.get().expect("REASON")
}

pub fn conn() -> &'static Connection {
    CONNECTION.get().expect("REASON")
}

pub fn map() -> &'static HashMap<&'static str, &'static str> {
    static MAP: OnceLock<HashMap<&str, &str>> = OnceLock::new();
    MAP.get_or_init(HashMap::new)
}

static SYS_PROXY: OnceCell<sysdcom::SysDManagerComLinkProxy> = OnceCell::const_new();
static SES_PROXY: OnceCell<sysdcom::SysDManagerComLinkProxy> = OnceCell::const_new();

async fn system_proxy() -> Result<&'static sysdcom::SysDManagerComLinkProxy<'static>, zbus::Error> {
    SYS_PROXY
        .get_or_try_init(
            async || -> Result<sysdcom::SysDManagerComLinkProxy, zbus::Error> {
                let connection = Connection::system().await?;
                let proxy = sysdcom::SysDManagerComLinkProxy::builder(&connection)
                    .build()
                    .await?;
                Ok(proxy)
            },
        )
        .await
}

async fn session_proxy() -> Result<&'static sysdcom::SysDManagerComLinkProxy<'static>, zbus::Error>
{
    SES_PROXY
        .get_or_try_init(
            async || -> Result<sysdcom::SysDManagerComLinkProxy, zbus::Error> {
                let connection = Connection::session().await?;
                let proxy = sysdcom::SysDManagerComLinkProxy::builder(&connection)
                    .build()
                    .await?;
                Ok(proxy)
            },
        )
        .await
}

async fn get_proxy(
    dbus_level: UnitDBusLevel,
) -> Result<&'static sysdcom::SysDManagerComLinkProxy<'static>, zbus::Error> {
    match dbus_level {
        UnitDBusLevel::UserSession => session_proxy().await,
        _ => system_proxy().await,
    }
}

pub struct SysDManagerProxy {}

impl SysDManagerProxy {
    pub fn new() -> Result<Self, zbus_polkit::Error> {
        Ok(SysDManagerProxy {})
    }

    async fn check_autorisation(&self, header: Header<'_>) -> Result<(), zbus::fdo::Error> {
        let autority = AUTHORITY.get().expect("REASON");

        let subject = Subject::new_for_message_header(&header).map_err(|err| {
            warn!("Subject new_for_message_header{:?}", err);
            zbus::fdo::Error::AccessDenied("PolKit Subject".to_owned())
        })?;
        let authorization_result = autority
            .check_authorization(
                &subject,
                "io.github.plrigaux.SysDManager",
                map(),
                CheckAuthorizationFlags::AllowUserInteraction.into(),
                "",
            )
            .await;

        match authorization_result {
            Ok(a) => {
                debug!("is_authorized {}", a.is_authorized);
                if a.is_authorized {
                    Ok(())
                } else if a.is_challenge {
                    let msg = format!("{:?}", a.details);
                    Err(zbus::fdo::Error::InteractiveAuthorizationRequired(msg))
                } else {
                    let msg = format!("{:?}", a.details);
                    Err(zbus::fdo::Error::AuthFailed(msg))
                }
            }
            Err(e) => {
                warn!("check_authorization {:?}", e);
                let err: zbus::fdo::Error = e.into();
                Err(err)
            }
        }
    }
}

#[interface(name = "io.github.plrigaux.SysDManager", introspection_docs = true)]
impl SysDManagerProxy {
    pub async fn create_drop_in(
        &mut self,
        #[zbus(header)] header: Header<'_>,
        dbus: u8,
        runtime: bool,
        unit_name: &str,
        file_name: &str,
        content: &str,
    ) -> zbus::fdo::Result<()> {
        //self.
        self.check_autorisation(header).await?;

        //   self.get_all(object_server, connection, header, emitter)
        file::create_drop_in(dbus, runtime, unit_name, file_name, content).await
    }

    pub async fn save_file(
        &mut self,
        #[zbus(header)] header: Header<'_>,
        dbus: u8,
        file_path: &str,
        content: &str,
    ) -> zbus::fdo::Result<()> {
        self.check_autorisation(header).await?;
        file::save(dbus, file_path, content).await
    }

    pub async fn my_user_id(
        &mut self,
        #[zbus(header)] header: Header<'_>,
    ) -> zbus::fdo::Result<u32> {
        self.check_autorisation(header).await?;

        let id = unsafe { libc::getegid() };
        info!("ids {}", id);

        Ok(id)
    }
    // "Bye" signal (note: no implementation body).
    #[zbus(signal)]
    async fn bye(signal_emitter: &SignalEmitter<'_>, message: &str) -> zbus::Result<()>;

    // "Quit" method. A method may throw errors.
    async fn quit(
        &self,
        #[zbus(header)] hdr: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        #[zbus(object_server)] _server: &ObjectServer,
    ) -> zbus::fdo::Result<()> {
        let path = hdr.path().unwrap();
        let msg = format!("You are leaving me on the {} path?", path);
        emitter.bye(&msg).await?;

        // Do some asynchronous tasks before quitting..

        Ok(())
    }

    async fn even_ping(
        &mut self,
        #[zbus(header)] header: Header<'_>,
        val: u32,
    ) -> zbus::fdo::Result<u32> {
        info!("even_ping {val}");
        self.check_autorisation(header).await?;
        if val.is_multiple_of(2) {
            Ok(val)
        } else {
            Err(zbus::fdo::Error::Failed(format!("{val} not even!")))
        }
    }

    async fn clean_unit(
        &self,
        #[zbus(header)] header: Header<'_>,
        dbus: u8,
        unit_name: &str,
        what: Vec<&str>,
    ) -> zbus::fdo::Result<()> {
        self.check_autorisation(header).await?;
        let proxy = get_proxy(UnitDBusLevel::from(dbus)).await?;

        info!("clean_unit {} {:?}", unit_name, what);
        proxy
            .clean_unit(unit_name, &what)
            .await
            .inspect_err(|e| warn!("Error while calling clean_unit on sysdbus proxy: {:?}", e))?;
        Ok(())
    }

    async fn freeze_unit(
        &self,
        #[zbus(header)] header: Header<'_>,
        dbus: u8,
        unit_name: &str,
    ) -> zbus::fdo::Result<()> {
        let proxy = get_proxy(UnitDBusLevel::from(dbus)).await?;
        self.check_autorisation(header).await?;
        info!("freeze_unit {}", unit_name);
        proxy
            .freeze_unit(unit_name)
            .await
            .inspect_err(|e| warn!("Error while calling freeze_unit on sysdbus proxy: {:?}", e))?;
        Ok(())
    }

    async fn thaw_unit(
        &self,
        #[zbus(header)] header: Header<'_>,
        dbus: u8,
        unit_name: &str,
    ) -> zbus::fdo::Result<()> {
        let proxy = get_proxy(UnitDBusLevel::from(dbus)).await?;
        self.check_autorisation(header).await?;
        info!("thaw_unit {}", unit_name);
        proxy
            .thaw_unit(unit_name)
            .await
            .inspect_err(|e| warn!("Error while calling thaw_unit on sysdbus proxy: {:?}", e))?;
        Ok(())
    }
}
