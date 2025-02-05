/// Call systemd journal
///
/// Fields
/// https://www.freedesktop.org/software/systemd/man/latest/systemd.journal-fields.html#
/// https://www.freedesktop.org/software/systemd/man/latest/sd_journal_open.html
///
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::{Local, Utc};
use log::{info, warn};
use sysd::{id128::Id128, journal::OpenOptions, Journal};

use crate::{
    systemd::journal_data::JournalEventChunkInfo,
    utils::th::TimestampStyle,
    widget::preferences::data::{DbusLevel, PREFERENCES},
};

use super::{
    data::UnitInfo,
    journal_data::{JournalEvent, JournalEventChunk},
    BootFilter, SystemdErrors,
};

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
pub const EVENT_MAX_ID: u8 = 201;

pub(super) fn get_unit_journal(
    unit: &UnitInfo,
    wait_time: Option<Duration>,
    _oldest_first: bool,
    max_chunk_events: u32,
    boot_filter: BootFilter,
    from_time: Option<u64>,
) -> Result<JournalEventChunk, SystemdErrors> {
    info!("Starting journal-logger");

    // Open the journal
    let mut journal_reader = OpenOptions::default()
        .open()
        .expect("Could not open journal");

    let unit_primary = unit.primary();
    let unit_name = unit_primary.as_str();

    let level = PREFERENCES.dbus_level();

    info!("JOURNAL UNIT NAME {} level {:?}", unit_name, level);

    match level {
        DbusLevel::System => {
            journal_reader.match_add(KEY_SYSTEMS_UNIT, unit_name)?;
            journal_reader.match_or()?;
            journal_reader.match_add(KEY_UNIT, unit_name)?;
            journal_reader.match_or()?;
            journal_reader.match_add(KEY_COREDUMP_UNIT, unit_name)?;
            journal_reader.match_or()?;
            journal_reader.match_add(KEY_OBJECT_SYSTEMD_UNIT, unit_name)?;
            journal_reader.match_or()?;
            journal_reader.match_add(KEY_SYSTEMD_SLICE, unit_name)?;
        }
        DbusLevel::Session => {
            journal_reader.match_add(KEY_SYSTEMS_USER_UNIT, unit_name)?;
            journal_reader.match_or()?;
            journal_reader.match_add(KEY_USER_UNIT, unit_name)?;
            journal_reader.match_or()?;
            journal_reader.match_add(KEY_COREDUMP_USER_UNIT, unit_name)?;
            journal_reader.match_or()?;
            journal_reader.match_add(KEY_OBJECT_SYSTEMD_USER_UNIT, unit_name)?;
            journal_reader.match_or()?;
            journal_reader.match_add(KEY_SYSTEMD_USER_SLICE, unit_name)?;
        }
    };

    match boot_filter {
        BootFilter::Current => {
            let boot_id = Id128::from_boot()?;
            let boot_str = format!("{}", boot_id);

            journal_reader.match_and()?;
            journal_reader.match_add(KEY_BOOT_ID, boot_str)?;
        }
        BootFilter::All => {
            //No filter
        }
        BootFilter::Id(boot_id) => {
            journal_reader.match_and()?;
            journal_reader.match_add(KEY_BOOT_ID, boot_id)?;
        }
    }

    let mut out_list = JournalEventChunk::new(max_chunk_events as usize);

    let default = "NONE".to_string();
    let default_priority = "7".to_string();

    let mut index = 0;
    let mut last_boot_id = String::new();

    let message_max_char = PREFERENCES.journal_event_max_size() as usize;

    let timestamp_style = PREFERENCES.timestamp_style();

    if let Some(from_time) = from_time {
        info!("Seek to {from_time}");
        if let Err(err) = journal_reader.seek_realtime_usec(from_time) {
            warn!("Seek real time error {:?}", err);
        };

        //skip the seek event
        loop {
            if journal_reader.next()? == 0 {
                out_list.set_info(JournalEventChunkInfo::NoMore);
                break;
            }

            let time_in_usec = get_realtime_usec(&journal_reader)?;

            //Continue until time change
            if time_in_usec != from_time {
                journal_reader.previous()?; //go back one event for capture
                break;
            }
        }
    }

    let mut last_time_in_usec: u64 = 0;

    loop {
        loop {
            if journal_reader.next()? == 0 {
                out_list.set_info(JournalEventChunkInfo::NoMore);
                break;
            }

            let mut message = get_data(&mut journal_reader, KEY_MESSAGE, &default);

            if message_max_char != 0 && message.len() > message_max_char {
                warn!(
                    "MESSAGE LEN {} will truncate to {message_max_char}",
                    message.len()
                );

                message = truncate(message, message_max_char);
            }

            //TODO get the u64 timestamp directly
            let time_in_usec = get_realtime_usec(&journal_reader)?;

            let pid = get_data(&mut journal_reader, KEY_PID, &default);
            let priority_str = get_data(&mut journal_reader, KEY_PRIORITY, &default_priority);
            let priority = priority_str.parse::<u8>().map_or(7, |u| u);

            let name = get_data(&mut journal_reader, KEY_COMM, &default);

            let boot_id = get_data(&mut journal_reader, KEY_BOOT_ID, &default);

            let prefix = make_prefix(time_in_usec, name, pid, timestamp_style);

            let journal_event = JournalEvent::new_param(priority, time_in_usec, prefix, message);

            if boot_id != last_boot_id {
                if !last_boot_id.is_empty() {
                    let boot_event = JournalEvent::new_param(
                        BOOT_IDX,
                        time_in_usec - 1,
                        String::new(),
                        format!("-- Boot {boot_id} --"),
                    );
                    out_list.push(boot_event);
                }

                last_boot_id = boot_id;
            }

            //if == 0 no limit
            if max_chunk_events != 0 {
                index += 1;
                if index >= max_chunk_events {
                    warn!("Journal log events reach the {max_chunk_events} limit!");

                    if last_time_in_usec != time_in_usec {
                        out_list.set_info(JournalEventChunkInfo::ChunkMaxReached);
                        break;
                    }
                }
            }
            out_list.push(journal_event);

            last_time_in_usec = time_in_usec;
        }

        if !out_list.is_empty() {
            //not empty so need to return
            break;
        }

        warn!("Waiting journal events {:?}", wait_time);
        match journal_reader.wait(wait_time)? {
            sysd::JournalWaitResult::Nop => {
                //no change
                out_list.set_info(JournalEventChunkInfo::NoEventsAfterWaiting);
                warn!("Finished Waiting no new");
                break;
            }
            sysd::JournalWaitResult::Append => {
                //new event, go capture them
                warn!("Finished Waiting some new events");
            }
            sysd::JournalWaitResult::Invalidate => {
                //Don't know what to do here
                out_list.set_info(JournalEventChunkInfo::Invalidate);
                warn!("Finished Waiting Invalidate");
                break;
            }
        }
    }

    Ok(out_list)
}

