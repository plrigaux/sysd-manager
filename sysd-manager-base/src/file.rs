use std::ffi::{OsStr, OsString};
use std::process::Stdio;
use std::{
    error::Error,
    io,
    path::{Path, PathBuf},
    sync::OnceLock,
};
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

use tokio::task::JoinError;
use tracing::debug;
#[allow(unused_imports)]
use tracing::{error, info, warn};

use crate::getuid;

#[macro_export]
macro_rules! args {
    ($($a:expr),*) => {
        [
            $(AsRef::<OsStr>::as_ref(&$a),)*
        ]
    }
}

#[macro_export]
macro_rules! vs {
    ($($a:expr),*) => {
        [
            $(AsRef::<String>::as_ref(&$a),)*
        ]
    }
}

#[derive(Debug)]
pub enum SysdBaseError {
    CmdNoFreedesktopFlatpakPermission,
    Command(
        OsString,
        Vec<OsString>,
        Vec<(OsString, Option<OsString>)>,
        io::Error,
    ),
    Custom(String),
    IoError(io::Error),
    NotAuthorizedAuthentificationDismissed,
    NotAuthorized,
    Tokio(JoinError),
}

impl SysdBaseError {
    pub(crate) fn create_command_error(command: &Command, error: std::io::Error) -> Self {
        let std_command = command.as_std();
        let program = std_command.get_program().to_os_string();
        let envs: Vec<(OsString, Option<OsString>)> = std_command
            .get_envs()
            .map(|(k, v)| (k.to_os_string(), v.map(|s| s.to_os_string())))
            .collect();
        let arg: Vec<OsString> = std_command.get_args().map(|s| s.to_os_string()).collect();

        SysdBaseError::Command(program, arg, envs, error)
    }
}

impl From<&str> for SysdBaseError {
    fn from(value: &str) -> Self {
        value.to_string().into()
    }
}

impl From<String> for SysdBaseError {
    fn from(value: String) -> Self {
        SysdBaseError::Custom(value)
    }
}

impl From<std::io::Error> for SysdBaseError {
    fn from(value: std::io::Error) -> Self {
        SysdBaseError::IoError(value)
    }
}

impl From<JoinError> for SysdBaseError {
    fn from(value: JoinError) -> Self {
        SysdBaseError::Tokio(value)
    }
}

pub fn determine_drop_in_path_dir(
    unit_name: &str,
    runtime: bool,
    user_session: bool,
) -> Result<String, Box<dyn Error + 'static>> {
    let path = match (runtime, user_session) {
        (true, false) => format!("/run/systemd/system/{}.d", unit_name),
        (false, false) => format!("/etc/systemd/system/{}.d", unit_name),
        (true, true) => {
            let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
                .unwrap_or_else(|_| format!("/run/user/{}", getuid()));

            format!("{runtime_dir}/systemd/user/{}.d", unit_name)
        }
        (false, true) => {
            let home_dir = std::env::home_dir().ok_or(Box::<dyn Error>::from(
                "No HOME found to create drop-in".to_string(),
            ))?;
            format!(
                "{}/.config/systemd/user/{}.d",
                home_dir.display(),
                unit_name
            )
        }
    };
    Ok(path)
}

pub fn create_drop_in_path_file(
    unit_name: &str,
    runtime: bool,
    user_session: bool,
    file_name: &str,
) -> Result<String, Box<dyn Error + 'static>> {
    let path_dir = determine_drop_in_path_dir(unit_name, runtime, user_session)?;

    let path = format!("{path_dir}/{file_name}.conf");

    info!(
        "Creating drop-in path for unit: {}, runtime: {}, user: {} -> path {}",
        unit_name, runtime, user_session, path
    );
    Ok(path)
}

