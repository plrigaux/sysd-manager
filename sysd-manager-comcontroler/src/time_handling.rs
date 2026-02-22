use chrono::{DateTime, Local, TimeDelta, TimeZone, Utc};
use gettextrs::pgettext;
use glib;
use strum::EnumIter;

use std::{
    ffi::CStr,
    fmt::{Display, Write},
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, EnumIter)]
pub enum TimestampStyle {
    Pretty,
    PrettyUsec,
    Utc,
    UtcUsec,
    Unix,
    UnixUsec,
}

impl TimestampStyle {
    pub fn code(&self) -> &str {
        match self {
            TimestampStyle::Pretty => "Pretty",
            TimestampStyle::PrettyUsec => "Pretty usec",
            TimestampStyle::Utc => "UTC",
            TimestampStyle::UtcUsec => "UTC usec",
            TimestampStyle::Unix => "Unix",
            TimestampStyle::UnixUsec => "Unix usec",
        }
    }

    pub fn label(&self) -> String {
        match self {
            //time style option
            TimestampStyle::Pretty => pgettext("pref time style", "Pretty"),
            //time style option
            TimestampStyle::PrettyUsec => pgettext("pref time style", "Pretty usec"),
            //time style option
            TimestampStyle::Utc => pgettext("pref time style", "UTC"),
            //time style option
            TimestampStyle::UtcUsec => pgettext("pref time style", "UTC usec"),
            //time style option
            TimestampStyle::Unix => pgettext("pref time style", "Unix"),
            //time style option
            TimestampStyle::UnixUsec => pgettext("pref time style", "Unix usec"),
        }
    }

    pub fn details(&self) -> String {
        match self {
            //time style option tooltip
            TimestampStyle::Pretty => pgettext("pref time style", "Day YYYY-MM-DD HH:MM:SS TZ"),

            //time style option tooltip
            TimestampStyle::PrettyUsec => {
                pgettext("pref time style", "Day YYYY-MM-DD HH:MM:SS.000000 TZ")
            }

            //time style option tooltip
            TimestampStyle::Utc => pgettext("pref time style", "Day YYYY-MM-DD HH:MM:SS UTC"),

            //time style option tooltip
            TimestampStyle::UtcUsec => {
                pgettext("pref time style", "Day YYYY-MM-DD HH:MM:SS.000000 UTC")
            }

            //time style option tooltip
            TimestampStyle::Unix => pgettext("pref time style", "Seconds since the epoch"),

            //time style option tooltip
            TimestampStyle::UnixUsec => {
                pgettext("pref time style", "Micro seconds since the epoch")
            }
        }
    }

    pub fn usec_formated(&self, timestamp_usec: u64) -> String {
        match self {
            TimestampStyle::Pretty => pretty(timestamp_usec, "%a, %d %b %Y %H:%M:%S"),
            TimestampStyle::PrettyUsec => pretty(timestamp_usec, "%a, %d %b %Y %H:%M:%S%.6f"),
            TimestampStyle::Utc => {
                let since_local = get_date_utc(timestamp_usec);
                since_local.format("%a, %d %b %Y %H:%M:%S %Z").to_string()
            }
            TimestampStyle::UtcUsec => {
                let since_local = get_date_utc(timestamp_usec);
                since_local
                    .format("%a, %d %b %Y %H:%M:%S%.6f %Z")
                    .to_string()
            }
            TimestampStyle::Unix => {
                let timestamp_sec = timestamp_usec / USEC_PER_SEC;
                format!("@{timestamp_sec}")
            }
            TimestampStyle::UnixUsec => {
                format!("@{timestamp_usec}")
            }
        }
    }
}

impl Display for TimestampStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label())
    }
}

impl From<glib::GString> for TimestampStyle {
    fn from(level: glib::GString) -> Self {
        level.as_str().into()
    }
}

impl From<&str> for TimestampStyle {
    fn from(style: &str) -> Self {
        match style {
            "UTC" => TimestampStyle::Utc,
            "UTC usec" => TimestampStyle::UtcUsec,
            "Unix" => TimestampStyle::Unix,
            "Unix usec" => TimestampStyle::UnixUsec,
            "Pretty usec" => TimestampStyle::PrettyUsec,
            _ => TimestampStyle::Pretty,
        }
    }
}

