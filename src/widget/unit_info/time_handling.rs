use chrono::DateTime;
use chrono::Local;
use chrono::TimeDelta;
use chrono::TimeZone;
use chrono::Utc;

use std::fmt::Write;

/* pub fn get_duration(duration: u64) -> String {
    let duration_str = String::new();
    duration_str
} */

pub fn get_since_and_passed_time(timestamp_u64: u64) -> (String, String) {
    let since_local = get_since_date_local(timestamp_u64);

    //let now = Local::now();

    //let duration = now.signed_duration_since(since_local);

    (
        since_local.to_rfc2822(),
        format_timestamp_relative_full(timestamp_u64),
    )
}

fn get_since_date_local(timestamp_u64: u64) -> DateTime<Local> {
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
/* const USEC_PER_SEC: u64 = 1_000_000;
const USEC_PER_YEAR: u64 = 31_557_600 * USEC_PER_SEC; */
const SEC_PER_DAY: u64 = 24 * SEC_PER_HOUR;
const SEC_PER_WEEK: u64 = SEC_PER_DAY * 7;
const SEC_PER_HOUR: u64 = 60 * 60;
const USEC_PER_MSEC: u64 = 1000;
const SEC_PER_MINUTE: u64 = 60;

fn format_timestamp_relative_full(timestamp_u64: u64) -> String {
    let since_local = get_since_date_local(timestamp_u64);

    let now = Local::now();

    let delta = now.signed_duration_since(since_local);

    format_timestamp_relative_full_delta(delta)
}

///from systemd
fn format_timestamp_relative_full_delta(delta: TimeDelta) -> String {
    let is_ago = delta.num_seconds() >= 0;
    let delta = delta.abs();

    let suffix = if is_ago { "ago" } else { "left" };

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
            swrite!(out, "{d_us}us {suffix}");
        } else {
            out.push_str("now");
        }
    } else {
        out.push_str("now");
    }

    out
}
/*
fn most_significant_duration(duration: Duration) -> String {
    let days = duration.num_days();

    let mut day_duration = if days > 0 {
        let plur = if days == 1 { "" } else { "s" };

        format!("{days} day{plur}")
    } else {
        String::new()
    };

    if days > 3 {
        return day_duration;
    } else if days > 0 {
        day_duration.push(' ');
    }

    let mut hours = duration.num_hours();
    if hours > 0 {
        hours -= days * 24;

        return format!("{day_duration}{hours}h",);
    }

    let minutes = duration.num_minutes();

    match minutes {
        0 => {
            format!("{}s", duration.num_seconds())
        }
        1..10 => {
            let seconds = duration.num_seconds() % 60;
            format!("{minutes}min {}s", seconds)
        }
        _ => {
            format!("{minutes} minutes")
        }
    }
}
*/
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

    const USEC_PER_SEC: u64 = 1_000_000;
    const NSEC_PER_USEC: u64 = 1_000;

    time.tv_sec as u64 * USEC_PER_SEC + time.tv_nsec as u64 / NSEC_PER_USEC
}

#[cfg(test)]
mod tests {

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
        let prev = get_since_date_local(1727116768682604);

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

        let enter = get_since_date_local(1727500436962647);
        let exit = get_since_date_local(1727501504134907);

        let d = exit.signed_duration_since(enter);
        println!("{:?} {:?}", format_timestamp_relative_full_delta(d), d);
    }

    const USEC_PER_SEC: u64 = 1_000_000;
    const USEC_PER_YEAR: u64 = SEC_PER_YEAR * USEC_PER_SEC;
    const USEC_PER_MONTH: u64 = SEC_PER_MONTH * USEC_PER_SEC;
    const USEC_PER_DAY: u64 = SEC_PER_DAY * USEC_PER_SEC;
    const USEC_PER_WEEK: u64 = SEC_PER_WEEK * USEC_PER_SEC;
    //const USEC_PER_MIN : u64 = USEC_PER_SEC * 60;

    #[test]
    fn test_format_timestamp_relative_full() {
        //println!("{} - {}", now_realtime(), (USEC_PER_YEAR + USEC_PER_MONTH));

        let tests = vec![
            (
                now_realtime() - (USEC_PER_YEAR + USEC_PER_MONTH),
                "1 year 1 month ago",
            ),
            (
                now_realtime() + (USEC_PER_YEAR + (1.5 * USEC_PER_MONTH as f64) as u64),
                "1 year 1 month left",
            ),
            (
                now_realtime() - (USEC_PER_YEAR + (2 * USEC_PER_MONTH)),
                "1 year 2 months ago",
            ),
            (
                now_realtime() - (2 * USEC_PER_YEAR + USEC_PER_MONTH),
                "2 years 1 month ago",
            ),
            (
                now_realtime() - (2 * USEC_PER_YEAR + 2 * USEC_PER_MONTH),
                "2 years 2 months ago",
            ),
            (
                now_realtime() - (USEC_PER_MONTH + USEC_PER_DAY),
                "1 month 1 day ago",
            ),
            (
                now_realtime() - (USEC_PER_MONTH + 2 * USEC_PER_DAY),
                "1 month 2 days ago",
            ),
            (
                now_realtime() - (2 * USEC_PER_MONTH + USEC_PER_DAY),
                "2 months 1 day ago",
            ),
            (
                now_realtime() - (2 * USEC_PER_MONTH + 2 * USEC_PER_DAY),
                "2 months 2 days ago",
            ),
            /* Weeks and days */
             (
                now_realtime() - (USEC_PER_WEEK + USEC_PER_DAY),
                "1 week 1 day ago",
            ),
            (
                now_realtime() - (USEC_PER_WEEK + 2 * USEC_PER_DAY),
                "1 week 2 days ago",
            ),
            (
                now_realtime() - (2 * USEC_PER_WEEK + USEC_PER_DAY),
                "2 weeks 1 day ago",
            ),
            (
                now_realtime() - (2 * USEC_PER_WEEK + 2 * USEC_PER_DAY),
                "2 weeks 2 days ago",
            ), 
/*             (
                now_realtime() - 7 * USEC_PER_MIN,
                "now",
            ),  */
        ];

        for (time_us, time_output) in tests {
            let value = format_timestamp_relative_full(time_us);
            assert_eq!(value, time_output);
        }
    }
}