fn get_realtime_usec(journal_reader: &Journal) -> Result<u64, SystemdErrors> {
    let timestamp: SystemTime = journal_reader.timestamp()?;
    let since_the_epoch = timestamp
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    let time_in_usec = since_the_epoch.as_micros() as u64;
    Ok(time_in_usec)
}

fn truncate(s: String, max_chars: usize) -> String {
    match s.char_indices().nth(max_chars - 1) {
        None => s,
        Some((idx, _)) => {
            let mut trunk_string = s[..idx].to_string();
            trunk_string.push('\u{2026}');
            trunk_string
        }
    }
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

const FMT: &str = "%b %d %T";
const FMT_USEC: &str = "%b %d %T%.6f";

macro_rules! formated_time {
    ($local_result:expr, $fmt:expr) => {
        match $local_result {
            chrono::offset::LocalResult::Single(l) => l.format($fmt).to_string(),
            chrono::offset::LocalResult::Ambiguous(a, _b) => a.format($fmt).to_string(),
            chrono::offset::LocalResult::None => "NONE".to_owned(),
        }
    };
}

fn make_prefix(
    timestamp_usec: u64,
    name: String,
    pid: String,
    timestamp_style: TimestampStyle,
) -> String {
    let date = match timestamp_style {
        TimestampStyle::Pretty => {
            let local_result = chrono::TimeZone::timestamp_micros(&Local, timestamp_usec as i64);
            formated_time!(local_result, FMT)
        }
        TimestampStyle::PrettyUsec => {
            let local_result = chrono::TimeZone::timestamp_micros(&Local, timestamp_usec as i64);
            formated_time!(local_result, FMT_USEC)
        }
        TimestampStyle::Utc => {
            let local_result = chrono::TimeZone::timestamp_millis_opt(&Utc, timestamp_usec as i64);
            formated_time!(local_result, FMT)
        }
        TimestampStyle::UtcUsec => {
            let local_result = chrono::TimeZone::timestamp_millis_opt(&Utc, timestamp_usec as i64);
            formated_time!(local_result, FMT_USEC)
        }
        TimestampStyle::Unix => {
            let timestamp = timestamp_usec / 1000_000;
            format!("@{timestamp}")
        }
        TimestampStyle::UnixUsec => format!("@{timestamp_usec}"),
    };

    format!("{date} {name}[{pid}]: ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate() {
        let s = "12345678901234567890".to_string();

        let new_s = truncate(s.clone(), 10);

        println!("{s} {new_s}")
    }

    #[test]
    fn test_truncate2() {
        let s = "1".to_string();

        let new_s = truncate(s.clone(), 1);

        println!("{s} {new_s}")
    }
}
