use std::{collections::BTreeMap, env, error::Error, path::PathBuf};

use log::warn;
use tokio::{fs, process::Command};
use tracing::info;

const SYSTEMD_DIR: &str = "/usr/share/dbus-1/system.d";
const ACTION_DIR: &str = "/usr/share/polkit-1/actions";
const SERVICE_DIR: &str = "/usr/lib/systemd/system";

pub async fn install() -> Result<(), Box<dyn Error>> {
    info!("Install proxy");
    let path = env::current_dir()?;
    info!("The current directory is {}", path.display());

    let src_base = PathBuf::from("data");
    let src = src_base.join("io.github.plrigaux.SysDManager.conf");
    let dst = PathBuf::from(SYSTEMD_DIR);
    install_file(&src, &dst, true).await?;

    let src = src_base.join("io.github.plrigaux.SysDManager.policy");
    let dst = PathBuf::from(ACTION_DIR);
    install_file(&src, &dst, true).await?;

    let src = src_base.join("sysd-manager-proxy.service");
    let dst = PathBuf::from(SERVICE_DIR).join("sysd-manager-proxy-dev.service");

    install_file(&src, &dst, false).await?;

    let mut map = BTreeMap::new();

    let exec = std::env::current_exe().expect("suppose to exist");
    let exec = exec.to_string_lossy().to_string();
    map.insert("EXECUTABLE", exec);
    map.insert("SERVICE_ID", "sysd-manager-proxy-dev".to_string());

    install_edit_file(&map, dst).await?;

    Ok(())
}

async fn install_edit_file(
    map: &BTreeMap<&str, String>,
    dst: PathBuf,
) -> Result<(), Box<dyn Error + 'static>> {
    info!("Edit file -- {}", dst.display());

    let mut cmd = Command::new("sudo");
    cmd.arg("sed").arg("-i");

    for (k, v) in map {
        cmd.arg("-e");
        cmd.arg(format!("s/{k}/{}/", v.replace("/", r"\/")));
    }

    let output = cmd.arg(dst).output().await?;
    ouput_to_screen(output);
    Ok(())
}

async fn install_file(
    src: &PathBuf,
    dst: &PathBuf,
    dst_is_dir: bool,
) -> Result<(), Box<dyn Error + 'static>> {
    info!("Copying {} --> {}", src.display(), dst.display());

    let dir_arg = if dst_is_dir { "-t" } else { "-T" };

    let output = Command::new("sudo")
        .arg("install")
        .arg("-v")
        .arg("-Dm644")
        .arg(src)
        .arg(dir_arg)
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
