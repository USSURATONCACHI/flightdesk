//! Small formatting helpers shared by all places.

use chrono::{DateTime, FixedOffset, Local, Utc};

/// Parse Camunda's `2026-10-20T09:48:00.854+0300` (also accepts `Z` and `+03:00`).
pub fn parse_time(s: &str) -> Option<DateTime<FixedOffset>> {
    DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.3f%z")
        .or_else(|_| DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%z"))
        .or_else(|_| DateTime::parse_from_rfc3339(s))
        .ok()
}

/// Camunda wire format for dates we send.
pub fn format_camunda(t: DateTime<Local>) -> String {
    t.format("%Y-%m-%dT%H:%M:%S%.3f%z").to_string()
}

pub fn fmt_local(s: &str) -> String {
    match parse_time(s) {
        Some(t) => t.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S").to_string(),
        None => s.to_string(),
    }
}

pub fn fmt_local_short(s: &str) -> String {
    match parse_time(s) {
        Some(t) => {
            let l = t.with_timezone(&Local);
            if l.date_naive() == Local::now().date_naive() {
                l.format("%H:%M:%S").to_string()
            } else {
                l.format("%m-%d %H:%M").to_string()
            }
        }
        None => s.to_string(),
    }
}

/// "3 s ago", "in 2 h 13 m", "12 d ago".
pub fn fmt_relative(s: &str) -> String {
    let Some(t) = parse_time(s) else { return s.to_string() };
    relative_from_now(t.with_timezone(&Utc))
}

pub fn relative_from_now(t: DateTime<Utc>) -> String {
    let d = Utc::now().signed_duration_since(t);
    let secs = d.num_seconds();
    let (past, a) = if secs >= 0 { (true, secs) } else { (false, -secs) };
    let body = human_secs(a);
    if past {
        format!("{body} ago")
    } else {
        format!("in {body}")
    }
}

pub fn human_secs(a: i64) -> String {
    if a < 60 {
        format!("{a} s")
    } else if a < 3600 {
        format!("{} m {} s", a / 60, a % 60)
    } else if a < 86400 {
        format!("{} h {} m", a / 3600, (a % 3600) / 60)
    } else {
        format!("{} d {} h", a / 86400, (a % 86400) / 3600)
    }
}

pub fn human_millis(ms: i64) -> String {
    if ms < 1000 {
        format!("{ms} ms")
    } else {
        human_secs(ms / 1000)
    }
}

pub fn human_duration(d: std::time::Duration) -> String {
    let s = d.as_secs();
    if s < 1 {
        "just now".into()
    } else {
        format!("{} ago", human_secs(s as i64))
    }
}

pub fn short_uuid(id: &str) -> String {
    camunda::short_id(id)
}

pub fn is_uuid(s: &str) -> bool {
    s.len() == 36 && s.chars().enumerate().all(|(i, c)| if [8, 13, 18, 23].contains(&i) { c == '-' } else { c.is_ascii_hexdigit() })
}

/// Truncate for table cells with an ellipsis.
pub fn ellipsize(s: &str, max: usize) -> String {
    let s = s.replace('\n', " ");
    if s.chars().count() <= max {
        s
    } else {
        let t: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{t}…")
    }
}

/// Compact preview of a variable value for tables.
pub fn value_preview(v: &serde_json::Value, type_: Option<&str>) -> String {
    match v {
        serde_json::Value::Null => "null".into(),
        serde_json::Value::String(s) => {
            if matches!(type_, Some("Json") | Some("Object") | Some("Xml")) {
                ellipsize(s, 120)
            } else {
                ellipsize(s, 120)
            }
        }
        other => ellipsize(&other.to_string(), 120),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_camunda_dates() {
        assert!(parse_time("2026-10-20T09:48:00.854+0300").is_some());
        assert!(parse_time("2026-10-20T09:48:00+03:00").is_some());
        assert!(parse_time("2026-10-20T06:48:00Z").is_some());
        assert_eq!(human_secs(7384), "2 h 3 m");
        assert!(is_uuid("30f8285a-cc52-11f1-8f3a-c6104db61f7c"));
        assert!(!is_uuid("ORD-1001"));
    }
}
