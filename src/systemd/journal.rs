use std::{collections::HashSet, ops::DerefMut};

/// Call systemd journal
///
/// Fields
/// https://www.freedesktop.org/software/systemd/man/latest/systemd.journal-fields.html#
/// https://www.freedesktop.org/software/systemd/man/latest/sd_journal_open.html
///
use crate::{
    systemd::{enums::UnitDBusLevel, journal_data::JournalEventChunkInfo},
    utils::th::{TimestampStyle, USEC_PER_SEC},
    widget::preferences::data::PREFERENCES,
};
use chrono::{Local, Utc};
use foreign_types_shared::ForeignType;
use libsysd::{self};
use log::{info, warn};
use sysd::{Journal, id128::Id128, journal::OpenOptions};

use super::{
    BootFilter, SystemdErrors,
    data::UnitInfo,
    journal_data::{EventRange, JournalEvent, JournalEventChunk},
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
//pub const EVENT_MAX_ID: u8 = 201;

pub(super) fn get_unit_journal(
    unit: &UnitInfo,
    boot_filter: BootFilter,
    range: EventRange,
) -> Result<JournalEventChunk, SystemdErrors> {
    let mut journal_reader = create_journal_reader(unit, boot_filter)?;

    let mut out_list = JournalEventChunk::new((range.batch_size + 10) as usize);

    let default = "NONE".to_string();
    let default_priority = "7".to_string();

    //let mut index = 0;
    let mut last_boot_id = String::new();

    let message_max_char = PREFERENCES.journal_event_max_size() as usize;

    let timestamp_style = PREFERENCES.timestamp_style();

    //Position the indexer
    if range.decending() {
        journal_reader.seek_tail()?;
    }

    if let Some(begin_from_time) = range.begin {
        info!("Seek to time {begin_from_time}");
        journal_reader.seek_realtime_usec(begin_from_time)?;

        //skip the seek event
        loop {
            if next(&mut journal_reader, range.oldest_first)? == 0 {
                out_list.set_info(JournalEventChunkInfo::NoMore);
                break;
            }

            let time_in_usec = get_realtime_usec(&journal_reader)?;

            //Continue until time change
            if time_in_usec != begin_from_time {
                previous(&mut journal_reader, range.oldest_first)?; //go back one event for capture
                break;
            }
        }
    }

    let mut last_time_in_usec: u64 = 0;

    loop {
        if next(&mut journal_reader, range.oldest_first)? == 0 {
            out_list.set_info(JournalEventChunkInfo::NoMore);
            break;
        }

        let time_in_usec = get_realtime_usec(&journal_reader)?;

        //if == 0 no limit
        if range.batch_size != 0 && out_list.len32() >= range.batch_size {
            info!(
                "Journal log events  count ({}) reached the {} limit!",
                out_list.len(),
                range.batch_size
            );

            //Ensure the time to be different
            if last_time_in_usec != time_in_usec {
                out_list.set_info(JournalEventChunkInfo::ChunkMaxReached);
                break;
            }
        }

        let mut message = get_data(&mut journal_reader, KEY_MESSAGE, &default);

        if message_max_char != 0 && message.len() > message_max_char {
            warn!(
                "MESSAGE LEN {} will truncate to {message_max_char}",
                message.len()
            );

            message = truncate(message, message_max_char);
        }

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

        if range.has_reached_end(time_in_usec) {
            break;
        }

        out_list.push(journal_event);

        last_time_in_usec = time_in_usec;
    }

    Ok(out_list)
}
/*
pub(super) fn get_unit_journal_continuous(
    unit: &UnitInfo,
) -> Result<JournalEventChunk, SystemdErrors> {
    let mut journal_reader = create_journal_reader(unit, BootFilter::All)?;

    let mut out_list = JournalEventChunk::new(50);

    let default = "NONE".to_string();
    let default_priority = "7".to_string();

    let mut index = 0;
    let mut last_boot_id = String::new();

    let message_max_char = PREFERENCES.journal_event_max_size() as usize;

    let timestamp_style = PREFERENCES.timestamp_style();

    //Position the indexer

    journal_reader.seek_tail()?;

    let mut last_time_in_usec: u64 = 0;

    loop {
        if next(&mut journal_reader, true)? == 0 {
            out_list.set_info(JournalEventChunkInfo::NoMore);

            let asdf = journal_reader.wait(Some(std::time::Duration::from_secs(1)))?;
            /*
            match journal_reader.wait(Some(std::time::Duration::from_secs(1))) {
                Ok(wait_result) => match wait_result {
                    sysd::JournalWaitResult::Nop => todo!(),
                    sysd::JournalWaitResult::Append => todo!(),
                    sysd::JournalWaitResult::Invalidate => todo!(),
                },
                Err(e) => return e.into(),
            } */
        }

        let mut message = get_data(&mut journal_reader, KEY_MESSAGE, &default);

        if message_max_char != 0 && message.len() > message_max_char {
            warn!(
                "MESSAGE LEN {} will truncate to {message_max_char}",
                message.len()
            );

            message = truncate(message, message_max_char);
        }

        let time_in_usec = get_realtime_usec(&journal_reader)?;

        let pid = get_data(&mut journal_reader, KEY_PID, &default);
        let priority_str = get_data(&mut journal_reader, KEY_PRIORITY, &default_priority);
        let priority = priority_str.parse::<u8>().map_or(7, |u| u);

        let name = get_data(&mut journal_reader, KEY_COMM, &default);

        let boot_id = get_data(&mut journal_reader, KEY_BOOT_ID, &default);

        let prefix = make_prefix(time_in_usec, name, pid, timestamp_style);

        let journal_event = JournalEvent::new_param(priority, time_in_usec, prefix, message);

        //if == 0 no limit

        /*  if range.has_reached_end(time_in_usec) {
            break;
        } */

        out_list.push(journal_event);

        last_time_in_usec = time_in_usec;
    }

    Ok(out_list)
}
*/
pub struct Boot {
    pub index: i32,
    pub boot_id: String,
    pub first: u64,
    pub last: u64,
    pub total: i32,
}

