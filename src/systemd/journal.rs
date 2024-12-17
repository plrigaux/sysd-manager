/// Call systemd journal
///
/// Fields
/// https://www.freedesktop.org/software/systemd/man/latest/systemd.journal-fields.html#
/// https://www.freedesktop.org/software/systemd/man/latest/sd_journal_open.html
///
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::Local;
use log::{debug, info, warn};
use sysd::{id128::Id128, journal::OpenOptions, Journal};

use crate::widget::preferences::data::{DbusLevel, PREFERENCES};

use super::{data::UnitInfo, journal_data::JournalEvent, BootFilter, SystemdErrors};

const KEY_SYSTEMS_UNIT: &str = "_SYSTEMD_UNIT";
const KEY_SYSTEMS_USER_UNIT: &str = "_SYSTEMD_USER_UNIT";
const KEY_UNIT: &str = "UNIT";
const KEY_USER_UNIT: &str = "USER_UNIT";
const KEY_COREDUMP_UNIT: &str = "COREDUMP_UNIT";
const KEY_COREDUMP_USER_UNIT: &str = "COREDUMP_USER_UNIT";
const KEY_OBJECT_SYSTEMD_UNIT: &str = "OBJECT_SYSTEMD_UNIT";
const KEY_OBJECT_SYSTEMD_USER_UNIT: &str = "OBJECT_SYSTEMD_USER_UNIT";
const KEY_SYSTEMD_SLICE: &str = "_SYSTEMD_SLICE";
const KEY_SYSTEMD_USER_SLICE: &str = "_SYSTEMD_USER_SLICE";

const KEY_BOOT_ID: &str = "_BOOT_ID";
const KEY_MESSAGE: &str = "MESSAGE";
const KEY_PRIORITY: &str = "PRIORITY";
const KEY_PID: &str = "_PID";
const KEY_COMM: &str = "_COMM";

pub const BOOT_IDX: u8 = 200;

pub(super) fn get_unit_journal(
    unit: &UnitInfo,
    _in_color: bool,
    _oldest_first: bool,
    max_events: u32,
    boot_filter: BootFilter,
) -> Result<Vec<JournalEvent>, SystemdErrors> {
    info!("Starting journal-logger");

    // Open the journal
    let mut journal = OpenOptions::default()
        .open()
        .expect("Could not open journal");

    let unit_primary = unit.primary();
    let unit_name = unit_primary.as_str();

    let level = PREFERENCES.dbus_level();

    info!("JOURNAL UNIT NAME {} level {:?}", unit_name, level);

    match level {
        DbusLevel::System => {
            journal.match_add(KEY_SYSTEMS_UNIT, unit_name)?;
            journal.match_or()?;
            journal.match_add(KEY_UNIT, unit_name)?;
            journal.match_or()?;
            journal.match_add(KEY_COREDUMP_UNIT, unit_name)?;
            journal.match_or()?;
            journal.match_add(KEY_OBJECT_SYSTEMD_UNIT, unit_name)?;
            journal.match_or()?;
            journal.match_add(KEY_SYSTEMD_SLICE, unit_name)?;
        }
        DbusLevel::Session => {
            journal.match_add(KEY_SYSTEMS_USER_UNIT, unit_name)?;
            journal.match_or()?;
            journal.match_add(KEY_USER_UNIT, unit_name)?;
            journal.match_or()?;
            journal.match_add(KEY_COREDUMP_USER_UNIT, unit_name)?;
            journal.match_or()?;
            journal.match_add(KEY_OBJECT_SYSTEMD_USER_UNIT, unit_name)?;
            journal.match_or()?;
            journal.match_add(KEY_SYSTEMD_USER_SLICE, unit_name)?;
        }
    };

    match boot_filter {
        BootFilter::Current => {
            let boot_id = Id128::from_boot()?;
            let boot_str = format!("{}", boot_id);

            journal.match_and()?;
            journal.match_add(KEY_BOOT_ID, boot_str)?;
        }
        BootFilter::All => {
            //No filter
        }
        BootFilter::Id(boot_id) => {
            journal.match_and()?;
            journal.match_add(KEY_BOOT_ID, boot_id)?;
        }
    }

    let mut vec = Vec::new();

    let default = "NONE".to_string();
    let default_priority = "7".to_string();

    let mut index = 0;
    let mut last_boot_id = String::new();

    loop {
        if journal.next()? == 0 {
            debug!("BREAK nb {}", index);
            break;
        }

        let message = get_data(&mut journal, KEY_MESSAGE, &default);

        //TODO get the u64 timestamp directly
        let timestamp: SystemTime = journal.timestamp()?;

        let since_the_epoch = timestamp
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        let time_in_ms = since_the_epoch.as_millis();

        let pid = get_data(&mut journal, KEY_PID, &default);
        let priority_str = get_data(&mut journal, KEY_PRIORITY, &default_priority);
        let priority = priority_str.parse::<u8>().map_or(7, |u| u);

        let name = get_data(&mut journal, KEY_COMM, &default);

        let boot_id = get_data(&mut journal, KEY_BOOT_ID, &default);

        let prefix = make_prefix(time_in_ms, name, pid);

        let journal_event = JournalEvent::new_param(priority, time_in_ms as u64, prefix, message);

        if boot_id != last_boot_id && !last_boot_id.is_empty() {
            let boot_event = JournalEvent::new_param(
                BOOT_IDX,
                time_in_ms as u64 + 1,
                format!("-- Boot {boot_id} --"),
                String::new(),
            );
            last_boot_id = boot_id;
            vec.push(boot_event);
        }

        vec.push(journal_event);

        //if == 0 no limit
        if max_events != 0 {
            index += 1;
            if index >= max_events {
                warn!("journal events maxed!");
                return Ok(vec);
            }
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

fn make_prefix(timestamp: u128, name: String, pid: String) -> String {
    let local_result = chrono::TimeZone::timestamp_millis_opt(&Local, timestamp as i64);
    let fmt = "%b %d %T";
    let date = match local_result {
        chrono::offset::LocalResult::Single(l) => l.format(fmt).to_string(),
        chrono::offset::LocalResult::Ambiguous(a, _b) => a.format(fmt).to_string(),
        chrono::offset::LocalResult::None => "NONE".to_owned(),
    };

    format!("{date} {name}[{pid}]: ")
}
