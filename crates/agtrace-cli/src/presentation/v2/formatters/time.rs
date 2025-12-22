use chrono::{DateTime, Utc};

pub fn format_time(ts: DateTime<Utc>) -> String {
    ts.with_timezone(&chrono::Local)
        .format("%H:%M:%S")
        .to_string()
}

pub fn format_duration(start: DateTime<Utc>, end: DateTime<Utc>) -> String {
    let duration = end.signed_duration_since(start);

    let minutes = duration.num_minutes();
    let seconds = duration.num_seconds() % 60;

    if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}
