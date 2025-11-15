use std::{error::Error, future::pending};
use tracing::info;
use zbus::{connection, interface};

struct Greeter {
    count: u64,
}

#[interface(name = "org.zbus.MyGreeter1")]
impl Greeter {
    // Can be `async` as well.
    fn say_hello(&mut self, name: &str) -> String {
        let id = unsafe { libc::getegid() };
        info!("id {}", id);
        self.count += 1;
        format!("Hello {}! I have been called {} times.", name, self.count)
    }
}

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
