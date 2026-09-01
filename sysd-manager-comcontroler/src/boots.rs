use crate::{errors::SystemdErrors, journal_data::Boot};
use std::{
    fmt::{self, Write},
    hash::{Hash, Hasher},
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

pub(super) fn list_boots_new_to_old() -> Result<Vec<Boot>, SystemdErrors> {
    info!("Starting journal-logger list boot");
    let mut journal_reader = OpenOptions::default()
        .system(true)
        .local_only(true)
        .open()
        .expect("Could not open journal");

    let mut boots: Vec<Boot> = Vec::with_capacity(200);
    // let mut index = 1;

    //Find first boot
    //position to the oldest one
    journal_reader.seek_tail(); //seek the newest

    journal_reader.match_flush();

    info!("Get boots");

    const MAX: usize = i32::MAX as usize;
    let mut previous_id = Id128::default();
    loop {
        if journal_reader.previous()? == 0 {
            journal_reader.match_flush(); //End of journal
            return Ok(boots);
        }

        let (_, boot_id) = journal_reader.monotonic_timestamp()?;

        if previous_id == boot_id {
            continue;
        }

        set_matches_for_discover_id(&mut journal_reader, boot_id);

        journal_reader.seek_tail()?;
        journal_reader.previous()?;

        let last_usec = journal_reader.timestamp_usec()?;

        journal_reader.seek_head();
        journal_reader.next();

        let first_usec = journal_reader.timestamp_usec()?;

        boots.push(Boot {
            // 0,
            boot_id: MyId128(boot_id),
            first: first_usec,
            last: last_usec,
            // total: 0,
        });

        if boots.len() >= MAX {
            break;
        }

        journal_reader.match_flush();

        previous_id = boot_id;
    }

    Ok(boots)
}

fn set_matches_for_discover_id(j: &mut Journal, boot_id: Id128) {
    j.match_flush();
    j.match_add(KEY_BOOT_ID, boot_id.to_string());
}

#[cfg(test)]
mod tests {
    use test_base::init_logs;

    use super::*;
    use crate::time_handling::get_since_time;
    use crate::{errors::SystemdErrors, time_handling::TimestampStyle};

    #[test]
    #[ignore = "Too long"]
    fn test_get_boot2() -> Result<(), SystemdErrors> {
        init_logs();

        info!("start logs");
        let boots = list_boots_old_to_new()?;

        info!("Fonding boots olds to new");
        for (idx, boot) in boots.iter().enumerate() {
            let time = get_since_time(boot.first, TimestampStyle::Pretty);

            let time2 = get_since_time(boot.last, TimestampStyle::Pretty);

            info!("{idx} {} {} {}", boot.boot_id, time, time2);
        }

        Ok(())
    }

    #[test]
    #[ignore = "Too long"]
    fn test_get_boot3() -> Result<(), SystemdErrors> {
        init_logs();

        info!("Fonding boots new to olds");
        let boots = list_boots_new_to_old()?;

        info!("boots {}", boots.len());
        for (idx, boot) in boots.iter().enumerate() {
            let time = get_since_time(boot.first, TimestampStyle::Pretty);

            let time2 = get_since_time(boot.last, TimestampStyle::Pretty);

            info!("{idx} {} {} {}", boot.boot_id, time, time2);
        }

        Ok(())
    }

    pub(super) fn list_boots_old_to_new() -> Result<Vec<Boot>, SystemdErrors> {
        info!("Starting journal-logger list boot");
        let mut journal_reader = OpenOptions::default()
            .system(true)
            .local_only(true)
            .open()
            .expect("Could not open journal");

        let mut boots: Vec<Boot> = Vec::with_capacity(200);
        // let mut index = 1;

        //Find first boot
        //position to the oldest one
        journal_reader.seek_head(); //seek the oldest

        journal_reader.match_flush();

        info!("Get boots");

        const MAX: usize = i32::MAX as usize;
        let mut previous_id = Id128::default();
        loop {
            if journal_reader.next()? == 0 {
                journal_reader.match_flush(); //End of journal
                return Ok(boots);
            }

            let (_, boot_id) = journal_reader.monotonic_timestamp()?;

            if previous_id == boot_id {
                continue;
            }

            set_matches_for_discover_id(&mut journal_reader, boot_id);

            journal_reader.seek_head()?;

            journal_reader.next()?;

            let first_usec = journal_reader.timestamp_usec()?;

            journal_reader.seek_tail();
            journal_reader.previous();

            let last_usec = journal_reader.timestamp_usec()?;

            boots.push(Boot {
                // 0,
                boot_id: MyId128(boot_id),
                first: first_usec,
                last: last_usec,
                // total: 0,
            });

            if boots.len() >= MAX {
                break;
            }
            journal_reader.match_flush();

            previous_id = boot_id;
        }

        Ok(boots)
    }
}
