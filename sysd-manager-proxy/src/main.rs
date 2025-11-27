mod install;
use clap::{Parser, Subcommand};
use std::{error::Error, future::pending};
use sysd_manager_proxy_lib::SysDManagerProxy;
use sysd_manager_proxy_lib::auth;
use sysd_manager_proxy_lib::init_authority;
use tracing::{error, info};
use zbus::connection;
use zbus_polkit::policykit1::*;

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
    tracing_subscriber::fmt().init();

    let args = Args::parse();

    let result = match args.cmd {
        Some(CommandArg::Install) => install::install().await,
        Some(CommandArg::Clean) => install::clean().await,
        Some(CommandArg::Serve) => serve_proxy().await,
        None => serve_proxy().await,
    };

    if let Err(error) = result {
        error!("{:?}", error);
    }

    Ok(())
}

async fn serve_proxy() -> Result<(), Box<dyn Error>> {
    let id = unsafe { libc::getegid() };

    init_authority().await?;
    let auth = auth();
    let proxy = SysDManagerProxy::new()?;
    let result = auth
        .check_authorization(
            &proxy.subject,
            "io.github.plrigaux.SysDManager",
            &std::collections::HashMap::new(),
            CheckAuthorizationFlags::AllowUserInteraction.into(),
            "",
        )
        .await?;

    info!("Polkit {result:?}");

    info!("User id {id}");
    //let greeter = SysDManagerProxy { count: 0 };
    let _conn = connection::Builder::system()?
        .name("io.github.plrigaux.SysDManager")?
        .serve_at("/io/github/plrigaux/SysDManager", proxy)?
        .build()
        .await?;

    // Do other things or go to wait forever
    pending::<()>().await;

    Ok(())
}
