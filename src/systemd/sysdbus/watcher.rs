use log::{debug, info, warn};
use tokio::sync::mpsc;
use zbus::proxy;
use zvariant::OwnedObjectPath;

use crate::systemd::{
    SystemdSignal, SystemdSignalRow, enums::UnitDBusLevel, errors::SystemdErrors,
    sysdbus::get_connection,
};
use futures_util::stream::StreamExt;

#[proxy(
    default_service = "org.freedesktop.systemd1",
    default_path = "/org/freedesktop/systemd1",
    interface = "org.freedesktop.systemd1.Manager"
)]
trait Systemd1Manager {
    // Defines signature for D-Bus signal named `JobNew`
    #[zbus(signal)]
    fn unit_new(&self, id: String, unit: OwnedObjectPath) -> zbus::Result<()>;

    #[zbus(signal)]
    fn unit_removed(&self, id: String, unit: OwnedObjectPath) -> zbus::Result<()>;

    // Defines signature for D-Bus signal named `JobNew`
    #[zbus(signal)]
    fn job_new(&self, id: u32, job: OwnedObjectPath, unit: String) -> zbus::Result<()>;

    #[zbus(signal)]
    fn job_removed(
        &self,
        id: u32,
        job: OwnedObjectPath,
        unit: String,
        result: String,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    fn startup_finished(
        &self,
        firmware: u64,
        loader: u64,
        kernel: u64,
        initrd: u64,
        userspace: u64,
        total: u64,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    fn unit_files_changed(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    fn reloading(&self, active: bool) -> zbus::Result<()>;
}

pub async fn watch_systemd_signals(
    systemd_signal_sender: mpsc::Sender<SystemdSignalRow>,
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
                return Ok(());
            }
        );

        if let Some(signal) = msg {
            let signal_row = SystemdSignalRow::new(signal);

            if let Err(error) = systemd_signal_sender.send(signal_row).await {
                warn!("Send signal Error {error:?}")
            };
        }
    }

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
