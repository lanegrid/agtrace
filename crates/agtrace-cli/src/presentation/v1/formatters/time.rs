use chrono::{DateTime, Utc};

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

/// Format duration as "+2m5s" or "+30s", or None if < 2s
pub fn format_delta_time(duration: chrono::Duration) -> Option<String> {
    let seconds = duration.num_seconds();
    if seconds < 2 {
        return None;
    }

    if seconds < 60 {
        Some(format!("+{}s", seconds))
    } else {
        let minutes = seconds / 60;
        let remaining_secs = seconds % 60;
        if remaining_secs == 0 {
            Some(format!("+{}m", minutes))
        } else {
            Some(format!("+{}m{}s", minutes, remaining_secs))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_format_relative_time_recent() {
        let now = Utc::now();
        let ts = now.to_rfc3339();
        assert_eq!(format_relative_time(&ts), "just now");
    }

    #[test]
    fn test_format_delta_time_short() {
        let d = chrono::Duration::seconds(45);
        assert_eq!(format_delta_time(d), Some("+45s".to_string()));
    }

    #[test]
    fn test_format_delta_time_minutes() {
        let d = chrono::Duration::seconds(125);
        assert_eq!(format_delta_time(d), Some("+2m5s".to_string()));
    }

    #[test]
    fn test_format_delta_time_noise_filter() {
        let d = chrono::Duration::seconds(1);
        assert_eq!(format_delta_time(d), None);
    }
}
