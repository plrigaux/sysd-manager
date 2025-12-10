//#[cfg(feature = "flatpak")]
extern crate gio;

use std::{
    collections::BTreeMap,
    env,
    error::Error,
    path::{Path, PathBuf},
};

use base::{RunMode, consts::*};

use gio::{
    OutputStreamSpliceFlags, ResourceLookupFlags,
    prelude::{FileExt, IOStreamExt, OutputStreamExt},
};

use tokio::{fs, process::Command};
use tracing::{debug, error, info, warn};

const SYSTEMD_DIR: &str = "/usr/share/dbus-1/system.d";
const ACTION_DIR: &str = "/usr/share/polkit-1/actions";
const SERVICE_DIR: &str = "/usr/lib/systemd/system";
const POLICY_FILE: &str = "io.github.plrigaux.SysDManager.policy";
const SERVICE_FILE: &str = "sysd-manager-proxy.service";
const DBUSCONF_FILE: &str = "io.github.plrigaux.SysDManager.conf";

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
    #[cfg(feature = "flatpak")]
    if let Err(e) = gio::resources_register_include!("sysd-manager-proxy.gresource") {
        warn!("Failed to register resources. Error: {e:?}");
    }

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

    let base_path = if cfg!(feature = "flatpak") {
        PathBuf::from("/io/github/plrigaux/sysd-manager")
    } else {
        let src_sysd_path = normalized_path.join("sysd-manager-proxy");
        src_sysd_path.join("data")
    };

    let src = source_path(&base_path, DBUSCONF_FILE)?;
    let mut dst = PathBuf::from(SYSTEMD_DIR).join(bus_name);
    dst.add_extension("conf");
    install_file(&src, &dst, false).await?;

    let mut map = BTreeMap::new();

    map.insert("BUS_NAME", bus_name);
    map.insert("DESTINATION", destination);
    map.insert("INTERFACE", interface);
    map.insert("ENVIRONMENT", "");

    install_edit_file(&map, dst).await?;

    info!("Installing Polkit Policy");
    let src = source_path(&base_path, POLICY_FILE)?;
    let dst = PathBuf::from(ACTION_DIR);
    install_file(&src, &dst, true).await?;

    info!("Installing Service");

    let src = source_path(&base_path, SERVICE_FILE)?;
    let mut service_file_path = PathBuf::from(SERVICE_DIR).join(service_id);
    service_file_path.add_extension("service");

    install_file(&src, &service_file_path, false).await?;

    let exec = match run_mode {
        RunMode::Normal => {
            //  cmd = ["flatpak", "run", APP_ID]
            #[cfg(feature = "flatpak")]
            {
                format!("/usr/bin/flatpak run {} proxy", APP_ID)
            }

            #[cfg(not(feature = "flatpak"))]
            {
                const BIN_DIR: &str = "/usr/bin";
                const BIN_NAME: &str = "sysd-manager-proxy";
                let dst = PathBuf::from(BIN_DIR).join(BIN_NAME);
                let s = dst.to_string_lossy();
                s.into_owned()
            }
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

    install_edit_file(&map, service_file_path).await?;

    Ok(())
}

fn source_path(base_path: &Path, file_name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let src_path = base_path.join(file_name);

    #[cfg(feature = "flatpak")]
    {
        let stream = gio::functions::resources_open_stream(
            &src_path.to_string_lossy(),
            ResourceLookupFlags::NONE,
        )?;

        let path = PathBuf::from(format!("XXXXXX{}", POLICY_FILE));
        let (file, ios_stream) = gio::File::new_tmp(Some(&path)).unwrap();

        let tmp_path = file.path().ok_or(Box::<dyn Error>::from("No file path"))?;
        info!("temp file path {:?}", tmp_path);

        let os_strem = ios_stream.output_stream();
        os_strem
            .splice(
                &stream,
                OutputStreamSpliceFlags::NONE,
                None::<&gio::Cancellable>,
            )
            .unwrap();

        Ok(tmp_path)
    }

    #[cfg(not(feature = "flatpak"))]
    Ok(src_path)
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

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use crate::install::gio::prelude::FileExt;
    use gio::OutputStreamSpliceFlags;
    use gio::ResourceLookupFlags;
    use gio::prelude::IOStreamExt;
    use gio::prelude::OutputStreamExt;
    use log::info;
    use test_base::init_logs;

    #[test]
    fn test_getresource_data() {
        init_logs();
        gio::resources_register_include!("sysd-manager-proxy.gresource").unwrap();

        let stream = gio::functions::resources_open_stream(
            "/io/github/plrigaux/sysd-manager/io.github.plrigaux.SysDManager.conf",
            ResourceLookupFlags::NONE,
        )
        .unwrap();

        let path = PathBuf::from("XXXXXXvalue.txt");
        let (file, ios_stream) = gio::File::new_tmp(Some(&path)).unwrap();

        info!("fp {:?}", file.path());

        let os_strem = ios_stream.output_stream();
        os_strem
            .splice(
                &stream,
                OutputStreamSpliceFlags::NONE,
                None::<&gio::Cancellable>,
            )
            .unwrap();
    }
}
