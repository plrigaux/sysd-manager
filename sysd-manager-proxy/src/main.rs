use std::{error::Error, future::pending};
use sysd_manager_proxy_lib::Greeter;
use tracing::info;
use zbus::connection;

// Although we use `tokio` here, you can use any async runtime of choice.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt().init();

    let id = unsafe { libc::getegid() };
    info!("User id {id}");
    let greeter = Greeter { count: 0 };
    let _conn = connection::Builder::system()?
        .name("org.zbus.MyGreeter")?
        .serve_at("/org/zbus/MyGreeter", greeter)?
        .build()
        .await?;

    // Do other things or go to wait forever
    pending::<()>().await;

    Ok(())
}
