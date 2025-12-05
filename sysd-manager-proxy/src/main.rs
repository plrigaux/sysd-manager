mod install;
use base::{RunMode, consts::*};
use clap::{Parser, Subcommand};

use std::{error::Error, future::pending};
use sysd_manager_proxy_lib::init_connection;

use futures_util::stream::TryStreamExt;
use sysd_manager_proxy_lib::init_authority;
use tracing::{Level, debug, error, info};
use tracing_subscriber::fmt;
use zbus::Connection;

/// General purpose greet/farewell messaging.
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    cmd: Option<CommandArg>,

    /// Development mode
    #[arg(short, long, default_value_t = false)]
    dev: bool,

    /// Normal mode
    #[arg(short, long, default_value_t = false)]
    normal: bool,
}

#[derive(Subcommand, Debug, Clone)]
enum CommandArg {
    Serve,
    Install,
    Clean,
    Test,
}

// Although we use `tokio` here, you can use any async runtime of choice.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let timer = fmt::time::ChronoLocal::new("%Y-%m-%d %H:%M:%S%.3f".to_owned());
    //let timer = fmt::time::ChronoLocal::rfc_3339();

    tracing_subscriber::fmt()
        .with_timer(timer)
        .with_max_level(Level::DEBUG)
        .with_line_number(true)
        .init();
    //tracing_subscriber::fmt().init();

    debug!("Args {:?}", std::env::args_os());
    let args = Args::parse();

    let run_mode = RunMode::from_flags(args.dev, args.normal);

    if run_mode == RunMode::Development {
        info!("Serve in Development Mode");
    } else {
        info!("Serve in Production Mode");
    }

    let result = match args.cmd {
        Some(CommandArg::Install) => install::install(run_mode).await,
        Some(CommandArg::Clean) => install::clean(run_mode).await,
        Some(CommandArg::Serve) => serve_proxy(run_mode).await,
        Some(CommandArg::Test) => test(run_mode).await,
        None => serve_proxy(run_mode).await,
    };

    if let Err(error) = result {
        error!("{:?}", error);
    }

    Ok(())
}

async fn serve_proxy(run_mode: RunMode) -> Result<(), Box<dyn Error>> {
    init_authority().await?;
    init_connection(run_mode).await?;

    // Do other things or go to wait forever
    pending::<()>().await;

    Ok(())
}

async fn test(run_mode: RunMode) -> Result<(), Box<dyn Error>> {
    info!("TEST server");
    debug!("TEST server");
    let (default_name, _default_path) = if run_mode == RunMode::Development {
        (DBUS_NAME_DEV, DBUS_PATH_DEV)
    } else {
        (DBUS_NAME, DBUS_PATH)
    };
    let connection = Connection::session().await?;
    let mut stream = zbus::MessageStream::from(&connection);
    connection.request_name(default_name).await?;

    while let Some(msg) = stream.try_next().await? {
        let msg_header = msg.header();
        debug!("MH {:?}", msg_header);

        match msg_header.message_type() {
            zbus::message::Type::MethodCall => {
                // real code would check msg_header path(), interface() and member()
                // handle invalid calls, introspection, errors etc
                let header = msg.header();

                let dest = header.destination();

                info!("destination {:?}", dest);

                let sender = header.sender();

                info!("destination {:?}", sender);
                //let arg: &str = body.deserialize()?;

                connection
                    .reply(&header, &(format!("Hello {}!", "arg")))
                    .await?;

                break;
            }
            _ => continue,
        }
    }

    Ok(())
}
