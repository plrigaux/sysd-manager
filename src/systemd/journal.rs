use log::{debug, warn};

use crate::systemd::commander_output;

use super::{data::UnitInfo, SystemdErrors};

const JOURNALCTL: &str = "journalctl";


/// Obtains the journal log for the given unit.
pub(super) fn get_unit_journal(
    unit: &UnitInfo,
    in_color: bool,
    oldest_first: bool,
    max_events: u32,
) -> Result<String, SystemdErrors> {
    let unit_path = unit.primary();

    let mut jounal_cmd_line = vec![JOURNALCTL, "-b", "-u", &unit_path];

    let max_events_str = max_events.to_string();
    if max_events > 0 {
        jounal_cmd_line.push("-n");
        jounal_cmd_line.push(&max_events_str);
    }

    debug!("{:?}", jounal_cmd_line);

    let env = [("SYSTEMD_COLORS", "true")];
    let environment_variable: Option<&[(&str, &str)]> = if in_color { Some(&env) } else { None };

    let outout_utf8 = commander_output(&jounal_cmd_line, environment_variable)?.stdout;

    let logs = match String::from_utf8(outout_utf8) {
        Ok(logs) => logs,
        Err(e) => {
            warn!("Can't retreive journal:  {:?}", e);
            return Ok(String::new());
        }
    };

    let text = if oldest_first {
        logs.lines()
            .rev()
            .map(|x| x.trim())
            .fold(String::with_capacity(logs.len()), |acc, x| acc + "\n" + x)
    } else {
        logs
    };

    Ok(text)
}
