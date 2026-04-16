use crate::{
    data::UnitInfo,
    errors::SystemdErrors,
    runtime,
    sysdbus::{dbus_proxies::Systemd1ManagerProxy, get_connection},
};
use base::enums::UnitDBusLevel;
use futures_util::stream::StreamExt;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::{
    sync::{OnceCell, broadcast, oneshot},
    task::JoinHandle,
};
use tracing::{debug, error, info, warn};
use zbus::{MatchRule, MessageStream};
use zvariant::OwnedObjectPath;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum SystemdSignal {
    UnitNew(UnitDBusLevel, String),
    UnitRemoved(UnitDBusLevel, String),
    JobNew(UnitDBusLevel, u32, OwnedObjectPath, String),
    JobRemoved(UnitDBusLevel, u32, OwnedObjectPath, String, String),
    StartupFinished(UnitDBusLevel, u64, u64, u64, u64, u64, u64),
    UnitFilesChanged(UnitDBusLevel),
    Reloading(UnitDBusLevel, bool),
}

impl SystemdSignal {
    pub fn type_text(&self) -> &str {
        match self {
            SystemdSignal::UnitNew(_, _) => "UnitNew",
            SystemdSignal::UnitRemoved(_, _) => "UnitRemoved",
            SystemdSignal::JobNew(_, _, _, _) => "JobNew",
            SystemdSignal::JobRemoved(_, _, _, _, _) => "JobRemoved",
            SystemdSignal::StartupFinished(_, _, _, _, _, _, _) => "StartupFinished",
            SystemdSignal::UnitFilesChanged(_) => "UnitFilesChanged",
            SystemdSignal::Reloading(_, _) => "Reloading",
        }
    }

    pub fn bus_text(&self) -> &str {
        let level = match self {
            SystemdSignal::UnitNew(level, _) => level,
            SystemdSignal::UnitRemoved(level, _) => level,
            SystemdSignal::JobNew(level, _, _, _) => level,
            SystemdSignal::JobRemoved(level, _, _, _, _) => level,
            SystemdSignal::StartupFinished(level, _, _, _, _, _, _) => level,
            SystemdSignal::UnitFilesChanged(level) => level,
            SystemdSignal::Reloading(level, _) => level,
        };
        level.as_str()
    }

    pub fn details(&self) -> String {
        match self {
            SystemdSignal::UnitNew(_, id) => id.to_string(),
            SystemdSignal::UnitRemoved(_, id) => id.to_string(),
            SystemdSignal::JobNew(_, id, job, unit) => {
                format!("unit={unit} id={id} path={job}")
            }
            SystemdSignal::JobRemoved(_, id, job, unit, result) => {
                format!("unit={unit} id={id} path={job} result={result}")
            }
            SystemdSignal::StartupFinished(
                _,
                firmware,
                loader,
                kernel,
                initrd,
                userspace,
                total,
            ) => {
                format!(
                    "firmware={firmware} loader={loader} kernel={kernel} initrd={initrd} userspace={userspace} total={total}",
                )
            }
            SystemdSignal::UnitFilesChanged(_) => String::new(),
            SystemdSignal::Reloading(_, active) => format!("active={active}"),
        }
    }

    pub fn toggle_unit(self) -> Self {
        match self {
            SystemdSignal::UnitNew(unit_dbus_level, unit_name) => {
                Self::UnitRemoved(unit_dbus_level, unit_name)
            }
            SystemdSignal::UnitRemoved(unit_dbus_level, unit_name) => {
                Self::UnitNew(unit_dbus_level, unit_name)
            }
            _ => self,
        }
    }