impl From<i32> for TimestampStyle {
    fn from(style: i32) -> Self {
        match style {
            1 => TimestampStyle::PrettyUsec,
            2 => TimestampStyle::Utc,
            3 => TimestampStyle::UtcUsec,
            4 => TimestampStyle::Unix,
            5 => TimestampStyle::UnixUsec,
            _ => TimestampStyle::Pretty,
        }
    }
}

pub fn get_since_and_passed_time(
    timestamp_usec: u64,
    timestamp_style: TimestampStyle,
) -> (String, String) {
    let since = get_since_time(timestamp_usec, timestamp_style);

    (since, format_timestamp_relative_full(timestamp_usec))
}

pub fn get_since_time(timestamp_usec: u64, timestamp_style: TimestampStyle) -> String {
    timestamp_style.usec_formated(timestamp_usec)
}

fn pretty(timestamp_usec: u64, format: &str) -> String {
    let since_local = get_date_local(timestamp_usec);

    let time: libc::tm = localtime_or_gmtime_usec(timestamp_usec as i64, false);
    let time_zone = unsafe { CStr::from_ptr(time.tm_zone) };
    let time_zone = match time_zone.to_str() {
        Ok(s) => s,
        Err(_e) => &since_local.format("%Z").to_string(),
    };

    let formated_time = since_local.format(format).to_string();
    format!("{formated_time} {time_zone}")
}

fn get_date_local(timestamp_usec: u64) -> DateTime<Local> {
    let timestamp = get_date_utc(timestamp_usec);
    DateTime::from(timestamp)
}

fn get_date_utc(timestamp_usec: u64) -> DateTime<Utc> {
    match Utc.timestamp_micros(timestamp_usec as i64) {
        chrono::offset::LocalResult::Single(a) => a,
        chrono::offset::LocalResult::Ambiguous(a, _b) => a,
        chrono::offset::LocalResult::None => panic!("timestamp_opt None"),
    }
}

macro_rules! plur {
    ($num:expr, $single:expr, $plur:expr) => {{ if $num > 1 { $plur } else { $single } }};
}

macro_rules! plur_year {
    ($num:expr) => {{ plur!($num, "year", "years") }};
}

macro_rules! plur_month {
    ($num:expr) => {{ plur!($num, "month", "months") }};
}

macro_rules! plur_week {
    ($num:expr) => {{ plur!($num, "week", "weeks") }};
}

macro_rules! plur_day {
    ($num:expr) => {{ plur!($num, "day", "days") }};
}

#[macro_export]
macro_rules! swrite {

    ($out:expr, $($y:expr),+) => {
        // Call `find_min!` on the tail `$y`
        if let Err(e) = write!($out, $($y), +) {
            tracing::warn!("swrite error {:?}", e);
            //fall back
            let s = format!($($y), +);
            $out.push_str(&s);
    }}
}

#[macro_export]
macro_rules! timestamp_is_set {
    ($t:expr) => {
        $t > 0 && $t != u64::MAX
    };
}

const SEC_PER_MONTH: u64 = 2629800;
const SEC_PER_YEAR: u64 = 31_557_600;
pub const USEC_PER_SEC: u64 = 1_000_000;
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

pub fn format_timestamp_relative_full(timestamp_usec: u64) -> String {
    let since_time = get_date_local(timestamp_usec);

    let now = Local::now();

    let delta = now.signed_duration_since(since_time);

    format_timestamp_relative_full_delta(delta, true)
}

pub fn format_timestamp_relative_duration(
    begin_timestamp_usec: u64,
    finish_timestamp_usec: u64,
) -> String {
    let since_time = get_date_local(begin_timestamp_usec);

    let to_time = get_date_local(finish_timestamp_usec);

    let delta = to_time.signed_duration_since(since_time);

    format_timestamp_relative_full_delta(delta, false)
}

