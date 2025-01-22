use chrono::DateTime;
use chrono::Local;
use chrono::TimeDelta;
use chrono::TimeZone;
use chrono::Utc;

use std::fmt::Write;

use super::construct_info::U64MAX;

pub fn get_since_and_passed_time(timestamp_u64: u64) -> (String, String) {
    let since_local = get_date_local(timestamp_u64);

    (
        //format!("{}", since_local.format("%a, %d %b %Y %H:%M:%S %Z")),
        since_local.to_rfc2822(),
        format_timestamp_relative_full(timestamp_u64),
    )
}

fn get_date_local(timestamp_u64: u64) -> DateTime<Local> {
    let since = match Utc.timestamp_micros(timestamp_u64 as i64) {
        chrono::offset::LocalResult::Single(a) => a,
        chrono::offset::LocalResult::Ambiguous(a, _b) => a,
        chrono::offset::LocalResult::None => panic!("timestamp_opt None"),
    };

    DateTime::from(since)
}

macro_rules! plur {
    ($num:expr, $single:expr, $plur:expr) => {{
        if $num > 1 {
            $plur
        } else {
            $single
        }
    }};
}

macro_rules! plur_year {
    ($num:expr) => {{
        plur!($num, "year", "years")
    }};
}

macro_rules! plur_month {
    ($num:expr) => {{
        plur!($num, "month", "months")
    }};
}

macro_rules! plur_week {
    ($num:expr) => {{
        plur!($num, "week", "weeks")
    }};
}

macro_rules! plur_day {
    ($num:expr) => {{
        plur!($num, "day", "days")
    }};
}

#[macro_export]
macro_rules! swrite {

    ($out:expr, $($y:expr),+) => {
        // Call `find_min!` on the tail `$y`
        if let Err(e) = write!($out, $($y), +) {
            log::warn!("swrite error {:?}", e);
            //fall back
            let s = format!($($y), +);
            $out.push_str(&s);
    }}
}

const SEC_PER_MONTH: u64 = 2629800;
const SEC_PER_YEAR: u64 = 31_557_600;
const USEC_PER_SEC: u64 = 1_000_000;
const USEC_PER_YEAR: u64 = 31_557_600 * USEC_PER_SEC;
const SEC_PER_DAY: u64 = 24 * SEC_PER_HOUR;
const SEC_PER_WEEK: u64 = SEC_PER_DAY * 7;
const SEC_PER_HOUR: u64 = 60 * 60;
pub const SEC_PER_MINUTE: u64 = 60;
pub const MSEC_PER_SEC: u64 = 1000;

const USEC_PER_MONTH: u64 = SEC_PER_MONTH * USEC_PER_SEC;
const USEC_PER_WEEK: u64 = SEC_PER_WEEK * USEC_PER_SEC;
const USEC_PER_DAY: u64 = SEC_PER_DAY * USEC_PER_SEC;
const USEC_PER_HOUR: u64 = SEC_PER_HOUR * USEC_PER_SEC;
const USEC_PER_MINUTE: u64 = SEC_PER_MINUTE * USEC_PER_SEC;
pub const USEC_PER_MSEC: u64 = 1000;
pub const NSEC_PER_USEC: u64 = 1_000;

fn format_timestamp_relative_full(timestamp_u64: u64) -> String {
    let since_time = get_date_local(timestamp_u64);

    let now = Local::now();

    let delta = now.signed_duration_since(since_time);

    format_timestamp_relative_full_delta(delta)
}

