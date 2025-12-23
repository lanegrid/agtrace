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
        format!("{} min ago", minutes)
    } else if hours < 24 {
        format!("{} hours ago", hours)
    } else if days == 1 {
        "yesterday".to_string()
    } else if days < 7 {
        format!("{} days ago", days)
    } else if days < 30 {
        let weeks = days / 7;
        format!("{} weeks ago", weeks)
    } else if days < 365 {
        let months = days / 30;
        format!("{} months ago", months)
    } else {
        let years = days / 365;
        format!("{} years ago", years)
    }
}
