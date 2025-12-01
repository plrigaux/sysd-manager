use std::{collections::BTreeMap, env, error::Error, path::PathBuf};

use base::{RunMode, consts::*};

use tokio::{fs, process::Command};
use tracing::{debug, error, info, warn};

const SYSTEMD_DIR: &str = "/usr/share/dbus-1/system.d";
const ACTION_DIR: &str = "/usr/share/polkit-1/actions";
const SERVICE_DIR: &str = "/usr/lib/systemd/system";
const BIN_DIR: &str = "/usr/bin";
const BIN_NAME: &str = "sysd-manager-proxy";

pub async fn install(run_mode: RunMode) -> Result<(), Box<dyn Error>> {
    info!("Install proxy mode {:?}", run_mode);

    if run_mode == RunMode::Both {
        self::sub_install(RunMode::Development).await?;
        self::sub_install(RunMode::Normal).await?;
    } else {
        self::sub_install(run_mode).await?;
    }
    Ok(())
}

async fn sub_install(run_mode: RunMode) -> Result<(), Box<dyn Error>> {
    if run_mode == RunMode::Both {
        error!("sub_install should not be called with RunMode::Both");
        return Err("Invalid RunMode::Both for sub_install".into());
    }
    let path = env::current_dir()?;
    info!("The current directory is {}", path.display());

    let mut normalized_path = PathBuf::new();
    for token in path.iter() {
        normalized_path.push(token);
        if token == "sysd-manager" {
            break;
        } else {
            debug!("{:?}", token)
        }
    }

    info!("The base directory is {}", normalized_path.display());

    /*     if run_mode == RunMode::Normal {
        let src = PathBuf::from("../target/release").join(BIN_NAME);
        if !src.exists() {
            let msg = format!(
                "Binary file {} does not exist. Did you build the project in release mode?",
                src.display()
            );
            return Err(msg.into());
        }
        let dst = PathBuf::from(BIN_DIR).join(BIN_NAME);
        install_file_exec(&src, &dst, false).await?;
    } */

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

    let src_sysd_path = normalized_path.join("sysd-manager-proxy");
    let src_data = src_sysd_path.join("data");
    let src = src_data.join("io.github.plrigaux.SysDManager.conf");
    let mut dst = PathBuf::from(SYSTEMD_DIR).join(bus_name);
    dst.add_extension("conf");
    install_file(&src, &dst, false).await?;

    let mut map = BTreeMap::new();

    map.insert("BUS_NAME", bus_name);
    map.insert("DESTINATION", destination);
    map.insert("INTERFACE", interface);
    map.insert("ENVIRONMENT", "");

    install_edit_file(&map, dst).await?;

    let src = src_data.join("io.github.plrigaux.SysDManager.policy");
    let dst = PathBuf::from(ACTION_DIR);
    install_file(&src, &dst, true).await?;

    let src = src_data.join("sysd-manager-proxy.service");
    let mut dst = PathBuf::from(SERVICE_DIR).join(service_id);
    dst.add_extension("service");

    install_file(&src, &dst, false).await?;

    let exec = match run_mode {
        RunMode::Normal => {
            let dst = PathBuf::from(BIN_DIR).join(BIN_NAME);
            let s = dst.to_string_lossy();
            s.into_owned()
        }
        RunMode::Development => {
            let exec = std::env::current_exe().expect("supposed to exist");
            let exec = exec.to_string_lossy();

            format!("{} -d", exec)
        }
        _ => {
            return Err("Invalid RunMode::Both for sub_install".into());
        }
    };

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
    install_file_mode(src, dst, dst_is_dir, "644").await
}

#[allow(dead_code)]
async fn install_file_exec(
    src: &PathBuf,
    dst: &PathBuf,
    dst_is_dir: bool,
) -> Result<(), Box<dyn Error + 'static>> {
    install_file_mode(src, dst, dst_is_dir, "755").await
}

async fn install_file_mode(
    src: &PathBuf,
    dst: &PathBuf,
    dst_is_dir: bool,
    mode: &str,
) -> Result<(), Box<dyn Error + 'static>> {
    info!(
        "Installing {} --> {} with mode {}",
        src.display(),
        dst.display(),
        mode
    );

    let dir_arg = if dst_is_dir { "-t" } else { "-T" };

    let output = Command::new("sudo")
        .arg("install")
        .arg(format!("-vDm{}", mode))
        .arg(src)
        .arg(dir_arg)
        .arg(dst)
        .output()
        .await?;
    ouput_to_screen(output);
    Ok(())
}

enum Pattern {
    Equals(String),
    Start(String),
}

struct Clean {
    dir: String,
    patterns: Vec<Pattern>,
}

pub async fn clean(_run_mode: RunMode) -> Result<(), Box<dyn Error>> {
    info!("Clean proxy files");

    let mut to_clean = Vec::new();
    let clean = Clean {
        dir: SYSTEMD_DIR.to_string(),
        patterns: vec![Pattern::Start("io.github.plrigaux.SysDM".to_string())],
    };

    to_clean.push(clean);

    let clean = Clean {
        dir: ACTION_DIR.to_string(),
        patterns: vec![Pattern::Equals(
            "io.github.plrigaux.SysDManager.policy".to_string(),
        )],
    };

    to_clean.push(clean);

    let clean = Clean {
        dir: SERVICE_DIR.to_string(),
        patterns: vec![Pattern::Start("sysd-manager-proxy".to_string())],
    };
    to_clean.push(clean);

    let mut paths_to_clean = Vec::new();
    //TODO: use run_mode to clean only relevant files
    for clean in to_clean {
        let mut entries = fs::read_dir(clean.dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            for pattern in &clean.patterns {
                match pattern {
                    Pattern::Equals(s) => {
                        if let Some(file_name) = path.file_name() {
                            let fname = file_name.to_string_lossy();
                            if fname == *s {
                                paths_to_clean.push(path.clone());
                            }
                        }
                    }
                    Pattern::Start(s) => {
                        if let Some(file_name) = path.file_name() {
                            let fname = file_name.to_string_lossy();
                            if fname.starts_with(s) {
                                paths_to_clean.push(path.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    info!("{} file to clean", paths_to_clean.len());
    for path in paths_to_clean {
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