pub async fn create_drop_in_io(file_path_str: &str, content: &str) -> Result<(), SysdBaseError> {
    if file_path_str.contains("../") {
        let err = std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            r#"The "../" patern is not supported""#,
        );

        return Err(err)?;
    }

    let file_path = PathBuf::from(file_path_str);

    let unit_drop_in_dir = file_path.parent().ok_or(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!("Parent dir of file {:?} is invalid", file_path_str),
    ))?;

    if !unit_drop_in_dir.exists() {
        info!("Creating dir {}", unit_drop_in_dir.display());
        match fs::create_dir_all(&unit_drop_in_dir).await {
            Ok(_) => {}
            Err(err) => {
                if err.kind() == std::io::ErrorKind::PermissionDenied && getuid() != 0 {
                    create_dir_all_with_priviledge(unit_drop_in_dir).await
                } else {
                    Err(err)?
                }?
            }
        }
    }

    //Save content
    info!("Creating file {}", file_path.display());
    let bytes_written = write_on_disk(&file_path, true, content).await?;

    info!(
        "{bytes_written} bytes writen on File {}",
        file_path.to_string_lossy()
    );
    Ok(())
}

pub async fn write_on_disk(
    file_path: &Path,
    create_file: bool,
    content: &str,
) -> Result<u64, SysdBaseError> {
    let bytes_written = match save_io(file_path, create_file, content).await {
        Ok(b) => b,
        Err(err) => {
            if err.kind() == std::io::ErrorKind::PermissionDenied && getuid() != 0 {
                write_with_priviledge(file_path, content).await
            } else {
                Err(err)?
            }?
        }
    };
    Ok(bytes_written)
}

async fn create_dir_all_with_priviledge(dir_path: &Path) -> Result<(), SysdBaseError> {
    let prog_n_args = args!["pkexec", "mkdir", "-p", dir_path];
    execute_command(None, &prog_n_args).await?;
    Ok(())
}

pub async fn write_with_priviledge(file_path: &Path, text: &str) -> Result<u64, SysdBaseError> {
    let prog_n_args = args!["pkexec", "tee", file_path];
    let input = text.as_bytes();
    execute_command(Some(input), &prog_n_args).await?;
    Ok(input.len() as u64)
}

pub async fn execute_command(
    input: Option<&[u8]>,
    prog_n_args: &[&OsStr],
) -> Result<(), SysdBaseError> {
    let mut cmd = commander(prog_n_args, None);

    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error: std::io::Error| SysdBaseError::create_command_error(&cmd, error))?;

    let stdout = child
        .stdout
        .take()
        .ok_or("Child did not have a handle to stdout")?;
    //.expect("child did not have a handle to stdout");

    let stderr = child
        .stderr
        .take()
        .ok_or("Child did not have a handle to stderr")?;

    if let Some(input) = input {
        let mut child_stdin = child
            .stdin
            .take()
            .ok_or("Unable to pass stdin to command")?;
        child_stdin.write_all(input).await?;
        drop(child_stdin);
    }

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
                    SysdBaseError::CmdNoFreedesktopFlatpakPermission
                }
                #[cfg(not(feature = "flatpak"))]
                {
                    Err(format!("Subprocess exit code: {code}"))?
                }
            }
            126 => SysdBaseError::NotAuthorized,
            127 => SysdBaseError::NotAuthorizedAuthentificationDismissed,
            _ => Err(format!("Subprocess exit code: {code}"))?,
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

pub async fn save_io(
    file_path: impl AsRef<Path>,
    create: bool,
    content: &str,
) -> Result<u64, std::io::Error> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(create)
        .open(file_path)
        .await?;

    let test_bytes = content.as_bytes();

    file.write_all(test_bytes).await?;
    file.flush().await?;

    let bytes_written = test_bytes.len();

    Ok(bytes_written as u64)
}

pub const FLATPAK_SPAWN: &str = "flatpak-spawn";

pub static INSIDE_FLATPAK: OnceLock<bool> = OnceLock::new();

#[macro_export]
macro_rules! inside_flatpak {
    () => {
        *INSIDE_FLATPAK.get_or_init(|| {
            #[cfg(not(feature = "flatpak"))]
            warn!("Not supposed to be called");

            let in_flatpak = std::env::var("FLATPAK_ID").is_ok();

            #[cfg(feature = "flatpak")]
            if !in_flatpak {
                warn!("Your run the flatpak compilation, but you aren't running inside a Flatpak");
            }

            in_flatpak
        })
    };
}

