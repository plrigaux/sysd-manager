use crate::{errors::SystemdErrors, journal_data::Boot};
use std::{
    fmt::{self, Write},
    hash::{Hash, Hasher},
    ops::DerefMut,
};
use sysd::{Journal, id128::Id128, journal::OpenOptions};
use tracing::info;

pub const KEY_BOOT_ID: &str = "_BOOT_ID";

#[derive(Clone, Copy)]
pub struct MyId128(pub Id128);

impl Hash for MyId128 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Delegate to the inner struct's fields or a specific method
        self.0.as_bytes().hash(state);
    }
}

impl PartialEq for MyId128 {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_bytes() == other.0.as_bytes()
    }
}

impl Eq for MyId128 {}

impl fmt::Display for MyId128 {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 9a153724-1b4e-44e5-96af-4d5c8d2d7256
        for (pos, b) in self.0.as_bytes().iter().enumerate() {
            if matches!(pos, 4 | 6 | 8 | 10) {
                fmt.write_char('-')?;
            }
            write!(fmt, "{b:02x}")?;
        }
        Ok(())
    }
}

pub(super) fn list_boots() -> Result<Vec<Boot>, SystemdErrors> {
    info!("Starting journal-logger list boot");
    let mut journal_reader = OpenOptions::default()
        .system(true)
        .local_only(true)
        .open()
        .expect("Could not open journal");

    let mut last_boot_id = Id128::default();

    let mut boots: Vec<Boot> = Vec::with_capacity(200);
    // let mut index = 1;

    //Find first boot
    //position to the oldest one
    journal_reader.seek_head(); //seek the oldest

    const MAX: usize = u32::MAX as usize;

    journal_reader.match_flush();

    loop {
        //no more
        if journal_reader.next()? == 0 {
            break;
        }

        let (_, boot_id) = journal_reader.monotonic_timestamp()?;

        if last_boot_id == boot_id {
            continue;
        }

        last_boot_id = boot_id;

        // let boot_id_str = MyId128(boot_id).to_string();

        if !boots.is_empty() {
            if journal_reader.previous()? == 0 {
                break;
            }

            let previous = journal_reader.timestamp_usec()?;

            if journal_reader.next()? == 0 {
                break;
            }

            if let Some(prev) = boots.last_mut() {
                prev.last = previous
            }
        }
        //if == 0 no limit
        //println!("{idx} boot_id {boot_id} time {time_in_usec}");

        let time_in_usec = journal_reader.timestamp_usec()?;
        boots.push(Boot {
            // index,
            boot_id: MyId128(boot_id),
            first: time_in_usec,
            last: 0,
            // total: 0,
        });

        if boots.len() >= MAX {
            break;
        }

        // index += 1;
    }

    let previous = journal_reader.timestamp_usec()?;

    if let Some(mut prev) = boots.last_mut() {
        let m = prev.deref_mut();
        m.last = previous
    }

    // let total: i32 = boots.len() as i32;

    // for boot in boots.iter_mut() {
    //     boot.total = total;
    // }

    Ok(boots)
}

pub(super) fn list_boots2() -> Result<Vec<Boot>, SystemdErrors> {
    info!("Starting journal-logger list boot");
    let mut journal_reader = OpenOptions::default()
        .system(true)
        .local_only(true)
        .open()
        .expect("Could not open journal");

    let mut previous_id = Id128::default();

    let mut boots: Vec<Boot> = Vec::with_capacity(200);
    // let mut index = 1;

    //Find first boot
    //position to the oldest one
    journal_reader.seek_head(); //seek the oldest

    const MAX: usize = u32::MAX as usize;

    journal_reader.match_flush();

    loop {
        discover_next_id(&mut journal_reader, previous_id, &mut boots);
        if boots.len() >= MAX {
            break;
        }

        // index += 1;
    }

    let previous = journal_reader.timestamp_usec()?;

    if let Some(mut prev) = boots.last_mut() {
        let m = prev.deref_mut();
        m.last = previous
    }

    // let total: i32 = boots.len() as i32;

    // for boot in boots.iter_mut() {
    //     boot.total = total;
    // }

    Ok(boots)
}

