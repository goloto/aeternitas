use std::time::{SystemTime, UNIX_EPOCH};

pub struct TimeFormating {}

const SECONDS_IN_WEEK: i64 = 60 * 60 * 24 * 7;
const SECONDS_IN_DAY: u64 = 60 * 60 * 24;
const SECONDS_IN_HOUR: u64 = 60 * 60;
const SECONDS_IN_MINUTE: u64 = 60;

impl TimeFormating {
    fn euclid(divine: u64, divider: u64) -> (u64, u64) {
        let result = divine.div_euclid(divider);
        let remainder = divine.rem_euclid(divider);

        (result, remainder)
    }

    pub fn from_seconds(seconds: u64) -> String {
        if seconds > SECONDS_IN_DAY {
            let (days, days_remainder) = TimeFormating::euclid(seconds, SECONDS_IN_DAY);
            let (hours, hours_remainder) = TimeFormating::euclid(days_remainder, SECONDS_IN_HOUR);
            let (minutes, minutes_remainder) =
                TimeFormating::euclid(hours_remainder, SECONDS_IN_MINUTE);

            format!("{days}d {hours}h {minutes}m {minutes_remainder}s")
        } else if seconds > SECONDS_IN_HOUR {
            let (hours, hours_remainder) = TimeFormating::euclid(seconds, SECONDS_IN_HOUR);
            let (minutes, minutes_remainder) =
                TimeFormating::euclid(hours_remainder, SECONDS_IN_MINUTE);

            format!("{hours}h {minutes}m {minutes_remainder}s")
        } else if seconds > SECONDS_IN_MINUTE {
            let (minutes, minutes_remainder) = TimeFormating::euclid(seconds, SECONDS_IN_MINUTE);

            format!("{minutes}m {minutes_remainder}s")
        } else {
            format!("{seconds}s")
        }
    }

    pub fn from_seconds_short(seconds: u64) -> String {
        if seconds > SECONDS_IN_HOUR {
            let (hours, hours_remainder) = TimeFormating::euclid(seconds, SECONDS_IN_HOUR);
            let (minutes, _minutes_remainder) =
                TimeFormating::euclid(hours_remainder, SECONDS_IN_MINUTE);

            format!("{hours}h {minutes}m")
        } else if seconds > SECONDS_IN_MINUTE {
            let (minutes, _minutes_remainder) = TimeFormating::euclid(seconds, SECONDS_IN_MINUTE);

            format!("{minutes}m")
        } else {
            format!("0m")
        }
    }

    pub fn current_time() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Could not calculate current time")
            .as_secs() as i64
    }

    pub fn diff_from_now(time: i64) -> i64 {
        TimeFormating::current_time() - time
    }

    pub fn current_week() -> (i64, i64) {
        let current_time = TimeFormating::current_time();
        let week_start =
            current_time - current_time.rem_euclid(SECONDS_IN_WEEK) - 4 * SECONDS_IN_DAY as i64;
        let week_end = week_start + SECONDS_IN_WEEK;

        (week_start, week_end)
    }

    pub fn previous_week() -> (i64, i64) {
        let (current_week_start, _current_week_end) = TimeFormating::current_week();
        let previous_week_start = current_week_start - SECONDS_IN_WEEK;

        (previous_week_start, current_week_start)
    }
}