pub fn inside_flatpak() -> bool {
    inside_flatpak!()
}

/*     pub fn args<I, S>(&mut self, args: I) -> &mut Command
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>, */

#[cfg(feature = "flatpak")]
pub fn commander<I, S>(prog_n_args: I, environment_variables: Option<&[(&str, &str)]>) -> Command
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    if !inside_flatpak!() {
        error!("Command call might not work because you are not running inside a Flatpak")
    }

    let mut cmd = Command::new(FLATPAK_SPAWN);
    cmd.arg("--host");
    cmd.args(prog_n_args);

    if let Some(envs) = environment_variables {
        for env in envs {
            cmd.arg(format!("--env={}={}", env.0, env.1));
        }
    }

    cmd
}

#[cfg(not(feature = "flatpak"))]
pub fn commander<I, S>(prog_n_args: I, environment_variables: Option<&[(&str, &str)]>) -> Command
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut it = prog_n_args.into_iter();
    let mut cmd = Command::new(it.next().unwrap());

    for arg in it {
        cmd.arg(arg);
    }

    if let Some(envs) = environment_variables {
        for env in envs {
            cmd.env(env.0, env.1);
        }
    }

    cmd
}

pub fn commander_blocking<I, S>(
    prog_n_args: I,
    environment_variables: Option<&[(&str, &str)]>,
) -> std::process::Command
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    commander(prog_n_args, environment_variables).into_std()
}

pub fn test_flatpak_spawn() -> Result<(), io::Error> {
    #[cfg(feature = "flatpak")]
    {
        info!("test_flatpak_spawn");
        std::process::Command::new(FLATPAK_SPAWN)
            .arg("--help")
            .output()
            .map(|_o| ())
    }

    #[cfg(not(feature = "flatpak"))]
    Ok(())
}

/// To be able to acces the Flatpack mounted files.
/// Limit to /usr for the least access principle
pub fn flatpak_host_file_path(file_path: &str) -> PathBuf {
    #[cfg(feature = "flatpak")]
    {
        if inside_flatpak!() && (file_path.starts_with("/usr") || file_path.starts_with("/etc")) {
            let file_path = file_path.strip_prefix('/').unwrap_or(file_path);
            PathBuf::from_iter(["/run/host", file_path])
        } else {
            PathBuf::from(&file_path)
        }
    }

    #[cfg(not(feature = "flatpak"))]
    PathBuf::from(file_path)
}

#[cfg(test)]
mod test {
    use super::*;
    use test_base::init_logs;

    pub fn flatpak_host_file_path_t(file_path: &str) -> PathBuf {
        let file_path = if let Some(stripped) = file_path.strip_prefix('/') {
            stripped
        } else {
            file_path
        };
        PathBuf::from_iter(["/run/host", file_path])
    }

    pub fn flatpak_host_file_path_t2(file_path: &str) -> PathBuf {
        PathBuf::from("/run/host").join(file_path)
    }

    #[test]
    fn test_fp() {
        init_logs();

        let src = PathBuf::from("/tmp");
        let a = flatpak_host_file_path(&src.to_string_lossy());
        warn!("{} exists {}", a.display(), a.exists());
        warn!("{} exists {}", src.display(), src.exists());
    }

    #[test]
    fn test_fp2() {
        init_logs();

        let src = PathBuf::from("/tmp");
        let a = flatpak_host_file_path_t(&src.to_string_lossy());
        warn!("{} exists {}", a.display(), a.exists());
        warn!("{} exists {}", src.display(), src.exists());

        let b = flatpak_host_file_path_t("test");
        warn!("{} exists {}", b.display(), b.exists());

        let b = flatpak_host_file_path_t("/test");
        warn!("{} exists {}", b.display(), b.exists());

        let b = flatpak_host_file_path_t2("/test");
        warn!("{} exists {}", b.display(), b.exists());
    }
}
