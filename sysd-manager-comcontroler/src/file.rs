use std::{borrow::Cow, io::ErrorKind, process::Stdio};

use crate::{commander, errors::SystemdErrors};
use base::file::{create_drop_in_io, create_drop_in_path_file};
use log::{info, warn};
use std::io::Write;
use tokio::fs;
use tokio::io::AsyncWriteExt;

pub(crate) async fn create_drop_in(
    runtime: bool,
    unit_name: &str,
    file_name: &str,
    content: &str,
) -> Result<(), SystemdErrors> {
    let file_path = create_drop_in_path_file(unit_name, runtime, true, file_name)?;

    create_drop_in_io(&file_path, content, true).await?;

    Ok(())
}

/// To be able to acces the Flatpack mounted files.
/// Limit to /usr for the least access principle
pub fn flatpak_host_file_path(file_path: &str) -> Cow<'_, str> {
    if cfg!(feature = "flatpak") && (file_path.starts_with("/usr") || file_path.starts_with("/etc"))
    {
        Cow::from(format!("/run/host{file_path}"))
    } else {
        Cow::from(file_path)
    }
}

pub async fn save_text_to_file(file_path: &str, text: &str) -> Result<u64, SystemdErrors> {
    let host_file_path = flatpak_host_file_path(file_path);
    info!("Try to save content on File: {host_file_path}");
    match write_on_disk(text, &host_file_path).await {
        Ok(bytes_written) => Ok(bytes_written as u64),
        Err(error) => {
            if let SystemdErrors::IoError(ref err) = error {
                match err.kind() {
                    ErrorKind::PermissionDenied => {
                        info!("Some error : {err}, try executing command as another user");
                        write_with_priviledge(file_path, host_file_path, text)
                    }
                    _ => {
                        warn!("Unable to open file: {err:?}");
                        Err(error)
                    }
                }
            } else {
                Err(error)
            }
        }
    }
}

async fn write_on_disk(text: &str, file_path: &str) -> Result<usize, SystemdErrors> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(file_path)
        .await?;

    let test_bytes = text.as_bytes();
    file.write_all(test_bytes).await?;
    file.flush().await?;

    let bytes_written = test_bytes.len();
    info!("{bytes_written} bytes writen on File: {file_path}");
    Ok(bytes_written)
}

fn write_with_priviledge(
    file_path: &str,
    _host_file_path: Cow<'_, str>,
    text: &str,
) -> Result<u64, SystemdErrors> {
    let prog_n_args = &["pkexec", "tee", "tee", file_path];
    let mut cmd = commander(prog_n_args, None);
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| SystemdErrors::create_command_error(&cmd, error))?;

    let child_stdin = match child.stdin.as_mut() {
        Some(cs) => cs,
        None => {
            return Err(SystemdErrors::Custom(
                "Unable to write to file: No stdin".to_owned(),
            ));
        }
    };

    let bytes = text.as_bytes();
    let bytes_written = bytes.len();

    match child_stdin.write_all(bytes) {
        Ok(()) => {
            info!("Write content as root on {file_path}");
        }
        Err(error) => return Err(SystemdErrors::IoError(error)),
    };

    match child.wait() {
        Ok(exit_status) => {
            info!("Subprocess exit status: {exit_status:?}");
            if !exit_status.success() {
                let code = exit_status.code();
                warn!("Subprocess exit code: {code:?}");

                let Some(code) = code else {
                    return Err(SystemdErrors::Custom(
                        "Subprocess exit code: None".to_owned(),
                    ));
                };

                let subprocess_error = match code {
                    1 => {
                        if cfg!(feature = "flatpak") {
                            let vec = prog_n_args
                                .iter()
                                .map(|s| s.to_string())
                                .collect::<Vec<String>>()
                                .join(" ");
                            SystemdErrors::CmdNoFreedesktopFlatpakPermission(
                                Some(vec),
                                Some(file_path.to_string()),
                            )
                        } else {
                            SystemdErrors::Custom(format!("Subprocess exit code: {code}"))
                        }
                    }
                    126 | 127 => return Err(SystemdErrors::NotAuthorized),
                    _ => SystemdErrors::Custom(format!("Subprocess exit code: {code}")),
                };
                return Err(subprocess_error);
            }
        }
        Err(error) => {
            //warn!("Failed to wait suprocess: {:?}", error);
            return Err(SystemdErrors::IoError(error));
        }
    };

    Ok(bytes_written as u64)
}