impl Boot {
    pub fn neg_offset(&self) -> i32 {
        -(self.total - self.index)
    }

    pub fn index(&self) -> i32 {
        self.index
    }

    pub fn duration(&self) -> u64 {
        self.last - self.first
    }
}

pub(super) fn list_boots() -> Result<Vec<Boot>, SystemdErrors> {
    info!("Starting journal-logger list boot");
    let mut journal_reader = OpenOptions::default()
        .open()
        .expect("Could not open journal");

    let mut last_boot_id = Id128::default();

    let mut set = HashSet::with_capacity(100);
    let mut boots: Vec<Boot> = Vec::with_capacity(100);
    let mut index = 1;
    loop {
        if journal_reader.next()? == 0 {
            break;
        }

        let (_, boot_id) = journal_reader.monotonic_timestamp()?;

        if last_boot_id == boot_id {
            continue;
        }
        last_boot_id = boot_id;

        let boot_id = boot_id.to_string();

        if !set.insert(boot_id.clone()) {
            continue;
        }

        if !boots.is_empty() {
            if journal_reader.previous()? == 0 {
                break;
            }

            let previous = get_realtime_usec(&journal_reader)?;

            if journal_reader.next()? == 0 {
                break;
            }

            if let Some(prev) = boots.last_mut() {
                prev.last = previous
            }
        }
        //if == 0 no limit
        //println!("{idx} boot_id {boot_id} time {time_in_usec}");

        let time_in_usec = get_realtime_usec(&journal_reader)?;
        boots.push(Boot {
            index,
            boot_id,
            first: time_in_usec,
            last: 0,
            total: 0,
        });
        index += 1;
    }

    let previous = get_realtime_usec(&journal_reader)?;

    if let Some(mut prev) = boots.last_mut() {
        let m = prev.deref_mut();
        m.last = previous
    }

    let total: i32 = boots.len() as i32;

    for boot in boots.iter_mut() {
        boot.total = total;
    }

    Ok(boots)
}

pub(super) fn fetch_last_time() -> Result<u64, SystemdErrors> {
    info!("Starting journal-logger list boot");
    let mut journal_reader = OpenOptions::default()
        .open()
        .expect("Could not open journal");

    journal_reader.seek_tail()?;
    journal_reader.previous()?;

    let last_time = get_realtime_usec(&journal_reader)?;

    Ok(last_time)
}

fn create_journal_reader(
    unit: &UnitInfo,
    boot_filter: BootFilter,
) -> Result<Journal, SystemdErrors> {
    info!("Starting journal-logger");
    let mut journal_reader = OpenOptions::default()
        .open()
        .expect("Could not open journal");
    let unit_primary = unit.primary();
    let unit_name = unit_primary.as_str();
    let level = unit.dbus_level();
    info!("JOURNAL UNIT NAME {} level {:?}", unit_name, level);
    match level {
        UnitDBusLevel::System => {
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
        UnitDBusLevel::UserSession => {
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
    Ok(journal_reader)
}

fn next(journal_reader: &mut Journal, oldest_first: bool) -> Result<u64, sysd::Error> {
    if oldest_first {
        journal_reader.next()
    } else {
        journal_reader.previous()
    }
}

fn previous(journal_reader: &mut Journal, oldest_first: bool) -> Result<u64, sysd::Error> {
    if oldest_first {
        journal_reader.previous()
    } else {
        journal_reader.next()
    }
}

fn get_realtime_usec(journal_reader: &Journal) -> Result<u64, SystemdErrors> {
    //  libsysd::journal::sd_journal_get_realtime_usec(journal_reader)

    let mut timestamp_us: u64 = 0;
    sysd::sd_try!(libsysd::journal::sd_journal_get_realtime_usec(
        journal_reader.as_ptr(),
        &mut timestamp_us
    ));

    Ok(timestamp_us)
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
            let timestamp = timestamp_usec / USEC_PER_SEC;
            format!("@{timestamp}")
        }
        TimestampStyle::UnixUsec => format!("@{timestamp_usec}"),
    };

    format!("{date} {name}[{pid}]: ")
}

#[cfg(test)]
mod tests {
    use std::{path::Path, time::UNIX_EPOCH};

    use sysd::journal;

    use crate::utils::th::get_since_time;

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

    fn have_journal() -> bool {
        if !Path::new("/run/systemd/journal/").exists() {
            println!("missing journal files");
            false
        } else {
            true
        }
    }

    #[test]
    fn test_timestamp() {
        if !have_journal() {
            return;
        }

        let mut journal = journal::OpenOptions::default().open().unwrap();
        info!("rust-systemd ts entry");
        journal.seek(journal::JournalSeek::Head).unwrap();
        journal.next().unwrap();
        let real_time_system_time = journal.timestamp().unwrap();

        let real_time = get_realtime_usec(&journal).unwrap();

        let since_the_epoch = real_time_system_time
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        let time_in_usec = since_the_epoch.as_micros() as u64;

        assert_eq!(real_time, time_in_usec);
    }

    #[test]
    fn test_get_boot() -> Result<(), SystemdErrors> {
        for (idx, boot) in list_boots()?.iter().enumerate() {
            let time = get_since_time(boot.first, TimestampStyle::Pretty);

            let time2 = get_since_time(boot.last, TimestampStyle::Pretty);

            println!("{idx} {} {} {}", boot.boot_id, time, time2);
        }

        Ok(())
    }
}
