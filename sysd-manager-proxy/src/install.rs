use std::{collections::BTreeMap, env, error::Error, path::PathBuf};

use base::{PROXY_SERVICE, PROXY_SERVICE_DEV, RunMode};
use log::warn;
use sysd_manager_proxy_lib::consts::*;
use tokio::{fs, process::Command};
use tracing::info;

const SYSTEMD_DIR: &str = "/usr/share/dbus-1/system.d";
const ACTION_DIR: &str = "/usr/share/polkit-1/actions";
const SERVICE_DIR: &str = "/usr/lib/systemd/system";

pub async fn install(run_mode: RunMode) -> Result<(), Box<dyn Error>> {
    info!("Install proxy");
    let path = env::current_dir()?;
    info!("The current directory is {}", path.display());

    let (bus_name, interface, destination, service_id) = if run_mode == RunMode::Development {
        (
            DBUS_NAME_DEV,
            DBUS_INTERFACE,
            DBUS_DESTINATION_DEV,
            PROXY_SERVICE_DEV,
        )
    } else {
        (DBUS_NAME, DBUS_INTERFACE, DBUS_DESTINATION, PROXY_SERVICE)
    };

    let src_base = PathBuf::from("data");
    let src = src_base.join("io.github.plrigaux.SysDManager.conf");
    let mut dst = PathBuf::from(SYSTEMD_DIR).join(bus_name);
    dst.add_extension("conf");
    install_file(&src, &dst, false).await?;

    let mut map = BTreeMap::new();

    map.insert("BUS_NAME", bus_name);
    map.insert("DESTINATION", destination);
    map.insert("INTERFACE", interface);
    map.insert("ENVIRONMENT", "");

    install_edit_file(&map, dst).await?;

    let src = src_base.join("io.github.plrigaux.SysDManager.policy");
    let dst = PathBuf::from(ACTION_DIR);
    install_file(&src, &dst, true).await?;

    let src = src_base.join("sysd-manager-proxy.service");
    let mut dst = PathBuf::from(SERVICE_DIR).join(service_id);
    dst.add_extension("service");

    install_file(&src, &dst, false).await?;

    let exec = std::env::current_exe().expect("supposed to exist");
    let mut exec = exec.to_string_lossy();

    if run_mode == RunMode::Development {
        let cmd = format!("{} -d", exec);
        exec = std::borrow::Cow::Owned(cmd);
    }

    map.insert("EXECUTABLE", &exec);
    map.insert("SERVICE_ID", service_id);

    install_edit_file(&map, dst).await?;

    Ok(())
}

async fn install_edit_file(
    map: &BTreeMap<&str, &str>,
    dst: PathBuf,
) -> Result<(), Box<dyn Error + 'static>> {
    info!("Edit file -- {}", dst.display());

    let mut cmd = Command::new("sudo");
    cmd.arg("sed").arg("-i");

    for (k, v) in map {
        cmd.arg("-e");
        cmd.arg(format!("s/{{{k}}}/{}/", v.replace("/", r"\/")));
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

pub async fn clean(_run_mode: RunMode) -> Result<(), Box<dyn Error>> {
    info!("Clean proxy files");

    let mut path_to_clean = Vec::new();

    //TODO: use run_mode to clean only relevant files
    for dir in [SYSTEMD_DIR, ACTION_DIR] {
        let mut entries = fs::read_dir(dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(file_name) = path.file_name() {
                let fname = file_name.to_string_lossy();
                if fname.starts_with("io.github.plrigaux.SysDM")
                    || fname.starts_with("sysd-manager-proxy")
                {
                    path_to_clean.push(path);
                }
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
