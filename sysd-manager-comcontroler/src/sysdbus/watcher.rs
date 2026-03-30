use crate::{
    SystemdSignal, SystemdSignalRow,
    errors::SystemdErrors,
    runtime,
    sysdbus::{
        dbus_proxies::{
            JobNewArgs, JobNewStream, JobRemovedArgs, JobRemovedStream, ReloadingArgs,
            ReloadingStream, StartupFinishedArgs, StartupFinishedStream, Systemd1ManagerProxy,
            UnitFilesChangedStream, UnitNewArgs, UnitNewStream, UnitRemovedArgs, UnitRemovedStream,
        },
        get_connection,
    },
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
        let (systemd_signal_sender, _) = broadcast::channel(50);

        let cancellation_token = tokio_util::sync::CancellationToken::new();

        // runtime().spawn(watch_systemd_signals(
        //     systemd_signal_sender.clone(),
        //     cancellation_token,
        // ));

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
                message.header().member().map(|m| m.as_str()),
            ) {
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
                (zbus::message::Type::Signal, _) => {
                    info!("Signal {message:?}");

                    let h = message.header();
                    if let Some(m) = h.member() {
                        m.as_str();
                        println!("{:?}", m)
                    }
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
                warn!("Send signal Error {error:?}")
            };
        }
    }

    Ok(())
}

async fn watch_systemd_signals(
    systemd_signal_sender: broadcast::Sender<SystemdSignalRow>,
    cancellation_token: tokio_util::sync::CancellationToken,
) -> Result<(), SystemdErrors> {
    let connection = get_connection(UnitDBusLevel::System).await?;

    // `Systemd1ManagerProxy` is generated from `Systemd1Manager` trait
    let systemd_proxy = Systemd1ManagerProxy::new(&connection).await?;
    // Method `receive_job_new` is generated from `job_new` signal
    let mut jobs_new_stream = systemd_proxy.receive_job_new().await?;
    let mut job_removed_stream = systemd_proxy.receive_job_removed().await?;
    let mut unit_new_stream = systemd_proxy.receive_unit_new().await?;
    let mut unit_removed_stream = systemd_proxy.receive_unit_removed().await?;
    let mut reloading_stream = systemd_proxy.receive_reloading().await?;
    let mut unit_files_changed_stream = systemd_proxy.receive_unit_files_changed().await?;
    let mut startup_finished_stream = systemd_proxy.receive_startup_finished().await?;

    if let Err(err) = systemd_proxy.subscribe().await {
        warn!("Subscribe error {:?}", err);
    };

    info!("Subscribe to signals");

    loop {
        let msg = tokio::select!(
            m = fn_job_new(&mut jobs_new_stream) => {m},
            m = fn_job_removed(&mut job_removed_stream) => {m},
            m = fn_unit_new(&mut unit_new_stream) => {m},
            m = unit_removed(&mut unit_removed_stream) => {m},
            m = reloading(&mut reloading_stream) => {m},
            m = startup_finished(&mut startup_finished_stream) => {m},
            m = unit_files_changed(&mut unit_files_changed_stream) => {m},
            _ = cancellation_token.cancelled() => {
                info!("Watch Systemd Signals Close");
                break;
            }
        );

        if let Some(signal) = msg {
            let signal_row = SystemdSignalRow::new(signal);

            if let Err(error) = systemd_signal_sender.send(signal_row) {
                warn!("Send signal Error {error:?}")
            };
        }
    }

    error!("Stream ended unexpectedly");
    Ok(())
    //unreachable!("Stream ended unexpectedly");
}

async fn fn_job_new(jobs_new_stream: &mut JobNewStream) -> Option<SystemdSignal> {
    if let Some(msg) = jobs_new_stream.next().await {
        // struct `JobNewArgs` is generated from `job_new` signal function arguments
        let args: JobNewArgs = msg.args().expect("Error parsing message");

        debug!(
            "JobNew received: unit={} id={} path={}",
            args.unit, args.id, args.job
        );
        Some(SystemdSignal::JobNew(args.id, args.job, args.unit))
    } else {
        None
    }
}

async fn fn_job_removed(job_removed_stream: &mut JobRemovedStream) -> Option<SystemdSignal> {
    if let Some(msg) = job_removed_stream.next().await {
        let args: JobRemovedArgs = msg.args().expect("Error parsing message");

        debug!(
            "JobRemoved received: unit={} id={} path={} result={}",
            args.unit, args.id, args.job, args.result
        );
        Some(SystemdSignal::JobRemoved(
            args.id,
            args.job,
            args.unit,
            args.result,
        ))
    } else {
        None
    }
}

async fn fn_unit_new(unit_new_stream: &mut UnitNewStream) -> Option<SystemdSignal> {
    if let Some(msg) = unit_new_stream.next().await {
        let args: UnitNewArgs = msg.args().expect("Error parsing message");
        debug!("UnitNew received: unit={} id={}", args.unit, args.id,);
        Some(SystemdSignal::UnitNew(args.id, args.unit))
    } else {
        None
    }
}

async fn unit_removed(unit_removed_stream: &mut UnitRemovedStream) -> Option<SystemdSignal> {
    if let Some(msg) = unit_removed_stream.next().await {
        let args: UnitRemovedArgs = msg.args().expect("Error parsing message");
        debug!("UnitRemoved received: unit={} id={}", args.unit, args.id,);
        Some(SystemdSignal::UnitRemoved(args.id, args.unit))
    } else {
        None
    }
}

async fn startup_finished(
    startup_finished_stream: &mut StartupFinishedStream,
) -> Option<SystemdSignal> {
    if let Some(msg) = startup_finished_stream.next().await {
        let args: StartupFinishedArgs = msg.args().expect("Error parsing message");

        debug!(
            "Startup Finished received: firmware={} loader={} kernel={} initrd={} userspace={} total={}",
            args.firmware, args.loader, args.kernel, args.initrd, args.userspace, args.total,
        );

        Some(SystemdSignal::StartupFinished(
            args.firmware,
            args.loader,
            args.kernel,
            args.initrd,
            args.userspace,
            args.total,
        ))
    } else {
        None
    }
}

async fn unit_files_changed(
    unit_files_changed_stream: &mut UnitFilesChangedStream,
) -> Option<SystemdSignal> {
    if unit_files_changed_stream.next().await.is_some() {
        debug!("UnitFilesChanged");

        Some(SystemdSignal::UnitFilesChanged)
    } else {
        None
    }
}

async fn reloading(reloading_stream: &mut ReloadingStream) -> Option<SystemdSignal> {
    if let Some(msg) = reloading_stream.next().await {
        let args: ReloadingArgs = msg.args().expect("Error parsing message");

        debug!("Reloading received: active={}", args.active);

        Some(SystemdSignal::Reloading(args.active))
    } else {
        None
    }
}
