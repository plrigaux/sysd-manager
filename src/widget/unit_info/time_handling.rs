use chrono::DateTime;
use chrono::Local;
use chrono::TimeZone;
use chrono::Utc;

/* pub fn get_duration(duration: u64) -> String {
    let duration_str = String::new();
    duration_str
} */

pub fn get_since(timestamp_u64: u64) -> String {
    let since = match Utc.timestamp_micros(timestamp_u64 as i64) {
        chrono::offset::LocalResult::Single(a) => a,
        chrono::offset::LocalResult::Ambiguous(a, _b) => a,
        chrono::offset::LocalResult::None => panic!("timestamp_opt None"),
    };

    let since_local : DateTime<Local> = DateTime::from(since);
    //ActiveEnterTimestamp	1727116768682604
    since_local.to_rfc2822()
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_since() {
        let since = get_since(1727116768682604);
        println!("since {}", since);
        let since = get_since(1727116768682442);
        println!("since {}", since);
        let since = get_since(1727116768682435);
        println!("since {}", since);
        let since = get_since(1727413184243915);
        println!("since {}", since);
    }
}
