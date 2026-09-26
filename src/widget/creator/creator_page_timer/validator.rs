use adw::prelude::*;
use base::{
    args,
    consts::SYSTEMD_ANALYZE,
    file::{SysdBaseError, commander},
};
use std::{ffi::OsStr, process::Stdio};
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{debug, error, info, warn};

use crate::{consts::WARNING_CSS, widget::creator::unit_file::ON_CALENDAR};

#[derive(Debug, Default)]
pub struct TimeSpan {
    pub exit_status: i32,
    pub output: String,
    pub error: String,
}
impl TimeSpan {
    fn new(exit_status: i32, output: String, error: String) -> Self {
        Self {
            exit_status,
            output,
            error,
        }
    }
}

pub async fn validate_calendar(calendar: &str) -> Result<TimeSpan, SysdBaseError> {
    let cmd = args![SYSTEMD_ANALYZE, "calendar", calendar];
    execute_command(&cmd).await
}

pub async fn validate_timespan(timespan: &str) -> Result<TimeSpan, SysdBaseError> {
    let cmd = args![SYSTEMD_ANALYZE, "timespan", timespan];
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

pub async fn execute_command(prog_n_args: &[&OsStr]) -> Result<TimeSpan, SysdBaseError> {
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
        Ok(code) => Ok(TimeSpan::new(code, std_out, std_err)),
        Err(SysdBaseError::ErrorExit(code)) => Ok(TimeSpan::new(code, std_out, std_err)),
        Err(err) => Err(err),
    }
}

pub fn validate_monotonic_entry(label: String, entry_row: &adw::EntryRow) {
    let entry_row = entry_row.clone();
    glib::spawn_future_local(async move {
        let timespan = entry_row.text();

        let ts = if timespan.is_empty() {
            TimeSpan::default()
        } else {
            let Ok(r) = systemd::runtime()
                .block_on(async move { validate_timespan(timespan.as_str()).await })
                .inspect_err(|err| error!("{err:?}"))
            else {
                return;
            };
            r
        };

        if ts.exit_status == 0 {
            entry_row.set_tooltip_text(Some(&format!("{}\n{}", label, ts.output)));
            entry_row.set_title(&label);
            entry_row.remove_css_class(WARNING_CSS);
        } else {
            entry_row.set_title(&format!("{}\n{}", label, ts.error));
            entry_row.set_tooltip_text(None);
            entry_row.add_css_class(WARNING_CSS);
        }
    });
}

pub fn validate_calendar_entry(entry_row: &adw::EntryRow) {
    let entry_row = entry_row.clone();
    glib::spawn_future_local(async move {
        let calendar = entry_row.text();

        let ts = if calendar.is_empty() {
            TimeSpan::default()
        } else {
            let Ok(r) = systemd::runtime()
                .block_on(async move {
                    super::validator::validate_calendar(calendar.trim_ascii()).await
                })
                .inspect_err(|err| error!("{err:?}"))
            else {
                return;
            };
            r
        };

        if ts.exit_status == 0 {
            entry_row.set_tooltip_text(Some(&format!("{}\n{}", ON_CALENDAR, ts.output)));
            entry_row.set_title(ON_CALENDAR);
            entry_row.remove_css_class(WARNING_CSS);
        } else {
            entry_row.set_title(&format!("{}\n{}", ON_CALENDAR, ts.error));
            entry_row.set_tooltip_text(None);
            entry_row.add_css_class(WARNING_CSS);
        }
    });
}

#[cfg(test)]
mod test {
    use test_base::init_logs;
    use tracing::error;

    use super::*;

    fn show_output(ts: TimeSpan) {
        if ts.exit_status == 0 {
            info!("\n{:?}\n", ts);
        } else {
            error!("\n{:?}\n", ts);
        }
    }

    #[tokio::test]
    async fn test_calendar1() -> Result<(), SysdBaseError> {
        init_logs();
        let ts = validate_calendar("2027-11-28 23:02:15").await?;

        show_output(ts);
        Ok(())
    }

    #[tokio::test]
    async fn test_timespan() -> Result<(), SysdBaseError> {
        init_logs();
        let ts = validate_timespan("1h").await?;

        show_output(ts);
        Ok(())
    }

    #[tokio::test]
    async fn test_timespan_fail() -> Result<(), SysdBaseError> {
        init_logs();
        let ts = validate_timespan("1 fail").await?;
        show_output(ts);
        Ok(())
    }
}
