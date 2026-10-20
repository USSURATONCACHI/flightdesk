//! Humanising helpers for values that appear in BPMN: ISO 8601 durations, cycles and cron.

/// Render an ISO 8601 duration / repeating interval / cron-ish expression in words.
/// Returns `None` when the text is not something we recognise (e.g. an `${expression}`).
pub fn humanize_timer(text: &str) -> Option<String> {
    let t = text.trim();
    if t.starts_with("${") || t.starts_with("#{") {
        return None;
    }
    if let Some(rest) = t.strip_prefix('R') {
        // R3/PT10S  or R/PT1H or R3/2026-01-01T00:00:00Z/PT1H
        let mut parts = rest.splitn(3, '/');
        let count = parts.next().unwrap_or("");
        let a = parts.next().unwrap_or("");
        let b = parts.next();
        let dur = b.unwrap_or(a);
        let d = humanize_duration(dur)?;
        let times = if count.is_empty() { "forever".to_string() } else { format!("{count} times") };
        return Some(if let Some(start) = b.map(|_| a) { format!("every {d}, {times}, from {start}") } else { format!("every {d}, {times}") });
    }
    if t.starts_with('P') {
        return humanize_duration(t);
    }
    if t.split_whitespace().count() >= 5 && t.chars().all(|c| c.is_ascii_digit() || " */,-?LW#".contains(c) || c.is_ascii_alphabetic()) && t.contains(' ') {
        return Some(format!("cron: {t}"));
    }
    None
}

pub fn humanize_duration(s: &str) -> Option<String> {
    let s = s.trim();
    let body = s.strip_prefix('P')?;
    let (date, time) = match body.split_once('T') {
        Some((d, t)) => (d, Some(t)),
        None => (body, None),
    };
    let mut parts: Vec<String> = vec![];
    let mut push = |n: &str, unit: &str| {
        if let Ok(v) = n.parse::<f64>() {
            let unit = if v == 1.0 { unit.to_string() } else { format!("{unit}s") };
            let v = if v.fract() == 0.0 { format!("{}", v as i64) } else { format!("{v}") };
            parts.push(format!("{v} {unit}"));
        }
    };
    let mut scan = |seg: &str, units: &[(char, &str)]| {
        let mut num = String::new();
        for c in seg.chars() {
            if c.is_ascii_digit() || c == '.' {
                num.push(c);
            } else {
                if let Some((_, u)) = units.iter().find(|(k, _)| *k == c) {
                    push(&num, u);
                }
                num.clear();
            }
        }
    };
    scan(date, &[('Y', "year"), ('M', "month"), ('W', "week"), ('D', "day")]);
    if let Some(t) = time {
        scan(t, &[('H', "hour"), ('M', "minute"), ('S', "second")]);
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

/// Pretty-print an XML snippet with two-space indentation (good enough for the inspector).
pub fn pretty_xml(xml: &str) -> String {
    let mut out = String::with_capacity(xml.len() + xml.len() / 4);
    let mut depth: usize = 0;
    let mut i = 0;
    let bytes = xml.as_bytes();
    let mut pending_text = String::new();
    while i < bytes.len() {
        if bytes[i] == b'<' {
            let end = xml[i..].find('>').map(|e| i + e + 1).unwrap_or(bytes.len());
            let tag = &xml[i..end];
            let txt = pending_text.trim();
            let closing = tag.starts_with("</");
            let self_closing = tag.ends_with("/>") || tag.starts_with("<?") || tag.starts_with("<!");
            if closing {
                depth = depth.saturating_sub(1);
            }
            if !txt.is_empty() && closing {
                // inline text: <a>text</a>
                out.push_str(txt);
                out.push_str(tag);
                out.push('\n');
            } else {
                if !txt.is_empty() {
                    out.push_str(&"  ".repeat(depth));
                    out.push_str(txt);
                    out.push('\n');
                }
                out.push_str(&"  ".repeat(depth));
                out.push_str(tag);
                if closing || self_closing {
                    out.push('\n');
                }
            }
            if !closing && !self_closing {
                depth += 1;
                // peek: if next non-space is text followed by closing tag, keep on one line
                let rest = &xml[end..];
                let next_lt = rest.find('<').unwrap_or(rest.len());
                let between = rest[..next_lt].trim();
                if !between.is_empty() && rest[next_lt..].starts_with("</") {
                    // handled when we meet the closing tag (text + closing on the same line)
                } else {
                    out.push('\n');
                }
            }
            pending_text.clear();
            i = end;
        } else {
            let next = xml[i..].find('<').map(|e| i + e).unwrap_or(bytes.len());
            pending_text.push_str(&xml[i..next]);
            i = next;
        }
    }
    out.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn durations() {
        assert_eq!(humanize_duration("P2D").as_deref(), Some("2 days"));
        assert_eq!(humanize_duration("PT10S").as_deref(), Some("10 seconds"));
        assert_eq!(humanize_duration("P1DT2H30M").as_deref(), Some("1 day 2 hours 30 minutes"));
        assert_eq!(humanize_timer("R3/PT10S").as_deref(), Some("every 10 seconds, 3 times"));
        assert_eq!(humanize_timer("${dur}"), None);
        assert_eq!(humanize_timer("0 0 * * * ?").as_deref(), Some("cron: 0 0 * * * ?"));
    }
    #[test]
    fn xml_pretty() {
        let s = pretty_xml("<a x=\"1\"><b>t</b><c/></a>");
        assert_eq!(s, "<a x=\"1\">\n  <b>t</b>\n  <c/>\n</a>");
    }
}

#[cfg(test)]
mod more_tests {
    use super::*;

    #[test]
    fn timer_expressions_edge_cases() {
        assert_eq!(humanize_timer("R/PT1H").as_deref(), Some("every 1 hour, forever"));
        assert_eq!(humanize_timer("R2/2026-01-01T00:00:00Z/P1D").as_deref(), Some("every 1 day, 2 times, from 2026-01-01T00:00:00Z"));
        assert_eq!(humanize_duration("P1.5D").as_deref(), Some("1.5 days"));
        assert_eq!(humanize_duration("PT0S").as_deref(), Some("0 seconds"));
        assert_eq!(humanize_duration("P"), None);
        assert_eq!(humanize_duration("1h"), None, "non ISO strings are rejected rather than guessed");
        assert_eq!(humanize_timer("#{timerExpr}"), None);
        assert_eq!(humanize_timer("2026-01-01T10:00:00+03:00"), None, "a date is left to the caller to format");
    }

    #[test]
    fn pretty_xml_keeps_text_and_handles_nesting() {
        let out = pretty_xml(r#"<?xml version="1.0"?><a><b c="1"><d>text</d><e/></b></a>"#);
        assert_eq!(out, "<?xml version=\"1.0\"?>\n<a>\n  <b c=\"1\">\n    <d>text</d>\n    <e/>\n  </b>\n</a>");
        // an element whose text contains angle-bracket-free expressions survives intact
        let out = pretty_xml("<x><cond>${a > 1}</cond></x>");
        assert!(out.contains("${a > 1}"));
    }
}
