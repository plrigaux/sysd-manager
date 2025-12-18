#[cfg(feature = "flatpak")]
extern crate gio;

use std::{
    collections::BTreeMap,
    env,
    error::Error,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use base::{
    RunMode, args,
    consts::*,
    file::{commander, flatpak_host_file_path},
};

#[cfg(feature = "flatpak")]
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
    for (key, value) in std::env::vars() {
        println!("{}: {}", key, value);
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

    let (interface, destination) = if run_mode == RunMode::Development {
        (DBUS_INTERFACE, DBUS_DESTINATION_DEV)
    } else {
        (DBUS_INTERFACE, DBUS_DESTINATION)
    };

    let base_path = if cfg!(feature = "flatpak") {
        PathBuf::from("/io/github/plrigaux/sysd-manager")
    } else {
        let mut src_sysd_path = normalized_path.join("sysd-manager-proxy");
        src_sysd_path.push("data");
        src_sysd_path
    };

    let mut map = BTreeMap::new();

    map.insert("BUS_NAME", run_mode.bus_name());
    map.insert("DESTINATION", destination);
    map.insert("INTERFACE", interface);
    map.insert("ENVIRONMENT", "");

    let exec = match run_mode {
        RunMode::Normal => {
            //  cmd = ["flatpak", "run", APP_ID]
            #[cfg(feature = "flatpak")]
            {
                format!(
                    "/usr/bin/flatpak --system-talk-name=org.freedesktop.PolicyKit1 --system-own-name={} run {} proxy",
                    DBUS_NAME_FLATPAK, APP_ID
                )
            }

            #[cfg(not(feature = "flatpak"))]
            {
                const BIN_DIR: &str = "/usr/bin";
                const BIN_NAME: &str = "sysd-manager-proxy";
                String::from_iter([BIN_DIR, "/", BIN_NAME])
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

    let src = source_path(&base_path, DBUSCONF_FILE)?;
    let mut dst = PathBuf::from_iter(args!(
        flatpak_host_file_path(SYSTEMD_DIR),
        run_mode.bus_name()
    ));
    dst.add_extension("conf");

    let mut content = String::new();
    install_file(&src, &dst, false, &mut content).await?;
    install_edit_file(&map, dst, &mut content).await?;

    info!("Installing Polkit Policy");
    let src = source_path(&base_path, POLICY_FILE)?;
    let dst = flatpak_host_file_path(ACTION_DIR);
    install_file(&src, &dst, true, &mut content).await?;

    info!("Installing Service");

    let src = source_path(&base_path, SERVICE_FILE)?;
    let service_file_path = PathBuf::from_iter(args![
        flatpak_host_file_path(SERVICE_DIR),
        run_mode.proxy_service_name()
    ]);

    map.insert("EXECUTABLE", &exec);
    map.insert("SERVICE_ID", run_mode.proxy_service_id());
    install_file(&src, &service_file_path, false, &mut content).await?;
    install_edit_file(&map, service_file_path, &mut content).await?;

    let script_file = create_script(&content).await?;

    content.push_str("echo End of script");

    let output = commander(args!(sudo(), "sh", script_file), None)
        .output()
        .await?;

    ouput_to_screen(output);
    Ok(())
}

fn source_path(base_path: &Path, file_name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let src_path = base_path.join(file_name);

    #[cfg(feature = "flatpak")]
    {
        use base::file::inside_flatpak;

        let stream = gio::functions::resources_open_stream(
            &src_path.to_string_lossy(),
            ResourceLookupFlags::NONE,
        )?;

        let path = PathBuf::from(format!("XXXXXX{}", POLICY_FILE));
        let (file, ios_stream) = gio::File::new_tmp(Some(&path)).unwrap();

        let mut tmp_path = file.path().ok_or(Box::<dyn Error>::from("No file path"))?;
        info!("temp file path {:?}", tmp_path);

        let os_strem = ios_stream.output_stream();
        os_strem
            .splice(
                &stream,
                OutputStreamSpliceFlags::NONE,
                None::<&gio::Cancellable>,
            )
            .unwrap();

        /*         /run/user/1000/.flatpak/io.github.plrigaux.sysd-manager/tmp
        /run/user/USERID/.flatpak/FLATPAK_ID/tmp/ */

        if inside_flatpak()
            && let Ok(run_time_dir) = env::var("XDG_RUNTIME_DIR")
            && let Ok(flatpak_id) = env::var("FLATPAK_ID")
        {
            tmp_path = PathBuf::from_iter(args![
                run_time_dir,
                ".flatpak",
                flatpak_id,
                tmp_path.strip_prefix("/").expect("tmp_path not empty")
            ]);
            debug!("flatpack tmp dir {}", tmp_path.display());
        }

        Ok(tmp_path)
    }

    #[cfg(not(feature = "flatpak"))]
    Ok(src_path)
}

async fn install_edit_file(
    map: &BTreeMap<&str, &str>,
    dst: PathBuf,
    content: &mut String,
) -> Result<(), Box<dyn Error + 'static>> {
    info!("Edit file -- {}", dst.display());

    let mut s = vec!["sed".to_string(), "-i".to_string()];

    //let mut command = commander(args!(sudo(), "sed", "-i"), None);

    for (k, v) in map {
        s.push("-e".to_string());
        s.push(format!(
            "s/{{{k}}}/{}/",
            v.replace("/", r"\\/").replace(" ", r"\ ")
        ));
        // command.args(args!("-e", );
    }
    s.push(dst.to_string_lossy().to_string());

    //command.arg(dst);

    content.push_str(&s.join(" "));
    content.push('\n');
    /*  let output = command.output().await?;

    ouput_to_screen(output); */
    Ok(())
}

async fn install_file(
    src: &Path,
    dst: &Path,
    dst_is_dir: bool,
    content: &mut String,
) -> Result<(), Box<dyn Error + 'static>> {
    install_file_mode(src, dst, dst_is_dir, "644", content).await
}

fn sudo() -> &'static str {
    #[cfg(feature = "flatpak")]
    {
        "pkexec"
    }

    #[cfg(not(feature = "flatpak"))]
    {
        "sudo"
    }
}

async fn install_file_mode(
    src: &Path,
    dst: &Path,
    dst_is_dir: bool,
    mode: &str,
    //  map: Option<&BTreeMap<&str, &str>>,
    content: &mut String,
) -> Result<(), Box<dyn Error + 'static>> {
    info!(
        "Installing {} --> {} with mode {}",
        src.display(),
        dst.display(),
        mode
    );

    let dir_arg = if dst_is_dir { "-t" } else { "-T" };

    /*     let mut command = commander(
           args!(
               sudo(),
               "install",
               format!("-vDm{}", mode),
               src,
               dir_arg,
               dst
           ),
           None,
       );
    */
    let s = [
        //     sudo(),
        "install",
        &format!("-vDm{}", mode),
        &src.to_string_lossy(),
        dir_arg,
        &dst.to_string_lossy(),
    ]
    .join(" ");

    content.push_str(&s);
    content.push('\n');

    /*     if let Some(map) = map {
        command.args(["&&", "sed", "-i"]);
        for (k, v) in map {
            command.args(args!("-e", format!("s/{{{k}}}/{}/", v.replace("/", r"\/"))));
        }
        command.arg(dst);
    }

    let output = command.output().await?;
    ouput_to_screen(output); */
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

use tokio::io::AsyncWriteExt;
async fn create_script(content: &str) -> Result<PathBuf, std::io::Error> {
    let mut file_path = env::temp_dir();

    file_path.push("sysd-manager-install.sh");

    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(&file_path)
        .await?;

    file.write_all(b"#!/bin/bash\n\n").await?;

    file.write_all(content.as_bytes()).await?;

    info!("Script created to {}", file_path.display());

    Ok(file_path)
}

fn ouput_to_screen(output: std::process::Output) {
    if output.status.success() {
        for l in String::from_utf8_lossy(&output.stdout).lines() {
            info!("{l}");
        }
    } else {
        warn!("Exit code {:?}", output.status.code());
        for line in String::from_utf8_lossy(&output.stderr).lines() {
            warn!("{line}");
        }
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn test_string() {
        let k = "A";
        let v = "B";
        let x = format!(r#"s/{{{k}}}/{}/"#, v.replace("/", r"\/"));

        assert_eq!(x, "s/{A}/B/")
    }
}
