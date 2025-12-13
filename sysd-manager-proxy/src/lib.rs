mod file;
mod interface;
mod sysdcom;
use base::RunMode;
pub mod install;
use log::{debug, info, warn};
use std::{collections::HashMap, error::Error, future::pending, sync::OnceLock};
use tracing_subscriber::fmt;
use zbus::{Connection, message::Header};
use zbus_polkit::policykit1::{AuthorityProxy, CheckAuthorizationFlags, Subject};

use crate::interface::init_serve_connection;
static AUTHORITY: OnceLock<AuthorityProxy> = OnceLock::new();
static CONNECTION: OnceLock<Connection> = OnceLock::new();

pub async fn init_authority() -> Result<(), zbus::Error> {
    info!("Init Proxy Authority");
    let connection = Connection::system().await?;
    let proxy = AuthorityProxy::new(&connection).await?;

    info!("backend name {}", proxy.backend_name().await?);
    info!("backend version {}", proxy.backend_version().await?);
    info!("backend feature {:?}", proxy.backend_features().await?);

    AUTHORITY.get_or_init(|| proxy);
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

pub fn init_tracing() {
    let timer = fmt::time::ChronoLocal::new("%Y-%m-%d %H:%M:%S%.3f".to_owned());
    //let timer = fmt::time::ChronoLocal::rfc_3339();

    tracing_subscriber::fmt()
        .with_timer(timer)
        .with_max_level(tracing::Level::DEBUG)
        .with_line_number(true)
        .init();
}

pub async fn serve_proxy(run_mode: RunMode) -> Result<(), Box<dyn Error>> {
    init_authority().await?;
    init_serve_connection(run_mode).await?;

    // Do other things or go to wait forever
    pending::<()>().await;

    Ok(())
}
