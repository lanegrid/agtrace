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

/// Helper to format with correct singular/plural
fn pluralize(count: i64, unit: &str, suffix: &str) -> String {
    if count == 1 {
        format!("{} {} {}", count, unit, suffix)
    } else {
        format!("{} {}s {}", count, unit, suffix)
    }
}

/// Format RFC3339 timestamp as relative time ("2 min ago", "yesterday")
pub fn format_relative_time(ts: &str) -> String {
    let parsed = match DateTime::parse_from_rfc3339(ts) {
        Ok(dt) => dt.with_timezone(&Utc),
        Err(_) => return ts.to_string(),
    };

    let now = Utc::now();
    let duration = now.signed_duration_since(parsed);

    let seconds = duration.num_seconds();
    let minutes = duration.num_minutes();
    let hours = duration.num_hours();
    let days = duration.num_days();

    if seconds < 60 {
        "just now".to_string()
    } else if minutes < 60 {
        pluralize(minutes, "min", "ago")
    } else if hours < 24 {
        pluralize(hours, "hour", "ago")
    } else if days == 1 {
        "yesterday".to_string()
    } else if days < 7 {
        pluralize(days, "day", "ago")
    } else if days < 30 {
        let weeks = days / 7;
        pluralize(weeks, "week", "ago")
    } else if days < 365 {
        let months = days / 30;
        pluralize(months, "month", "ago")
    } else {
        let years = days / 365;
        pluralize(years, "year", "ago")
    }
}