///from systemd
fn format_timestamp_relative_full_delta(delta: TimeDelta) -> String {
    let is_ago = delta.num_seconds() >= 0;
    let suffix = if is_ago { "ago" } else { "left" };

    let delta = delta.abs();

    let d = delta.num_seconds() as u64;

    let mut out = String::with_capacity(256);

    if d >= SEC_PER_YEAR {
        let years = d / SEC_PER_YEAR;
        let months = (d % SEC_PER_YEAR) / SEC_PER_MONTH;

        swrite!(
            out,
            "{} {} {} {} {suffix}",
            years,
            plur_year!(years),
            months,
            plur_month!(months)
        );
    } else if d >= SEC_PER_MONTH {
        let months = d / SEC_PER_MONTH;
        let days = (d % SEC_PER_MONTH) / SEC_PER_DAY;

        swrite!(
            out,
            "{} {} {} {} {suffix}",
            months,
            plur_month!(months),
            days,
            plur_day!(days)
        );
    } else if d >= SEC_PER_WEEK {
        let weeks = d / SEC_PER_WEEK;
        let days = (d % SEC_PER_WEEK) / SEC_PER_DAY;

        swrite!(
            out,
            "{} {} {} {} {suffix}",
            weeks,
            plur_week!(weeks),
            days,
            plur_day!(days)
        );
    } else if d >= 2 * SEC_PER_DAY {
        let days = d / SEC_PER_DAY;
        swrite!(out, "{days} days {suffix}");
    } else if d >= 25 * SEC_PER_HOUR {
        let hours = (d - SEC_PER_DAY) / SEC_PER_HOUR;
        swrite!(out, "1 days {hours}h {suffix}");
    } else if d >= 6 * SEC_PER_HOUR {
        let hours = d / SEC_PER_HOUR;
        swrite!(out, "{hours}h {suffix}");
    } else if d >= SEC_PER_HOUR {
        let hours = d / SEC_PER_HOUR;
        let mins = (d % SEC_PER_HOUR) / SEC_PER_MINUTE;
        swrite!(out, "{hours}h {mins}min {suffix}");
    } else if d >= 5 * SEC_PER_MINUTE {
        let mins = d / SEC_PER_MINUTE;
        swrite!(out, "{mins}min {suffix}");
    } else if d >= SEC_PER_MINUTE {
        let mins = d / SEC_PER_MINUTE;
        let sec = d % SEC_PER_MINUTE;
        swrite!(out, "{mins}min {suffix} {sec}s");
    } else if d > 0 {
        swrite!(out, "{d}s {suffix}");
    } else if let Some(d_us) = delta.num_microseconds() {
        let d_us = d_us as u64;
        if d_us >= USEC_PER_MSEC {
            let ms = d_us / USEC_PER_MSEC;
            swrite!(out, "{ms}ms {suffix}");
        } else if d_us > 0 {
            swrite!(out, "{d_us}μs {suffix}");
        } else {
            out.push_str("now");
        }
    } else {
        out.push_str("now");
    }

    out
}

pub fn now_monotonic() -> u64 {
    now(libc::CLOCK_MONOTONIC)
}

pub fn now_realtime() -> u64 {
    now(libc::CLOCK_REALTIME)
}

fn now(clock_id: i32) -> u64 {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let ret = unsafe { libc::clock_gettime(clock_id, &mut time) };
    assert!(ret == 0);

    time.tv_sec as u64 * USEC_PER_SEC + time.tv_nsec as u64 / NSEC_PER_USEC
}

pub fn format_timespan(mut duration: u64, accuracy: u64) -> String {
    let mut out = String::with_capacity(64);

    if duration == U64MAX {
        out.push_str("infinity");
        return out;
    }

    if duration == 0 {
        out.push('0');
        return out;
    }

    const TABLE: [(&str, u64); 9] = [
        ("y", USEC_PER_YEAR),
        ("month", USEC_PER_MONTH),
        ("w", USEC_PER_WEEK),
        ("d", USEC_PER_DAY),
        ("h", USEC_PER_HOUR),
        ("min", USEC_PER_MINUTE),
        ("s", USEC_PER_SEC),
        ("ms", USEC_PER_MSEC),
        ("μs", 1),
    ];

    let mut something = false;
    let mut done = false;

    for (suffix, unit_magnitute_in_usec) in TABLE {
        if duration == 0 {
            break;
        }

        if duration < accuracy && something {
            break;
        }

        if duration < unit_magnitute_in_usec {
            continue;
        }

        let a = duration / unit_magnitute_in_usec;
        let mut b = duration % unit_magnitute_in_usec;

        if duration < USEC_PER_MINUTE && b > 0 {
            let mut zero_padding = 0;

            let mut cc = unit_magnitute_in_usec;
            while cc > 1 {
                zero_padding += 1;
                cc /= 10;
            }

            let mut cc = accuracy;
            while cc > 1 {
                b /= 10;
                zero_padding -= 1;
                cc /= 10;
            }

            if zero_padding > 0 {
                let space_padding = if out.is_empty() { "" } else { " " };
                swrite!(out, "{space_padding}{a}.{:0zero_padding$}{suffix}", b);

                duration = 0;
                done = true;
            }
        }

        /* No? Then let's show it normally */
        if !done {
            let pad = if out.is_empty() { "" } else { " " };
            swrite!(out, "{pad}{a}{suffix}");
            duration = b;
        }

        something = true;
    }

    out
}