    pub fn create_unit(&self) -> Option<UnitInfo> {
        match self {
            SystemdSignal::UnitNew(unit_dbus_level, unit_name) => {
                Some(UnitInfo::from_unit_key(unit_name, *unit_dbus_level))
            }
            SystemdSignal::UnitRemoved(unit_dbus_level, unit_name) => {
                Some(UnitInfo::from_unit_key(unit_name, *unit_dbus_level))
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SystemdSignalRow {
    pub time_stamp: u64,
    pub signal: SystemdSignal,
}

impl SystemdSignalRow {
    pub fn new(signal: SystemdSignal) -> Self {
        let current_system_time = SystemTime::now();
        let since_the_epoch = current_system_time
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        let time_stamp =
            since_the_epoch.as_secs() * 1_000_000 + since_the_epoch.subsec_nanos() as u64 / 1_000;
        SystemdSignalRow { time_stamp, signal }
    }

    pub fn type_text(&self) -> &str {
        self.signal.type_text()
    }

    pub fn bus_text(&self) -> &str {
        self.signal.bus_text()
    }

    pub fn details(&self) -> String {
        self.signal.details()
    }
}

static SENDER: OnceCell<broadcast::Sender<SystemdSignal>> = OnceCell::const_new();
static WACHER_SYSTEM: OnceCell<JoinHandle<Result<(), SystemdErrors>>> = OnceCell::const_new();
static WACHER_USER_SESSION: OnceCell<JoinHandle<Result<(), SystemdErrors>>> = OnceCell::const_new();

pub async fn init_signal_watcher(level: UnitDBusLevel) -> broadcast::Receiver<SystemdSignal> {
    let sender = SENDER
        .get_or_init(async || {
            let (systemd_signal_sender, _) = broadcast::channel(2500);

            // let cancellation_token = tokio_util::sync::CancellationToken::new();

            systemd_signal_sender
        })
        .await;

    match level {
        UnitDBusLevel::System => {
            WACHER_SYSTEM
                .get_or_init(|| spawn_signal_watcher(UnitDBusLevel::System, sender))
                .await;
        }
        UnitDBusLevel::UserSession => {
            WACHER_USER_SESSION
                .get_or_init(|| spawn_signal_watcher(UnitDBusLevel::UserSession, sender))
                .await;
        }
        UnitDBusLevel::Both => {
            WACHER_SYSTEM.get_or_init(|| spawn_signal_watcher(UnitDBusLevel::System, sender));
            WACHER_USER_SESSION
                .get_or_init(|| spawn_signal_watcher(UnitDBusLevel::UserSession, sender))
                .await;
        }
    };

    sender.subscribe()
}

async fn spawn_signal_watcher(
    level: UnitDBusLevel,
    sender: &broadcast::Sender<SystemdSignal>,
) -> JoinHandle<Result<(), SystemdErrors>> {
    let sender = sender.clone();
    let (tell_is_ready, is_ready_ok) = oneshot::channel();
    let handle = runtime().spawn(signal_watcher(level, sender, tell_is_ready));

    let _ = is_ready_ok
        .await
        .inspect_err(|err| error!("Tokio channel dropped {err:?}"));
    handle
}

async fn signal_watcher(
    level: UnitDBusLevel,
    systemd_signal_sender: broadcast::Sender<SystemdSignal>,
    tell_is_ready: oneshot::Sender<()>,
) -> Result<(), SystemdErrors> {
    info!("Starting Watcher {:?}", level);
    let connection = get_connection(level).await?;

    let systemd_proxy = Systemd1ManagerProxy::new(&connection).await?;
    if let Err(err) = systemd_proxy.subscribe().await {
        warn!("Subscribe error {:?}", err);
    };
    let rule = MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        // .sender("org.freedesktop.DBus")?
        .interface("org.freedesktop.systemd1.Manager")?
        // .member("NameOwnerChanged")?
        // .add_arg("org.freedesktop.zbus.MatchRuleStreamTest42")?
        .build();

    let mut stream = MessageStream::for_match_rule(
        rule,
        &connection,
        // For such a specific match rule, we don't need a big queue.
        Some(100),
    )
    .await?;

    tell_is_ready.send(());

    while let Some(message) = stream.next().await {
        let signal = match message {
            Ok(message) => match (
                message.message_type(),
                message
                    .header()
                    .member()
                    .map(|member_name| member_name.as_str()),
            ) {
                (zbus::message::Type::Signal, Some("UnitNew")) => {
                    let (unit, _path): (String, OwnedObjectPath) = message.body().deserialize()?;
                    Some(SystemdSignal::UnitNew(level, unit))
                }
                (zbus::message::Type::Signal, Some("UnitRemoved")) => {
                    let (unit, _path): (String, OwnedObjectPath) = message.body().deserialize()?;
                    Some(SystemdSignal::UnitRemoved(level, unit))
                }
                (zbus::message::Type::Signal, Some("JobRemoved")) => {
                    let (id, job, unit, result): (u32, OwnedObjectPath, String, String) =
                        message.body().deserialize()?;
                    Some(SystemdSignal::JobRemoved(level, id, job, unit, result))
                }
                (zbus::message::Type::Signal, Some("JobNew")) => {
                    let (id, job, unit): (u32, OwnedObjectPath, String) =
                        message.body().deserialize()?;
                    Some(SystemdSignal::JobNew(level, id, job, unit))
                }
                (zbus::message::Type::Signal, Some("Reloading")) => {
                    let active: bool = message.body().deserialize()?;
                    Some(SystemdSignal::Reloading(level, active))
                }
                (zbus::message::Type::Signal, Some("StartupFinished")) => {
                    let (firmware, loader, kernel, initrd, userspace, total): (
                        u64,
                        u64,
                        u64,
                        u64,
                        u64,
                        u64,
                    ) = message.body().deserialize()?;
                    Some(SystemdSignal::StartupFinished(
                        level, firmware, loader, kernel, initrd, userspace, total,
                    ))
                }
                (zbus::message::Type::Signal, Some("UnitFilesChanged")) => {
                    Some(SystemdSignal::UnitFilesChanged(level))
                }

                (zbus::message::Type::Signal, _) => {
                    warn!("Unhandled Signal {message:?}");
                    None
                }
                (zbus::message::Type::MethodCall, _) => {
                    info!("Method Call {message:?}");
                    None
                }
                (zbus::message::Type::MethodReturn, _) => {
                    info!("Method Ret {message:?}");
                    None
                }
                (zbus::message::Type::Error, _) => {
                    warn!("Error {message:?}");
                    None
                }
            },
            Err(err) => {
                error!("{err}");
                None
            }
        };

        if let Some(signal) = signal
            && let Err(error) = systemd_signal_sender.send(signal)
        {
            debug!("Send signal Error {error:?}")
        };
    }

    Ok(())
}
