mod file;
mod interface;
mod sysdcom;
use base::{
    RunMode,
    consts::{DBUS_NAME, DBUS_NAME_DEV, DBUS_PATH, MAX_HEART_BEAT_ELAPSE, MIN_HEART_BEAT_ELAPSE},
};
use tokio::{
    sync::Mutex,
    time::{self, Instant, sleep},
};
pub mod install;
use crate::interface::SysDManagerProxySignals;
use std::{
    borrow::Cow,
    collections::HashMap,
    env,
    error::Error,
    future::pending,
    sync::{Arc, OnceLock},
    time::Duration,
};
use tracing::{debug, info, warn};
use tracing_subscriber::{EnvFilter, fmt};
use zbus::{Connection, message::Header};
use zbus_polkit::policykit1::{AuthorityProxy, CheckAuthorizationFlags, Subject};

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

pub struct SysDManagerProxy {
    last_heart_beat: Arc<Mutex<Instant>>,
    heart_beat_delay: u64,
}

impl SysDManagerProxy {
    pub fn new() -> Result<Self, zbus_polkit::Error> {
        let value = env::var("HEART_BEAT_DELAY").unwrap_or_default();

        let value = value
            .parse::<u64>()
            .unwrap_or(15_000)
            .clamp(MIN_HEART_BEAT_ELAPSE, MAX_HEART_BEAT_ELAPSE);
        info!("Heart Beat Delay {} millis", value);
        let now = time::Instant::now();

        let proxy = SysDManagerProxy {
            last_heart_beat: Arc::new(Mutex::new(now)),
            heart_beat_delay: value,
        };
        Ok(proxy)
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
        //.with_max_level(tracing ::Level::DEBUG)
        .with_env_filter(EnvFilter::from_default_env())
        .with_line_number(true)
        .init();
}

pub async fn serve_proxy(run_mode: RunMode, heart_beat: bool) -> Result<(), Box<dyn Error>> {
    init_authority().await?;
    let (connection, path) = init_serve_connection(run_mode, heart_beat).await?;

    connection
        .object_server()
        .interface(path)
        .await?
        .hello("Bob")
        .await?;
    // Do other things or go to wait forever
    pending::<()>().await;

    Ok(())
}

pub async fn init_serve_connection(
    run_mode: RunMode,
    heart_beat: bool,
) -> Result<(Connection, String), Box<dyn Error>> {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    info!("Init Proxy version {VERSION}");

    let proxy = SysDManagerProxy::new()?;

    let last_heart_beat = proxy.last_heart_beat.clone();
    let heart_beat_delay = Duration::from_millis(proxy.heart_beat_delay);
    let time_trigger = heart_beat_delay * 3;

    if heart_beat {
        tokio::spawn(async move {
            loop {
                debug!("Sleep {:?}", heart_beat_delay);
                sleep(heart_beat_delay).await;
                debug!("Wake");
                let last_heart_beat = last_heart_beat.lock().await;
                let elapsed = last_heart_beat.elapsed();

                debug!("Check Heart Beat {:?} {:?}", elapsed, time_trigger);
                if elapsed > time_trigger {
                    warn!("Time trigger busted! {:?}", last_heart_beat.elapsed());
                    std::process::exit(0);
                }
            }
        });
    }

    let id = unsafe { libc::getegid() };
    info!("User id {id}");

    let default_name = if run_mode == RunMode::Development {
        DBUS_NAME_DEV
    } else {
        DBUS_NAME
    };

    let dbus_name = get_env("DBUS_NAME", default_name);
    let dbus_path = get_env("DBUS_PATH", DBUS_PATH);

    info!("DBus name {dbus_name}");
    info!("DBus path {dbus_path}");

    let connection = zbus::connection::Builder::system()?
        .name(dbus_name)?
        .serve_at(dbus_path.clone(), proxy)?
        .build()
        .await?;

    Ok((connection, dbus_path.to_string()))
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
