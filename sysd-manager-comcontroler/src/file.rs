use crate::{commander, errors::SystemdErrors};
use base::args;
#[cfg(not(feature = "flatpak"))]
use base::file::create_drop_in_io;
use base::file::{create_drop_in_path_file, flatpak_host_file_path};
use log::{debug, error, info, warn};
use std::{ffi::OsStr, fmt::Write, io::ErrorKind, path::Path, process::Stdio};
use tokio::{
    fs,
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
};

pub(crate) async fn create_drop_in(
    runtime: bool,
    unit_name: &str,
    file_name: &str,
    content: &str,
) -> Result<(), SystemdErrors> {
    let file_path = create_drop_in_path_file(unit_name, runtime, true, file_name)?;

    #[cfg(not(feature = "flatpak"))]
    create_drop_in_io(&file_path, content).await?;

    #[cfg(feature = "flatpak")]
    create_drop_in_script(&file_path, content).await?;

    Ok(())
}

pub async fn save_text_to_file(file_path: &str, text: &str) -> Result<u64, SystemdErrors> {
    let host_file_path = flatpak_host_file_path(file_path);
    info!("Try to save content on File: {}", host_file_path.display());
    match write_on_disk(text, &host_file_path).await {
        Ok(bytes_written) => Ok(bytes_written),
        Err(error) => {
            if let SystemdErrors::IoError(ref err) = error {
                match err.kind() {
                    ErrorKind::PermissionDenied => {
                        info!("Some error : {err}, try executing command as another user");
                        write_with_priviledge(file_path, text).await
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

async fn write_on_disk(text: &str, file_path: &Path) -> Result<u64, SystemdErrors> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(file_path)
        .await?;

    let test_bytes = text.as_bytes();
    file.write_all(test_bytes).await?;
    file.flush().await?;

    let bytes_written = test_bytes.len();
    info!(
        "{bytes_written} bytes writen on File: {}",
        file_path.display()
    );
    Ok(bytes_written as u64)
}

async fn write_with_priviledge(file_path: &str, text: &str) -> Result<u64, SystemdErrors> {
    let prog_n_args = args!["pkexec", "tee", file_path];
    let input = text.as_bytes();
    execute_command(input, &prog_n_args).await?;
    Ok(input.len() as u64)
}

async fn create_drop_in_script(file_path: &str, content: &str) -> Result<u64, SystemdErrors> {
    let file_path = flatpak_host_file_path(file_path);

    let file_path_str = file_path.to_string_lossy();

    let dir_name = file_path.parent().ok_or(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!("Parent dir of file {:?} is invalid", file_path_str),
    ))?;

    let mut script = String::new();

    writeln!(script, "echo Start script")?;
    writeln!(script, "echo Create drop-in at {}", file_path_str)?;
    writeln!(script, "mkdir -vp {}", dir_name.to_string_lossy())?;
    writeln!(script, "cat > {} <<- EOM", file_path_str)?;
    writeln!(script, "{}", content)?;
    writeln!(script, "EOM")?;
    writeln!(script, "echo End Script")?;

    script_with_priviledge(&script)
        .await
        .map(|_| content.len() as u64)
}

async fn script_with_priviledge(script: &str) -> Result<(), SystemdErrors> {
    let prog_n_args = args!["pkexec", "sh"];
    execute_command(script.as_bytes(), &prog_n_args).await
}

async fn execute_command(input: &[u8], prog_n_args: &[&OsStr]) -> Result<(), SystemdErrors> {
    let mut cmd = commander(prog_n_args, None);

    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error: std::io::Error| {
            SystemdErrors::create_command_error(cmd.as_std(), error)
        })?;

    let mut child_stdin = child
        .stdin
        .take()
        .ok_or("Unable to pass stdin to command")?;

    let stdout = child
        .stdout
        .take()
        .ok_or("Child did not have a handle to stdout")?;
    //.expect("child did not have a handle to stdout");

    let stderr = child
        .stderr
        .take()
        .ok_or("Child did not have a handle to stderr")?;

    child_stdin.write_all(input).await?;

    drop(child_stdin);

    let handle = tokio::spawn(async move {
        let exit_status = child.wait().await?;
        if exit_status.success() {
            info!("Script executed with success");
            return Ok(());
        }

        let code = exit_status
            .code()
            .inspect(|code| warn!("Subprocess exit code: {code:?}"))
            .ok_or("Subprocess exit code: None")?;

        let err = match code {
            1 => {
                #[cfg(feature = "flatpak")]
                {
                    SystemdErrors::CmdNoFreedesktopFlatpakPermission(None, None)
                }
                #[cfg(not(feature = "flatpak"))]
                {
                    SystemdErrors::Custom(format!("Subprocess exit code: {code}"))
                }
            }
            126 => SystemdErrors::NotAuthorized,
            127 => SystemdErrors::NotAuthorizedAuthentificationDismissed,
            _ => SystemdErrors::Custom(format!("Subprocess exit code: {code}")),
        };
        Err(err)
    });

    let mut reader_out = BufReader::new(stdout).lines();
    let mut reader_err = BufReader::new(stderr).lines();
    debug!("Going to read out");

    while let Some(line) = reader_out.next_line().await? {
        info!("Script line: {}", line);
    }

    debug!("Going to read err");

    while let Some(line) = reader_err.next_line().await? {
        error!("Script line: {}", line);
    }

    debug!("Going to wait");

    handle.await?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write;
    use std::{fs, path::PathBuf};
    use test_base::init_logs;
    use tokio::io::AsyncBufReadExt;
    use tokio::io::BufReader;

    use crate::{errors::SystemdErrors, file::write_with_priviledge};

    #[tokio::test]
    async fn test_write_with_prvi() -> Result<(), SystemdErrors> {
        init_logs();

        let p = PathBuf::from(".").canonicalize()?.join("test.txt");

        let r = write_with_priviledge(&p.to_string_lossy(), "Some text for a test 2").await?;

        info!("Bytes written: {}", r);

        Ok(())
    }

    #[test]
    fn test_canonicalize() {
        let srcdir = PathBuf::from("./src");
        println!("{:?}", fs::canonicalize(&srcdir));

        let solardir = PathBuf::from(".");
        println!("{:?}", fs::canonicalize(&solardir));
    }

    #[tokio::test]
    async fn test_script() -> Result<(), SystemdErrors> {
        init_logs();

        let mut s = String::new();

        let path = PathBuf::from(".");
        let mut dir_name = fs::canonicalize(&path)?;
        dir_name.push("asdf.d");

        let file_name = "test_out.txt";
        let file_name = dir_name.join(file_name);
        let file_name = file_name.to_string_lossy();

        println!("{}", file_name);

        let content =
            "thet is a test \n abigeg test\n ffffffffffffffffffffffffff\n aaaaaaaaaaaaaaa";

        writeln!(s, "echo TEST file {}", file_name)?;
        writeln!(s, "mkdir -vp {}", dir_name.to_string_lossy())?;
        writeln!(s, "cat > {} <<- EOM", file_name)?;
        writeln!(s, "{}", content)?;
        writeln!(s, "EOM")?;

        script_with_priviledge(&s).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_create_drop_in_script() -> Result<(), SystemdErrors> {
        init_logs();

        let path = PathBuf::from(".");
        let mut dir_name = fs::canonicalize(&path)?;
        dir_name.push("test_dir.d");

        let file_name = "test_out.txt";
        let file_name = dir_name.join(file_name);
        let file_name = file_name.to_string_lossy();
        info!("{}", file_name);

        let content =
            "thet is a test \n abigeg test\n ffffffffffffffffffffffffff\n aaaaaaaaaaaaaaa";

        create_drop_in_script(&file_name, content).await?;

        Ok(())
    }

    use std::process::Stdio;
    use tokio::io::AsyncWriteExt;
    #[tokio::test]
    async fn test_sort() -> Result<(), Box<dyn std::error::Error>> {
        init_logs();
        let mut cmd = tokio::process::Command::new("sh");

        // Specifying that we want pipe both the output and the input.
        // Similarly to capturing the output, by configuring the pipe
        // to stdin it can now be used as an asynchronous writer.
        cmd.stdout(Stdio::piped());
        cmd.stdin(Stdio::piped());

        let mut child = cmd.spawn().expect("failed to spawn command");

        let stdout = child
            .stdout
            .take()
            .expect("child did not have a handle to stdout");

        let mut stdin = child
            .stdin
            .take()
            .expect("child did not have a handle to stdin");

        let mut reader = BufReader::new(stdout).lines();
        stdin
            .write_all("echo test test".as_bytes())
            .await
            .expect("could not write to stdin");

        // We drop the handle here which signals EOF to the child process.
        // This tells the child process that it there is no more data on the pipe.
        drop(stdin);

        let op = child.wait().await?;

        println!("ExitStatus: {}", op);

        while let Some(line) = reader.next_line().await? {
            info!("Line: {}", line);
        }

        Ok(())
    }
}
