use std::time::{SystemTime, UNIX_EPOCH};

use log::{debug, info};
use sysd::{id128::Id128, journal::OpenOptions, sd_try, Journal};

use super::{data::UnitInfo, SystemdErrors};
/*
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
 */
const KEY_SYSTEMS_UNIT: &str = "_SYSTEMD_UNIT";
const KEY_MESSAGE: &str = "MESSAGE";

const KEY_BOOT: &str = "_BOOT_ID";

pub(super) fn get_unit_journal2(
    unit: &UnitInfo,
    _in_color: bool,
    _oldest_first: bool,
    max_events: u32,
) -> Result<Vec<(u128, String)>, SystemdErrors> {
    info!("Starting journal-logger");

    // Open the journal
    let mut journal = OpenOptions::default()
        //.system(true)
        .open()
        .expect("Could not open journal");

    let mut i = 0;

    //journal.match_and()
    // tiny_daemon.service

    let unit_name = unit.primary();

    journal.match_add(KEY_SYSTEMS_UNIT, unit_name.as_str())?;

    let boot_id = Id128::from_boot()?;

    debug!("BOOT {}", boot_id);

    let bt = format!("{}", boot_id);
    journal.match_add(KEY_BOOT, bt)?;

    let mut vec = Vec::new();

    loop {
        if journal.next()? == 0 {
            println!("BREAK");
            break;
        }

        let message_op = get_data(&mut journal, KEY_MESSAGE);

        match message_op {
            Some(message) => {
                let mut timestamp_us: u64 = 0;

                let timestamp: SystemTime = journal.timestamp()?;

                let since_the_epoch = timestamp
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards");
                let in_ms = since_the_epoch.as_millis();

                vec.push((in_ms, message));
            }
            None => {}
        }

        i += 1;
        if i >= max_events {
            info!("journal events maxed!");
            return Ok(vec);
        }
    }

    Ok(vec)
}

fn get_data(reader: &mut Journal, field: &str) -> Option<String> {
    let s = match reader.get_data(field) {
        Ok(journal_entry_op) => match journal_entry_op {
            Some(journal_entry_field) => journal_entry_field
                .value()
                .map(|v| String::from_utf8_lossy(v))
                .map(|v| v.into_owned()),
            None => None,
        },
        Err(e) => {
            println!("Error get data {:?}", e);
            None
        }
    };
    s
}