///from systemd
fn format_timestamp_relative_full_delta(delta: TimeDelta, show_suffix: bool) -> String {
    let is_ago = delta.num_seconds() >= 0;
    let suffix = if show_suffix {
        if is_ago { "ago" } else { "left" }
    } else {
        ""
    };

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
        swrite!(out, "{days} {} {suffix}", plur_day!(days));
    } else if d >= 25 * SEC_PER_HOUR {
        let hours = (d - SEC_PER_DAY) / SEC_PER_HOUR;
        swrite!(out, "1 {} {hours}h {suffix}", plur_day!(1));
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
        swrite!(out, "{mins}min {sec}s {suffix}");
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

    if duration == u64::MAX {
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

    let time_usec_ptr: *const libc::time_t = &(time_usec as libc::time_t);

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

///from systemd
pub fn calc_next_elapse(next_elapse_realtime: u64, next_elapse_monotonic: u64) -> u64 {
    let now_realtime = now_realtime();
    let now_monotonic = now_monotonic();

    if timestamp_is_set!(next_elapse_monotonic) {
        let converted = if next_elapse_monotonic > now_monotonic {
            now_realtime + (next_elapse_monotonic - now_monotonic)
        } else {
            now_realtime - (now_monotonic - next_elapse_monotonic)
        };

        if timestamp_is_set!(next_elapse_realtime) {
            converted.min(next_elapse_realtime)
        } else {
            converted
        }
    } else {
        next_elapse_realtime
    }
}

#[cfg(test)]
mod tests {

    use std::ffi::CStr;

    use chrono::{Duration, TimeDelta};

    use super::*;

    #[test]
    fn test_since() {
        let since = get_since_and_passed_time(1727116768682604, TimestampStyle::Pretty);
        println!("since {since:?}");
        let since = get_since_and_passed_time(1727116768682442, TimestampStyle::Pretty);
        println!("since {since:?}");
        let since = get_since_and_passed_time(1727116768682435, TimestampStyle::Pretty);
        println!("since {since:?}");
        let since = get_since_and_passed_time(1727413184243915, TimestampStyle::Pretty);
        println!("since {since:?}");
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

        println!("Duration between {now:?} and {tomorrow_midnight:?}: {duration:?}");
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

        println!(
            "{} ago",
            format_timestamp_relative_full_delta(duration, true)
        )
    }

    #[test]
    fn most_significant_duration_test() {
        let a = TimeDelta::minutes(1) + TimeDelta::seconds(30);
        println!("{a:?}");
        println!("{:?}", format_timestamp_relative_full_delta(a, true));

        let b = TimeDelta::minutes(2);
        println!("{b:?}");
        println!("{:?}", format_timestamp_relative_full_delta(b, true));

        let a = TimeDelta::minutes(10) + TimeDelta::seconds(30);
        println!("{a:?}");
        println!("{:?}", format_timestamp_relative_full_delta(a, true));

        let a = TimeDelta::minutes(9) + TimeDelta::seconds(30);
        println!("{a:?}");
        println!("{:?}", format_timestamp_relative_full_delta(a, true));
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
        println!(
            "{:?} {:?}",
            format_timestamp_relative_full_delta(d, true),
            d
        );
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

            let value = format_timestamp_relative_full_delta(delta, true);
            assert_eq!(value, time_output);
        }
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
        println!("time {asdf:?}");

        let c_str = unsafe { CStr::from_ptr(asdf.tm_zone) };
        println!("time zone {c_str:?}");

        let asdf = localtime_or_gmtime_usec(0, true);
        println!("time {asdf:?}");

        let c_str = unsafe { CStr::from_ptr(asdf.tm_zone) };
        println!("time zone {c_str:?}");
    }

    #[test]
    fn test_date_format() {
        let date = Local::now();

        //%Z: Since chrono is not aware of timezones beyond their offsets, this specifier only prints the offset when used for formatting. The timezone abbreviation will NOT be printed. See this issue for more information.
        println!("{}", date.format("%a, %d %b %Y %H:%M:%S %Z"));
        println!("{}", date.to_rfc2822());
        println!("{}", date.to_rfc3339());

        let date = Utc::now();

        //%Z: Since chrono is not aware of timezones beyond their offsets, this specifier only prints the offset when used for formatting. The timezone abbreviation will NOT be printed. See this issue for more information.
        println!("{}", date.format("%a, %d %b %Y %H:%M:%S %Z"));
        println!("{}", date.to_rfc2822());
        println!("{}", date.to_rfc3339());
    }

    #[test]
    fn test_casting() {
        let a: i64 = 0x0000_FFFF_FFFF_FFFF;

        let b: i32 = a as i32;

        println!("a {a:#x} b {b:#x}");
    }
}
