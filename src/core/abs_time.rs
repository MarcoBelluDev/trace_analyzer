use chrono::NaiveDateTime;

use crate::types::absolute_time::AbsoluteTime;

pub(crate) fn from_line(line: &str) -> Option<AbsoluteTime> {
    // splits in words by whitespaces
    let mut parts = line.split_ascii_whitespace();

    // check first word
    if parts.next()? != "date" {
        return None;
    }

    // rebuild data
    let date_str: String = parts.collect::<Vec<_>>().join(" ");

    // Chrono parsing pattern
    let fmt: &str = "%a %b %d %I:%M:%S%.3f %P %Y";

    // parsing
    let naive_dt: NaiveDateTime = NaiveDateTime::parse_from_str(&date_str, fmt).ok()?;

    Some(AbsoluteTime {
        text: date_str,
        value: Some(naive_dt),
    })
}
