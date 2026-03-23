mod install;
use base::{RunMode, consts::*};
use clap::{Parser, Subcommand};

use std::error::Error;
use sysd_manager_proxy_lib::{init_tracing, serve_proxy};

use futures_util::stream::TryStreamExt;
use tracing::{debug, error, info};
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
    #[arg(long, default_value_t = false)]
    normal: bool,

    /// No Heart Beat
    #[arg(short, long, default_value_t = false)]
    no_heart_beat: bool,
}

#[derive(Subcommand, Debug, Clone)]
enum CommandArg {
    /// Starts the Proxy
    Serve,
    Install,
    Clean,
    Test,
}

// Although we use `tokio` here, you can use any async runtime of choice.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    init_tracing();

    debug!("CLI Args {:?}", std::env::args_os());
    let args = Args::parse();

    let run_mode = run_mode(args.dev, args.normal);

    let heart_beat = !args.no_heart_beat;

    let result = match args.cmd {
        Some(CommandArg::Install) => install::install(run_mode).await,
        Some(CommandArg::Clean) => install::clean(run_mode).await,
        Some(CommandArg::Serve) => serve_proxy(run_mode, heart_beat).await,
        Some(CommandArg::Test) => test(run_mode).await,
        None => serve_proxy(run_mode, true).await,
    };

    if let Err(error) = result {
        error!("{:?}", error);
    }

    Ok(())
}

fn run_mode(dev: bool, normal: bool) -> RunMode {
    let run_mode = RunMode::from_flags(dev, normal);

    if run_mode == RunMode::Development {
        info!("Serve in Development Mode");
    } else {
        info!("Serve in Production Mode");
    }
    run_mode
}

async fn test(run_mode: RunMode) -> Result<(), Box<dyn Error>> {
    info!("TEST server");

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