fn discover_next_id(
    j: &mut Journal,
    //  boot_id: Id128, /* optional, used when type == JOURNAL_{SYSTEM,USER}_UNIT_INVOCATION_ID */
    // const char *unit,    /* mandatory when type == JOURNAL_{SYSTEM,USER}_UNIT_INVOCATION_ID */
    previous_id: Id128,
    boots: &mut Vec<Boot>,
) -> Result<(), SystemdErrors> {
    /* We expect the journal to be on the last position of a boot
     * (in relation to the direction we are going), so that the next
     * invocation of sd_journal_next/previous will be from a different
     * boot. We then collect any information we desire and then jump
     * to the last location of the new boot by using a _BOOT_ID match
     * coming from the other journal direction. */

    /* Make sure we aren't restricted by any _BOOT_ID matches, so that
     * we can actually advance to a *different* boot. */

    //set_matches_for_discover_id(j, boot_id);

    loop {
        if j.next()? == 0 {
            j.match_flush(); //End of journal
            break;
        }

        let (_, boot_id) = j.monotonic_timestamp()?;

        /* We iterate through this in a loop, until the boot or invocation ID differs from the
         * previous one. Note that normally, this will only require a single iteration, as we moved
         * to the last entry of the previous boot or invocation entry already. However, it might
         * happen that the per-journal-field entry arrays are less complete than the main entry
         * array, and hence might reference an entry that's not actually the last one of the boot or
         * invocation ID as last one. Let's hence use the per-field array is initial seek position to
         * speed things up, but let's not trust that it is complete, and hence, manually advance as
         * necessary. */

        // if (!sd_id128_is_null(previous_id) && sd_id128_equal(id.id, previous_id))
        // continue;

        if previous_id == boot_id {
            continue;
        }
        // if (set_contains(broken_ids, &id.id))
        // continue;

        /* Yay, we found a new boot or invocation ID from the entry object. Let's check there exist
         * corresponding entries matching with the _BOOT_ID=, INVOCATION_ID= or friends data. */

        set_matches_for_discover_id(j, boot_id);

        /* First, seek to the first (or the last when we are going upwards) occurrence of this boot
         * or invocation ID. You may think this is redundant. Yes, that's redundant unless the
         * journal is corrupted. But when the journal is corrupted, especially, badly 'truncated',
         * then the below may fail.
         * See https://github.com/systemd/systemd/pull/29334#issuecomment-1736567951. */
        // if (advance_older)
        //         r = sd_journal_seek_tail(j);
        // else
        //         r = sd_journal_seek_head(j);
        // if (r < 0)
        //         return r;

        j.seek_head()?;

        j.next()?;

        // r = sd_journal_step_one(j, 0);
        // if (r < 0)
        //         return r;
        // if (r == 0) {
        //         log_debug("Whoopsie! We found a %s %s but can't read its first entry. "
        //                   "The journal seems to be corrupted. Ignoring the %s.",
        //                   log_id_type_to_string(type),
        //                   SD_ID128_TO_STRING(id.id),
        //                   log_id_type_to_string(type));
        //         goto try_again;
        // }

        let first_usec = j.timestamp_usec()?;
        // r = sd_journal_get_realtime_usec(j, advance_older ? &id.last_usec : &id.first_usec);
        // if (r < 0)
        //         return r;

        // /* Next, seek to the last occurrence of this boot or invocation ID. */
        // if (advance_older)
        //         r = sd_journal_seek_head(j);
        // else
        //         r = sd_journal_seek_tail(j);
        // if (r < 0)
        //         return r;

        // r = sd_journal_step_one(j, 0);
        // if (r < 0)
        //         return r;
        // if (r == 0) {
        //         log_debug("Whoopsie! We found a %s %s but can't read its last entry. "
        //                   "The journal seems to be corrupted. Ignoring the %s.",
        //                   log_id_type_to_string(type),
        //                   SD_ID128_TO_STRING(id.id),
        //                   log_id_type_to_string(type));
        //         goto try_again;
        // }

        j.seek_tail();
        j.previous();

        let last_usec = j.timestamp_usec()?;

        boots.push(Boot {
            // 0,
            boot_id: MyId128(boot_id),
            first: first_usec,
            last: last_usec,
            // total: 0,
        });

        j.match_flush();

        break;
        // r = sd_journal_get_realtime_usec(j, advance_older ? &id.first_usec : &id.last_usec);
        // if (r < 0)
        //         return r;

        // sd_journal_flush_matches(j);
        // *ret = id;
        // return 1;

        // try_again:
        //         /* Save the bad boot or invocation ID. */
        //         id_dup = newdup(sd_id128_t, &id.id, 1);
        //         if (!id_dup)
        //                 return -ENOMEM;

        //         r = set_ensure_consume(&broken_ids, &id128_hash_ops_free, id_dup);
        //         if (r < 0)
        //                 return r;

        //         /* Move to the previous position again. */
        //         r = set_matches_for_discover_id(j, type, boot_id, unit, previous_id);
        //         if (r < 0)
        //                 return r;

        //         if (advance_older)
        //                 r = sd_journal_seek_head(j);
        //         else
        //                 r = sd_journal_seek_tail(j);
        //         if (r < 0)
        //                 return r;

        //         r = sd_journal_step_one(j, 0);
        //         if (r < 0)
        //                 return r;
        //         if (r == 0)
        //                 return log_debug_errno(SYNTHETIC_ERRNO(ENODATA),
        //                                        "Whoopsie! Cannot seek to the last entry of %s %s.",
        //                                        log_id_type_to_string(type),
        //                                        SD_ID128_TO_STRING(previous_id));

        //         sd_journal_flush_matches(j);
    }
    Ok(())
}

fn set_matches_for_discover_id(j: &mut Journal, boot_id: Id128) {
    j.match_flush();
    j.match_add(KEY_BOOT_ID, boot_id.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time_handling::get_since_time;
    use crate::{errors::SystemdErrors, time_handling::TimestampStyle};

    #[test]
    #[ignore = "Too long"]
    fn test_get_boot() -> Result<(), SystemdErrors> {
        for (idx, boot) in list_boots()?.iter().enumerate() {
            let time = get_since_time(boot.first, TimestampStyle::Pretty);

            let time2 = get_since_time(boot.last, TimestampStyle::Pretty);

            println!("{idx} {} {} {}", boot.boot_id, time, time2);
        }

        Ok(())
    }
}
