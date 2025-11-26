use clap::{Parser, Subcommand};
use std::{env, error::Error, future::pending, path::PathBuf};
use sysd_manager_proxy_lib::SysDManagerProxy;
use tokio::process::Command;
use tracing::{error, info};
use zbus::connection;

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
}

// Although we use `tokio` here, you can use any async runtime of choice.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt().init();

    let args = Args::parse();

    let result = match args.cmd {
        Some(CommandArg::Install) => install().await,
        Some(CommandArg::Serve) => serve_proxy().await,
        None => serve_proxy().await,
    };

    if let Err(error) = result {
        error!("{:?}", error);
    }

    Ok(())
}

async fn install() -> Result<(), Box<dyn Error>> {
    info!("Install proxy");
    let path = env::current_dir()?;
    info!("The current directory is {}", path.display());

    let mut src = PathBuf::from("data");
    src.push("io.github.plrigaux.SysDManager.conf");

    let dst = PathBuf::from("/usr/share/dbus-1/system.d");
    //   dst.push("io.github.plrigaux.sysd-manager.conf");

    info!("Copying {} --> {}", src.display(), dst.display());
    //fs::copy(src, dst).await?;

    let x = Command::new("sudo")
        .arg("install")
        .arg("-v")
        .arg("-Dm644")
        .arg(src)
        .arg("-t")
        .arg(dst)
        .output()
        .await?;

    if x.status.success() {
        for l in String::from_utf8_lossy(&x.stdout).lines() {
            info!("{l}");
        }
    }

    Ok(())
}

async fn serve_proxy() -> Result<(), Box<dyn Error>> {
    let id = unsafe { libc::getegid() };
    info!("User id {id}");
    let greeter = SysDManagerProxy { count: 0 };
    let _conn = connection::Builder::system()?
        .name("io.github.plrigaux.SysDManager")?
        .serve_at("/io/github/plrigaux/SysDManager", greeter)?
        .build()
        .await?;

    // Do other things or go to wait forever
    pending::<()>().await;

    Ok(())
}
