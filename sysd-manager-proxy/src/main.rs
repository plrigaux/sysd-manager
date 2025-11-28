mod consts;
mod install;
use clap::{Parser, Subcommand};
use std::borrow::Cow;
use std::env;
use std::{error::Error, future::pending};
use sysd_manager_proxy_lib::SysDManagerProxy;
use sysd_manager_proxy_lib::init_authority;
use tracing::{debug, error, info};
use tracing_subscriber::fmt;
use zbus::connection;

use crate::consts::{DBUS_NAME, DBUS_NAME_DEV, DBUS_PATH, DBUS_PATH_DEV};

/// General purpose greet/farewell messaging.
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    cmd: Option<CommandArg>,
}

#[derive(Subcommand, Debug, Clone)]
enum CommandArg {
    Serve,
    Install,
    Clean,
}

// Although we use `tokio` here, you can use any async runtime of choice.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let timer = fmt::time::ChronoLocal::new("%Y-%m-%d %H:%M:%S%.3f".to_owned());
    //let timer = fmt::time::ChronoLocal::rfc_3339();

    tracing_subscriber::fmt().with_timer(timer).init();
    //tracing_subscriber::fmt().init();

    let args = Args::parse();

    let is_dev = if env::var("CARGO").is_ok() {
        info!("The program is being run by cargo. --> Assume DEV");
        true
    } else {
        false
    };

    let result = match args.cmd {
        Some(CommandArg::Install) => install::install(is_dev).await,
        Some(CommandArg::Clean) => install::clean().await,
        Some(CommandArg::Serve) => serve_proxy(is_dev).await,
        None => serve_proxy(is_dev).await,
    };

    if let Err(error) = result {
        error!("{:?}", error);
    }

    Ok(())
}

async fn serve_proxy(is_dev: bool) -> Result<(), Box<dyn Error>> {
    init_authority().await?;
    /*  let auth = auth(); */
    let proxy = SysDManagerProxy::new()?;

    let id = unsafe { libc::getegid() };
    info!("User id {id}");

    let (default_name, default_path) = if is_dev {
        (DBUS_NAME_DEV, DBUS_PATH_DEV)
    } else {
        (DBUS_NAME, DBUS_PATH)
    };

    let dbus_name = get_env("DBUS_NAME", default_name);
    let dbus_path = get_env("DBUS_PATH", default_path);

    let _conn = connection::Builder::system()?
        .name(dbus_name)?
        .serve_at(dbus_path, proxy)?
        .build()
        .await?;

    // Do other things or go to wait forever
    pending::<()>().await;

    Ok(())
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