fn localtime_or_gmtime_usec(time_usec: i64, utc: bool) -> libc::tm {
    let layout = std::alloc::Layout::new::<libc::tm>();
    let time_usec_ptr: *const i64 = &time_usec;

    unsafe {
        let returned_time_struct = std::alloc::alloc(layout) as *mut libc::tm;

        if utc {
            libc::gmtime_r(time_usec_ptr, returned_time_struct);
        } else {
            libc::localtime_r(time_usec_ptr, returned_time_struct);
        }

        *returned_time_struct
    }
}

#[cfg(test)]
mod tests {

    use std::ffi::CStr;

    use chrono::{Duration, TimeDelta};

    use super::*;

    #[test]
    fn test_since() {
        let since = get_since_and_passed_time(1727116768682604);
        println!("since {:?}", since);
        let since = get_since_and_passed_time(1727116768682442);
        println!("since {:?}", since);
        let since = get_since_and_passed_time(1727116768682435);
        println!("since {:?}", since);
        let since = get_since_and_passed_time(1727413184243915);
        println!("since {:?}", since);
    }

    #[test]
    fn test_duration() {
        let now = Local::now();

        let tomorrow_midnight = (now + Duration::days(1))
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap();

        let tomorrow_midnight_local = tomorrow_midnight
            .and_local_timezone(Local)
            .earliest()
            .unwrap();

        let duration = tomorrow_midnight_local
            .signed_duration_since(now)
            .to_std()
            .unwrap();

        println!(
            "Duration between {:?} and {:?}: {:?}",
            now, tomorrow_midnight, duration
        );
    }

    #[test]
    fn test_duration2() {
        let prev = get_date_local(1727116768682604);

        let now = Local::now();

        let duration = now.signed_duration_since(prev);

        println!(
            "Duration between {:?} and {:?}: {:?}",
            prev,
            now,
            duration.to_std().unwrap()
        );

        println!("{} ago", format_timestamp_relative_full_delta(duration))
    }

    #[test]
    fn most_significant_duration_test() {
        let a = TimeDelta::minutes(1) + TimeDelta::seconds(30);
        println!("{:?}", a);
        println!("{:?}", format_timestamp_relative_full_delta(a));

        let b = TimeDelta::minutes(2);
        println!("{:?}", b);
        println!("{:?}", format_timestamp_relative_full_delta(b));

        let a = TimeDelta::minutes(10) + TimeDelta::seconds(30);
        println!("{:?}", a);
        println!("{:?}", format_timestamp_relative_full_delta(a));

        let a = TimeDelta::minutes(9) + TimeDelta::seconds(30);
        println!("{:?}", a);
        println!("{:?}", format_timestamp_relative_full_delta(a));
    }

    #[test]
    fn test_duration_() {
        /*         ActiveEnterTimestamp	1727500436962647
        ActiveEnterTimestampMonotonic	383605378536
        ActiveExitTimestamp	1727501504134907
        ActiveExitTimestampMonotonic	384672550797 */

        let enter = get_date_local(1727500436962647);
        let exit = get_date_local(1727501504134907);

        let d = exit.signed_duration_since(enter);
        println!("{:?} {:?}", format_timestamp_relative_full_delta(d), d);
    }

