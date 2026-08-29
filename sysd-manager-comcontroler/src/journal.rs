use super::BootFilter;
use crate::{
    boots::{KEY_BOOT_ID, MyId128},
    errors::SystemdErrors,
    journal_data::{
        BOOT_IDX, EventRange, JournalEvent, JournalEventChunk, JournalEventChunkInfo, WhatGrab,
    },
    time_handling::{TimestampStyle, USEC_PER_SEC},
};
use base::enums::UnitDBusLevel;
use chrono::{Local, Utc};
use std::sync::mpsc::TryRecvError;
use sysd::{Journal, id128::Id128, journal::OpenOptions};
use tracing::{debug, info, trace, warn};

/// Call systemd journal
///
/// Fields
/// https://www.freedesktop.org/software/systemd/man/latest/systemd.journal-fields.html#
/// https://www.freedesktop.org/software/systemd/man/latest/sd_journal_open.html
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
const KEY_MESSAGE: &str = "MESSAGE";
const KEY_PRIORITY: &str = "PRIORITY";
const KEY_PID: &str = "_PID";
const KEY_COMM: &str = "_COMM";

//pub const EVENT_MAX_ID: u8 = 201;

pub(super) fn get_unit_journal_events(
    primary_name: String,
    level: UnitDBusLevel,
    boot_filter: BootFilter,
    range: EventRange,
    message_max_char: usize,
    timestamp_style: TimestampStyle,
) -> Result<JournalEventChunk, SystemdErrors> {
    let mut out_list = JournalEventChunk::new(range.batch_size + 10, range.what_grab);

    info!("Get journal Event {primary_name:?}");
    let mut journal_reader = create_journal_reader(&primary_name, level, boot_filter)?;

    let default = "NONE";
    let default_priority = "7";

    //let mut index = 0;
    let mut last_boot_id = String::new();

    //Position the indexer
    position_crawler(&mut journal_reader, &range)?;

    let mut last_time_in_usec: u64 = 0;

    loop {
        if next(&mut journal_reader, range.what_grab)? == 0 {
            out_list.set_info(JournalEventChunkInfo::NoMore);
            break;
        }

        let time_in_usec = journal_reader.timestamp_usec()?;

        //if == 0 no limit
        if range.batch_size != 0 && out_list.len() >= range.batch_size {
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

        let mut message = get_data(&mut journal_reader, KEY_MESSAGE, default);

        if message_max_char > 0 {
            message = truncate(message, message_max_char);
        }

        let pid = get_data(&mut journal_reader, KEY_PID, default);
        let priority_str = get_data(&mut journal_reader, KEY_PRIORITY, default_priority);
        let priority = priority_str.parse::<u8>().unwrap_or(7);

        let name = get_data(&mut journal_reader, KEY_COMM, default);

        let boot_id = get_data(&mut journal_reader, KEY_BOOT_ID, default);

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

        out_list.push(journal_event);

        last_time_in_usec = time_in_usec;
    }

    Ok(out_list)
}

fn position_crawler(journal_reader: &mut Journal, range: &EventRange) -> Result<(), SystemdErrors> {
    match range.what_grab {
        WhatGrab::Newer => {
            if let Some(newest_events_time) = range.newest_events_time {
                journal_reader.seek_realtime_usec(newest_events_time + 1)?;
            } else {
                //Go to the end
                // journal_reader.seek_tail()?;
                journal_reader.seek_head()?;
                //Go back to batch size
                journal_reader.previous_skip(range.batch_size as u64)?;
            }
        }
        WhatGrab::Older => {
            if let Some(oldest_events_time) = range.oldest_events_time {
                journal_reader.seek_realtime_usec(oldest_events_time - 1)?;
            } else {
                journal_reader.seek_tail()?;
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn get_unit_journal_events_continuous(
    unit_name: String,
    bus_level: UnitDBusLevel,
    range: EventRange,
    journal_continuous_receiver: std::sync::mpsc::Receiver<()>,
    sender: std::sync::mpsc::Sender<JournalEventChunk>,
    message_max_char: usize,
    timestamp_style: TimestampStyle,
    check_for_new_journal_entry: fn(),
) -> Result<(), SystemdErrors> {
    info!("Journal Continuous");
    let mut journal_reader = create_journal_reader(&unit_name, bus_level, BootFilter::Current)?;

    let default = "NONE".to_string();
    let default_priority = "7".to_string();

    //Position the indexer
    position_crawler(&mut journal_reader, &range)?;

    let mut out_list = JournalEventChunk::new_info(8, JournalEventChunkInfo::Tail, range.what_grab);

    loop {
        let mut idx = 0;
        loop {
            debug!("loop {} {:?}", unit_name, range.what_grab);
            match journal_continuous_receiver.try_recv() {
                Ok(_) | Err(TryRecvError::Disconnected) => {
                    info!("Terminating journal loop for {unit_name:?}.");
                    return Ok(());
                }
                Err(TryRecvError::Empty) => {}
            }

            if next(&mut journal_reader, range.what_grab)? == 0 {
                if !out_list.is_empty() {
                    if let Err(send_error) = sender.send(out_list) {
                        warn!("Send Error: {send_error:?}")
                    }

                    glib::source::idle_add(move || {
                        check_for_new_journal_entry(); //TODO validate if best

                        glib::ControlFlow::Break
                    });
                    out_list = JournalEventChunk::new_info(
                        8,
                        JournalEventChunkInfo::Tail,
                        range.what_grab,
                    );
                }

                match journal_reader.wait(Some(std::time::Duration::from_secs(1)))? {
                    sysd::JournalWaitResult::Nop => {
                        trace!("wait loop {idx}");
                        idx += 1;
                    }
                    sysd::JournalWaitResult::Append => {
                        debug!("New Results")
                    }
                    sysd::JournalWaitResult::Invalidate => {
                        debug!("Invalidate")
                    }
                }
            } else {
                debug!("break");
                break;
            }
        }

        debug!("END sub loop");

        let mut message = get_data(&mut journal_reader, KEY_MESSAGE, &default);

        if message_max_char != 0 && message.len() > message_max_char {
            warn!(
                "MESSAGE LEN {} will truncate to {message_max_char}",
                message.len()
            );

            message = truncate(message, message_max_char);
        }

        let time_in_usec = journal_reader.timestamp_usec()?;

        let pid = get_data(&mut journal_reader, KEY_PID, &default);
        let priority_str = get_data(&mut journal_reader, KEY_PRIORITY, &default_priority);
        let priority = priority_str.parse::<u8>().unwrap_or(7);

        let name = get_data(&mut journal_reader, KEY_COMM, &default);

        let prefix = make_prefix(time_in_usec, name, pid, timestamp_style);

        let journal_event = JournalEvent::new_param(priority, time_in_usec, prefix, message);
        println!("Push 1 event");
        out_list.push(journal_event);
    }
    // Unreachable
}

pub(super) fn fetch_last_time() -> Result<u64, SystemdErrors> {
    info!("Starting journal-logger list boot");
    let mut journal_reader = OpenOptions::default()
        .open()
        .expect("Could not open journal");

    journal_reader.seek_tail()?;
    journal_reader.previous()?;

    let last_time = journal_reader.timestamp_usec()?;

    Ok(last_time)
}

fn create_journal_reader(
    unit_name: &str,
    level: UnitDBusLevel,
    boot_filter: BootFilter,
) -> Result<Journal, SystemdErrors> {
    let mut journal_reader = OpenOptions::default()
        .open()
        .expect("Could not open journal");

    info!("JOURNAL UNIT {unit_name:?} LEVEL {level:?} BOOT {boot_filter:?}");
    match level {
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
        _ => {
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
    };

    match boot_filter {
        BootFilter::Current => {
            let boot_id = Id128::from_boot()?;

            journal_reader.match_and()?;
            info!("Current boot Id {}", MyId128(boot_id));
            journal_reader.match_add(KEY_BOOT_ID, boot_id.to_string())?;
        }
        BootFilter::All => {
            //No filter
        }
        BootFilter::Id(boot_id) => {
            journal_reader.match_and()?;
            let boot_id = BootFilter::normalize(&boot_id);
            journal_reader.match_add(KEY_BOOT_ID, boot_id)?;
        }
    }
    Ok(journal_reader)
}

fn next(journal_reader: &mut Journal, grab_direction: WhatGrab) -> Result<u64, sysd::Error> {
    match grab_direction {
        WhatGrab::Newer => journal_reader.next(),
        WhatGrab::Older => journal_reader.previous(),
    }
}

fn truncate(message: String, max_chars: usize) -> String {
    if message.len() > max_chars {
        for index in (0..=max_chars).rev() {
            if message.is_char_boundary(index) {
                warn!("MESSAGE LEN {} will truncate to {index}", message.len());

                return [&message[..index], "\u{2026}"].concat();
            }
        }
    }
    message
}

fn get_data(reader: &mut Journal, field: &str, default: &str) -> String {
    match reader.get_data(field) {
        Ok(journal_entry_op) => match journal_entry_op {
            Some(journal_entry_field) => journal_entry_field
                .value()
                .map(|v| String::from_utf8_lossy(v))
                .map_or(default.to_owned(), |v| v.into_owned()),
            None => default.to_owned(),
        },
        Err(e) => {
            warn!("Get data field {field} Error: {e:?}");
            default.to_owned()
        }
    }
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
    use test_base::init_logs;

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

    #[test]
    fn test_truncate3() {
        init_logs();
        let s = "Löwe 老虎 Léopard".to_string();

        let new_s = truncate(s.clone(), 8);
        info!("{s} {new_s}");

        let new_s = truncate(s.clone(), 9);
        info!("{s} {new_s}");
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

        let real_time = journal.timestamp_usec().unwrap();

        let since_the_epoch = real_time_system_time
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        let time_in_usec = since_the_epoch.as_micros() as u64;

        assert_eq!(real_time, time_in_usec);
    }
}
