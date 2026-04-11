use crate::{
    errors::SystemdErrors,
    runtime,
    sysdbus::{dbus_proxies::Systemd1ManagerProxy, get_connection},
};
use base::enums::UnitDBusLevel;
use futures_util::stream::StreamExt;
use std::{
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::{sync::broadcast, task::JoinHandle};
use tracing::{debug, error, info, warn};
use zbus::{MatchRule, MessageStream};
use zvariant::OwnedObjectPath;

#[derive(Debug, Clone)]
pub enum SystemdSignal {
    UnitNew(UnitDBusLevel, String, OwnedObjectPath),
    UnitRemoved(UnitDBusLevel, String, OwnedObjectPath),
    JobNew(UnitDBusLevel, u32, OwnedObjectPath, String),
    JobRemoved(UnitDBusLevel, u32, OwnedObjectPath, String, String),
    StartupFinished(UnitDBusLevel, u64, u64, u64, u64, u64, u64),
    UnitFilesChanged(UnitDBusLevel),
    Reloading(UnitDBusLevel, bool),
}

impl SystemdSignal {
    pub fn type_text(&self) -> &str {
        match self {
            SystemdSignal::UnitNew(_, _, _) => "UnitNew",
            SystemdSignal::UnitRemoved(_, _, _) => "UnitRemoved",
            SystemdSignal::JobNew(_, _, _, _) => "JobNew",
            SystemdSignal::JobRemoved(_, _, _, _, _) => "JobRemoved",
            SystemdSignal::StartupFinished(_, _, _, _, _, _, _) => "StartupFinished",
            SystemdSignal::UnitFilesChanged(_) => "UnitFilesChanged",
            SystemdSignal::Reloading(_, _) => "Reloading",
        }
    }

    pub fn bus_text(&self) -> &str {
        let level = match self {
            SystemdSignal::UnitNew(level, _, _) => level,
            SystemdSignal::UnitRemoved(level, _, _) => level,
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
            SystemdSignal::UnitNew(_, id, unit) => format!("{id} {unit}"),
            SystemdSignal::UnitRemoved(_, id, unit) => format!("{id} {unit}"),
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

static SENDER: OnceLock<broadcast::Sender<SystemdSignalRow>> = OnceLock::new();
static WACHER_SYSTEM: OnceLock<JoinHandle<Result<(), SystemdErrors>>> = OnceLock::new();
static WACHER_USER_SESSION: OnceLock<JoinHandle<Result<(), SystemdErrors>>> = OnceLock::new();

pub fn init_signal_watcher(level: UnitDBusLevel) -> broadcast::Receiver<SystemdSignalRow> {
    let sender = SENDER.get_or_init(|| {
        let (systemd_signal_sender, _) = broadcast::channel(2500);

        // let cancellation_token = tokio_util::sync::CancellationToken::new();

        systemd_signal_sender
    });

    match level {
        UnitDBusLevel::System => {
            WACHER_SYSTEM.get_or_init(|| runtime().spawn(signal_watcher(level, sender.clone())));
        }
        UnitDBusLevel::UserSession => {
            WACHER_USER_SESSION
                .get_or_init(|| runtime().spawn(signal_watcher(level, sender.clone())));
        }
        UnitDBusLevel::Both => {
            WACHER_SYSTEM.get_or_init(|| {
                runtime().spawn(signal_watcher(UnitDBusLevel::System, sender.clone()))
            });
            WACHER_USER_SESSION.get_or_init(|| {
                runtime().spawn(signal_watcher(UnitDBusLevel::UserSession, sender.clone()))
            });
        }
    };

    sender.subscribe()
}

async fn signal_watcher(
    level: UnitDBusLevel,
    systemd_signal_sender: broadcast::Sender<SystemdSignalRow>,
) -> Result<(), SystemdErrors> {
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
                    let (unit, path): (String, OwnedObjectPath) = message.body().deserialize()?;
                    Some(SystemdSignal::UnitNew(level, unit, path))
                }
                (zbus::message::Type::Signal, Some("UnitRemoved")) => {
                    let (unit, path): (String, OwnedObjectPath) = message.body().deserialize()?;
                    Some(SystemdSignal::UnitRemoved(level, unit, path))
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

        if let Some(signal) = signal {
            let signal_row = SystemdSignalRow::new(signal);

            if let Err(error) = systemd_signal_sender.send(signal_row) {
                debug!("Send signal Error {error:?}")
            };
        }
    }

    Ok(())
}
