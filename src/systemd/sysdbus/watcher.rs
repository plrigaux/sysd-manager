use zbus::proxy;
use zvariant::OwnedObjectPath;

use crate::systemd::{enums::UnitDBusLevel, errors::SystemdErrors, sysdbus::get_connection_async};
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

pub async fn watch_systemd_jobs() -> Result<(), SystemdErrors> {
    let connection = get_connection_async(UnitDBusLevel::System).await?;

    // `Systemd1ManagerProxy` is generated from `Systemd1Manager` trait
    let systemd_proxy = Systemd1ManagerProxy::new(&connection).await?;
    // Method `receive_job_new` is generated from `job_new` signal
    let mut jobs_new_stream = systemd_proxy.receive_job_new().await?;
    let mut job_removed_stream = systemd_proxy.receive_job_removed().await?;
    let mut unit_new_stream = systemd_proxy.receive_unit_new().await?;
    let mut unit_removed_stream = systemd_proxy.receive_unit_removed().await?;
    let mut reloading_stream = systemd_proxy.receive_reloading().await?;

    loop {
        tokio::select!(
            _ = fn_job_new(&mut jobs_new_stream) => {},
            _ = fn_job_removed(&mut job_removed_stream) => {},
            _ = fn_unit_new(&mut unit_new_stream) => {},
            _ = unit_removed(&mut unit_removed_stream) => {},
            _ = reloading(&mut reloading_stream) => {},
        );
    }

    //unreachable!("Stream ended unexpectedly");
}

async fn fn_job_new(jobs_new_stream: &mut JobNewStream) {
    if let Some(msg) = jobs_new_stream.next().await {
        // struct `JobNewArgs` is generated from `job_new` signal function arguments
        let args: JobNewArgs = msg.args().expect("Error parsing message");

        println!(
            "JobNew received: unit={} id={} path={}",
            args.unit, args.id, args.job
        );
    }
}

async fn fn_job_removed(job_removed_stream: &mut JobRemovedStream) {
    if let Some(msg) = job_removed_stream.next().await {
        let args: JobRemovedArgs = msg.args().expect("Error parsing message");

        println!(
            "JobRemoved received: unit={} id={} path={} result={}",
            args.unit, args.id, args.job, args.result
        );
    }
}

async fn fn_unit_new(unit_new_stream: &mut UnitNewStream) {
    if let Some(msg) = unit_new_stream.next().await {
        let args: UnitNewArgs = msg.args().expect("Error parsing message");

        println!("UnitNew received: unit={} id={}", args.unit, args.id,);
    }
}

async fn unit_removed(unit_removed_stream: &mut UnitRemovedStream) {
    if let Some(msg) = unit_removed_stream.next().await {
        let args: UnitRemovedArgs = msg.args().expect("Error parsing message");

        println!("UnitRemoved received: unit={} id={}", args.unit, args.id,);
    }
}

async fn reloading(reloading_stream: &mut ReloadingStream) {
    if let Some(msg) = reloading_stream.next().await {
        let args: ReloadingArgs = msg.args().expect("Error parsing message");

        println!("Reloading received: active={}", args.active);
    }
}
