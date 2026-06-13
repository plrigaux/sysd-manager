use base::{
    args,
    consts::SYSTEMD_ANALYSE,
    file::{SysdBaseError, commander},
};
use std::{ffi::OsStr, process::Stdio};
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{debug, info, warn};

pub async fn validate_calendar(calendar: &str) -> Result<(i32, String, String), SysdBaseError> {
    let cmd = args![SYSTEMD_ANALYSE, "calendar", calendar];
    execute_command(&cmd).await
}

pub async fn validate_timespan(timespan: &str) -> Result<(i32, String, String), SysdBaseError> {
    let cmd = args![SYSTEMD_ANALYSE, "timespan", timespan];
    execute_command(&cmd).await
}

macro_rules! read_std {
    ($reader:expr) => {{
        let mut out = String::new();
        let mut first = true;
        while let Some(line) = $reader.next_line().await? {
            if first {
                first = false
            } else {
                out.push('\n');
            }
            out.push_str(line.trim_ascii());
        }
        out
    }};
}

pub async fn execute_command(
    prog_n_args: &[&OsStr],
) -> Result<(i32, String, String), SysdBaseError> {
    let mut cmd = commander(prog_n_args, None);

    let mut child = cmd
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

    let handle: tokio::task::JoinHandle<Result<i32, SysdBaseError>> = tokio::spawn(async move {
        let exit_status = child.wait().await?;
        if exit_status.success() {
            info!("Script executed with success");
            return Ok(0);
        }

        let code = exit_status
            .code()
            .inspect(|code| warn!("Subprocess exit code: {code:?}"))
            .ok_or("Subprocess exit code: None")?;

        Ok(code)
    });

    let mut reader_out = BufReader::new(stdout).lines();
    let mut reader_err = BufReader::new(stderr).lines();
    debug!("Going to read out");

    let std_out = read_std!(reader_out);

    debug!("Going to read err");

    let std_err = read_std!(reader_err);

    debug!("Going to wait");

    match handle.await? {
        Ok(code) => Ok((code, std_out, std_err)),
        Err(SysdBaseError::ErrorExit(code)) => Ok((code, std_out, std_err)),
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod test {
    use test_base::init_logs;
    use tracing::error;

    use super::*;

    fn show_output(code: i32, out: String, err: String) {
        if code == 0 {
            info!("\n{}\n", out);
        } else {
            error!("\n{}\n", err);
        }
    }

    #[tokio::test]
    async fn test_calendar1() -> Result<(), SysdBaseError> {
        init_logs();
        let (code, out, err) = validate_calendar("2027-11-28 23:02:15").await?;

        show_output(code, out, err);
        Ok(())
    }

    #[tokio::test]
    async fn test_timespan() -> Result<(), SysdBaseError> {
        init_logs();
        let (code, out, err) = validate_timespan("1h").await?;

        show_output(code, out, err);
        Ok(())
    }

    #[tokio::test]
    async fn test_timespan_fail() -> Result<(), SysdBaseError> {
        init_logs();
        let (code, out, err) = validate_timespan("1 fail").await?;
        show_output(code, out, err);
        Ok(())
    }
}