    #[test]
    fn test_format_timestamp_relative_full() {
        //println!("{} - {}", now_realtime(), (USEC_PER_YEAR + USEC_PER_MONTH));
        const SEC_PER_YEAR_I: i64 = SEC_PER_YEAR as i64;
        const SEC_PER_MONTH_I: i64 = SEC_PER_MONTH as i64;
        const SEC_PER_DAY_I: i64 = SEC_PER_DAY as i64;
        const SEC_PER_WEEK_I: i64 = SEC_PER_WEEK as i64;

        const USEC_PER_SEC: i64 = 1_000_000;
        const USEC_PER_YEAR_I: i64 = SEC_PER_YEAR_I * USEC_PER_SEC;
        const USEC_PER_MONTH: i64 = SEC_PER_MONTH_I * USEC_PER_SEC;
        const USEC_PER_DAY: i64 = SEC_PER_DAY_I * USEC_PER_SEC;
        const USEC_PER_WEEK: i64 = SEC_PER_WEEK_I * USEC_PER_SEC;

        let tests: Vec<(i64, &str)> = vec![
            ((USEC_PER_YEAR_I + USEC_PER_MONTH), "1 year 1 month ago"),
            (
                -(USEC_PER_YEAR_I + (1.5 * USEC_PER_MONTH as f64) as i64),
                "1 year 1 month left",
            ),
            (
                (USEC_PER_YEAR_I + (2 * USEC_PER_MONTH)),
                "1 year 2 months ago",
            ),
            (
                (2 * USEC_PER_YEAR_I + USEC_PER_MONTH),
                "2 years 1 month ago",
            ),
            (
                (2 * USEC_PER_YEAR_I + 2 * USEC_PER_MONTH),
                "2 years 2 months ago",
            ),
            ((USEC_PER_MONTH + USEC_PER_DAY), "1 month 1 day ago"),
            ((USEC_PER_MONTH + 2 * USEC_PER_DAY), "1 month 2 days ago"),
            ((2 * USEC_PER_MONTH + USEC_PER_DAY), "2 months 1 day ago"),
            (
                (2 * USEC_PER_MONTH + 2 * USEC_PER_DAY),
                "2 months 2 days ago",
            ),
            /* Weeks and days */
            ((USEC_PER_WEEK + USEC_PER_DAY), "1 week 1 day ago"),
            ((USEC_PER_WEEK + 2 * USEC_PER_DAY), "1 week 2 days ago"),
            ((2 * USEC_PER_WEEK + USEC_PER_DAY), "2 weeks 1 day ago"),
            ((2 * USEC_PER_WEEK + 2 * USEC_PER_DAY), "2 weeks 2 days ago"),
            (3 * 1000, "3ms ago"),
            (2, "2μs ago"),
            (0, "now"),
        ];

        for (time_us, time_output) in tests {
            let delta = TimeDelta::new(
                time_us / USEC_PER_SEC,
                (time_us % USEC_PER_SEC) as u32 * 1000,
            )
            .expect("Time delta not supposed to have bondary issues");

            let value = format_timestamp_relative_full_delta(delta);
            assert_eq!(value, time_output);
        }
    }

    #[test]
    fn test_date_format() {
        let date = Local::now();

        //%Z: Since chrono is not aware of timezones beyond their offsets, this specifier only prints the offset when used for formatting. The timezone abbreviation will NOT be printed. See this issue for more information.
        println!("{}", date.format("%a, %d %b %Y %H:%M:%S %Z"));
        println!("{}", date.to_rfc2822());
        println!("{}", date.to_rfc3339());
    }

    #[test]
    fn test_format_timespan() {
        println!("{:?}", format_timespan(4 * USEC_PER_YEAR, MSEC_PER_SEC));
        println!("{:?}", format_timespan(4 * USEC_PER_MONTH, MSEC_PER_SEC));
        println!("{:?}", format_timespan(4 * USEC_PER_WEEK, MSEC_PER_SEC));
        println!("{:?}", format_timespan(4 * USEC_PER_DAY, MSEC_PER_SEC));
        println!("{:?}", format_timespan(4 * USEC_PER_HOUR, MSEC_PER_SEC));
        println!("{:?}", format_timespan(4 * USEC_PER_MINUTE, MSEC_PER_SEC));
        println!("{:?}", format_timespan(4 * USEC_PER_SEC, MSEC_PER_SEC));
        println!("{:?}", format_timespan(4 * USEC_PER_MSEC, MSEC_PER_SEC));

        println!(
            "{:?}",
            format_timespan(
                4 * USEC_PER_DAY + 4 * USEC_PER_HOUR + 4 * USEC_PER_SEC,
                MSEC_PER_SEC
            )
        );
        println!(
            "{:?}",
            format_timespan(
                4 * USEC_PER_DAY + 4 * USEC_PER_HOUR + 4 * USEC_PER_SEC + 4 * USEC_PER_MSEC,
                MSEC_PER_SEC
            )
        );
        println!(
            "{:?}",
            format_timespan(
                4 * USEC_PER_DAY + 4 * USEC_PER_HOUR + 4 * USEC_PER_SEC + 4 * USEC_PER_MSEC + 4,
                MSEC_PER_SEC
            )
        );

        println!(
            "{:?}",
            format_timespan(
                4 * USEC_PER_DAY + 4 * USEC_PER_HOUR + 4 * USEC_PER_SEC + 50 * USEC_PER_MSEC,
                MSEC_PER_SEC
            )
        );
    }

    #[test]
    fn test_localtime_or_gmtime_usec() {
        let asdf = localtime_or_gmtime_usec(0, false);
        println!("time {:?}", asdf);

        let c_str = unsafe { CStr::from_ptr(asdf.tm_zone) };
        println!("time zone {:?}", c_str);
    }
}
