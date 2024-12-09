use std::time::{SystemTime, UNIX_EPOCH};

use log::{debug, info, warn};
use sysd::{id128::Id128, journal::OpenOptions, Journal};

use super::{data::UnitInfo, JournalEventRaw, SystemdErrors};
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
const KEY_UNIT: &str = "UNIT";
const KEY_MESSAGE: &str = "MESSAGE";
const KEY_PRIORITY: &str = "PRIORITY";
const KEY_COREDUMP_UNIT: &str = "COREDUMP_UNIT";

const KEY_BOOT: &str = "_BOOT_ID";

pub(super) fn get_unit_journal2(
    unit: &UnitInfo,
    _in_color: bool,
    _oldest_first: bool,
    max_events: u32,
) -> Result<Vec<JournalEventRaw>, SystemdErrors> {
    info!("Starting journal-logger");

    // Open the journal
    let mut journal = OpenOptions::default() 
        .open()
        .expect("Could not open journal");

   

    let boot_id = Id128::from_boot()?;
    //debug!("BOOT {}", boot_id);
    let boot_str = format!("{}", boot_id);

    let unit_primary = unit.primary();
    let unit_name = unit_primary.as_str();

    warn!("JOURNAL UNIT NAME {}", unit_name);

    journal.match_add(KEY_SYSTEMS_UNIT, unit_name)?;
    journal.match_or()?;
    journal.match_add(KEY_UNIT, unit_name)?;
    journal.match_or()?;
    journal.match_add(KEY_COREDUMP_UNIT, unit_name)?;
    journal.match_or()?;
    journal.match_add("OBJECT_SYSTEMD_UNIT", unit_name)?;
    journal.match_or()?;
    journal.match_add("_SYSTEMD_SLICE", unit_name)?;
    journal.match_and()?;
    journal.match_add(KEY_BOOT, boot_str)?;

    let mut vec = Vec::new();

    let default = "NONE".to_string();
    let default_priority = "7".to_string();

    let mut index = 0;
    loop {        
        if journal.next()? == 0 {
            debug!("BREAK nb {}", index);
            break;
        }

        let message = get_data(&mut journal, KEY_MESSAGE, &default);

        let timestamp: SystemTime = journal.timestamp()?;

        let since_the_epoch = timestamp
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        let time_in_ms = since_the_epoch.as_millis();

        let priority_str = get_data(&mut journal, KEY_PRIORITY, &default_priority);

        let priority = priority_str.parse::<u8>().map_or(7, |u| u);

        let journal_event = JournalEventRaw {
            message,
            time: time_in_ms as u64,
            priority,
        };

        vec.push(journal_event);

        index += 1;
        if index >= max_events {
            warn!("journal events maxed!");
            return Ok(vec);
        }
    }

    Ok(vec)
}

fn get_data(reader: &mut Journal, field: &str, default: &String) -> String {
    let value = match reader.get_data(field) {
        Ok(journal_entry_op) => match journal_entry_op {
            Some(journal_entry_field) => journal_entry_field
                .value()
                .map(|v| String::from_utf8_lossy(v))
                .map_or(default.to_owned(), |v| v.into_owned()),
            None => default.to_owned(),
        },
        Err(e) => {
            warn!("Error get data {:?}", e);
            default.to_owned()
        }
    };
    value
}
