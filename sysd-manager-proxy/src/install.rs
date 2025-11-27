use std::{env, error::Error, path::PathBuf};

use log::warn;
use tokio::{fs, process::Command};
use tracing::info;

const SYSTEMD_DIR: &str = "/usr/share/dbus-1/system.d";
const ACTION_DIR: &str = "/usr/share/polkit-1/actions";

pub async fn install() -> Result<(), Box<dyn Error>> {
    info!("Install proxy");
    let path = env::current_dir()?;
    info!("The current directory is {}", path.display());

    let src_base = PathBuf::from("data");
    let src = src_base.join("io.github.plrigaux.SysDManager.conf");
    let dst = PathBuf::from(SYSTEMD_DIR);
    install_file(src, dst).await?;

    let src = src_base.join("io.github.plrigaux.SysDManager.policy");
    let dst = PathBuf::from(ACTION_DIR);
    install_file(src, dst).await?;

    Ok(())
}

async fn install_file(src: PathBuf, dst: PathBuf) -> Result<(), Box<dyn Error + 'static>> {
    info!("Copying {} --> {}", src.display(), dst.display());

    let output = Command::new("sudo")
        .arg("install")
        .arg("-v")
        .arg("-Dm644")
        .arg(src)
        .arg("-t")
        .arg(dst)
        .output()
        .await?;
    ouput_to_screen(output);
    Ok(())
}

pub async fn clean() -> Result<(), Box<dyn Error>> {
    info!("Clean proxy files");

    let mut path_to_clean = Vec::new();

    for dir in [SYSTEMD_DIR, ACTION_DIR] {
        let mut entries = fs::read_dir(dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let p = entry.path();
            if let Some(file_name) = p.file_name()
                && file_name
                    .to_string_lossy()
                    .starts_with("io.github.plrigaux.")
            {
                path_to_clean.push(p);
            }
        }
    }

    info!("{} file to clean", path_to_clean.len());
    for path in path_to_clean {
        let output = Command::new("sudo")
            .arg("rm")
            .arg("-v")
            .arg(path)
            .output()
            .await?;

        ouput_to_screen(output);
    }
    Ok(())
}

fn ouput_to_screen(x: std::process::Output) {
    if x.status.success() {
        for l in String::from_utf8_lossy(&x.stdout).lines() {
            info!("{l}");
        }
    } else {
        warn!("Exit code {:?}", x.status.code());
        for l in String::from_utf8_lossy(&x.stderr).lines() {
            warn!("{l}");
        }
    }
}
