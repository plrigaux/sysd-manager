use crate::{
    SystemdSignal, SystemdSignalRow,
    errors::SystemdErrors,
    runtime,
    sysdbus::{dbus_proxies::Systemd1ManagerProxy, get_connection},
};
use base::enums::UnitDBusLevel;
use futures_util::stream::StreamExt;
use std::sync::OnceLock;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};
use zbus::{MatchRule, MessageStream};
use zvariant::OwnedObjectPath;

static SENDER: OnceLock<broadcast::Sender<SystemdSignalRow>> = OnceLock::new();

pub fn init_signal_watcher() -> broadcast::Receiver<SystemdSignalRow> {
    let sender = SENDER.get_or_init(|| {
        let (systemd_signal_sender, _) = broadcast::channel(2500);

        // let cancellation_token = tokio_util::sync::CancellationToken::new();

        runtime().spawn(signal_watcher(systemd_signal_sender.clone()));

        systemd_signal_sender
    });

    sender.subscribe()
}

async fn signal_watcher(
    systemd_signal_sender: broadcast::Sender<SystemdSignalRow>,
) -> Result<(), SystemdErrors> {
    let connection = get_connection(UnitDBusLevel::System).await?;

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
                    Some(SystemdSignal::UnitNew(unit, path))
                }
                (zbus::message::Type::Signal, Some("UnitRemoved")) => {
                    let (unit, path): (String, OwnedObjectPath) = message.body().deserialize()?;
                    Some(SystemdSignal::UnitRemoved(unit, path))
                }
                (zbus::message::Type::Signal, Some("JobRemoved")) => {
                    let (id, job, unit, result): (u32, OwnedObjectPath, String, String) =
                        message.body().deserialize()?;
                    Some(SystemdSignal::JobRemoved(id, job, unit, result))
                }
                (zbus::message::Type::Signal, Some("JobNew")) => {
                    let (id, job, unit): (u32, OwnedObjectPath, String) =
                        message.body().deserialize()?;
                    Some(SystemdSignal::JobNew(id, job, unit))
                }
                (zbus::message::Type::Signal, Some("Reloading")) => {
                    let active: bool = message.body().deserialize()?;
                    Some(SystemdSignal::Reloading(active))
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
                        firmware, loader, kernel, initrd, userspace, total,
                    ))
                }
                (zbus::message::Type::Signal, Some("UnitFilesChanged")) => {
                    Some(SystemdSignal::UnitFilesChanged)
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
