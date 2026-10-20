//! The instance search language (DESIGN.md §7.1): parse ↔ print, mapping to Camunda queries,
//! and autocompletion.

use camunda::{HistoricProcessInstanceQuery, ProcessInstanceQuery, Sorting, VariableCriterion};
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    Like,
}

impl Op {
    pub fn as_str(self) -> &'static str {
        match self {
            Op::Eq => "=",
            Op::Neq => "!=",
            Op::Gt => ">",
            Op::Gte => ">=",
            Op::Lt => "<",
            Op::Lte => "<=",
            Op::Like => "~",
        }
    }
    fn camunda(self) -> &'static str {
        match self {
            Op::Eq => "eq",
            Op::Neq => "neq",
            Op::Gt => "gt",
            Op::Gte => "gteq",
            Op::Lt => "lt",
            Op::Lte => "lteq",
            Op::Like => "like",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Term {
    pub field: String,
    pub op: Op,
    /// true when written with ':' (default operator)
    pub colon: bool,
    pub value: String,
    pub quoted: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Query {
    pub terms: Vec<Term>,
    pub bare: Vec<String>,
    pub warnings: Vec<String>,
}

pub const FIELDS: &[(&str, &str)] = &[
    ("def", "process definition key"),
    ("ver", "definition version (with def)"),
    ("id", "process instance id"),
    ("bk", "business key (* wildcard)"),
    ("act", "currently at activity id"),
    ("exec", "has executed activity id (history)"),
    ("var.", "variable: var.name>10, var.flag=true, var.s~\"a*\""),
    ("state", "active | suspended | running | finished | completed | terminated | any"),
    ("inc", "incidents: yes | failedJob | failedExternalTask | ~\"message\""),
    ("started", "started>2026-10-01, started<2026-10-20T12:00"),
    ("finished", "finished>… / finished<… (history)"),
    ("by", "start user id"),
    ("super", "super process instance id"),
    ("tenant", "tenant id"),
];

pub const KNOWN_FIELDS: &[&str] = &["def", "ver", "id", "bk", "act", "exec", "state", "inc", "started", "finished", "by", "super", "tenant", "var"];

/// Split into whitespace-separated tokens honouring double/single quotes.
pub fn tokenize(s: &str) -> Vec<(usize, usize, String)> {
    let mut out = vec![];
    let mut cur = String::new();
    let mut start = 0;
    let mut in_q: Option<char> = None;
    let mut started = false;
    for (i, c) in s.char_indices() {
        match in_q {
            Some(q) => {
                cur.push(c);
                if c == q {
                    in_q = None;
                }
            }
            None => {
                if c.is_whitespace() {
                    if started {
                        out.push((start, i, std::mem::take(&mut cur)));
                        started = false;
                    }
                } else {
                    if !started {
                        start = i;
                        started = true;
                    }
                    if c == '"' || c == '\'' {
                        in_q = Some(c);
                    }
                    cur.push(c);
                }
            }
        }
    }
    if started {
        out.push((start, s.len(), cur));
    }
    out
}

fn unquote(v: &str) -> (String, bool) {
    let b = v.as_bytes();
    if b.len() >= 2 && ((b[0] == b'"' && b[b.len() - 1] == b'"') || (b[0] == b'\'' && b[b.len() - 1] == b'\'')) {
        (v[1..v.len() - 1].to_string(), true)
    } else {
        (v.to_string(), false)
    }
}

pub fn parse(s: &str) -> Query {
    let mut q = Query::default();
    for (_, _, tok) in tokenize(s) {
        // find operator position (first of : = != > >= < <= ~) outside quotes; field part has no spaces
        let ops = [">=", "<=", "!=", ":", "=", ">", "<", "~"];
        let mut best: Option<(usize, &str)> = None;
        if !tok.starts_with('"') && !tok.starts_with('\'') {
            for o in ops {
                if let Some(p) = tok.find(o) {
                    if p > 0 && best.map(|(bp, _)| p < bp || (p == bp && o.len() > best.unwrap().1.len())).unwrap_or(true) {
                        best = Some((p, o));
                    }
                }
            }
        }
        match best {
            Some((p, o)) => {
                let field = tok[..p].to_string();
                let raw = &tok[p + o.len()..];
                let (value, quoted) = unquote(raw);
                let (op, colon) = match o {
                    ":" => (Op::Eq, true),
                    "=" => (Op::Eq, false),
                    "!=" => (Op::Neq, false),
                    ">" => (Op::Gt, false),
                    ">=" => (Op::Gte, false),
                    "<" => (Op::Lt, false),
                    "<=" => (Op::Lte, false),
                    _ => (Op::Like, false),
                };
                let base = field.split('.').next().unwrap_or("").to_string();
                if !KNOWN_FIELDS.contains(&base.as_str()) {
                    q.warnings.push(format!("unknown field “{field}”"));
                }
                q.terms.push(Term { field, op, colon, value, quoted });
            }
            None => {
                let (v, _) = unquote(&tok);
                q.bare.push(v);
            }
        }
    }
    q
}

pub fn print(q: &Query) -> String {
    let mut parts: Vec<String> = vec![];
    for t in &q.terms {
        let v = if t.quoted || t.value.contains(' ') || t.value.is_empty() { format!("\"{}\"", t.value) } else { t.value.clone() };
        let op = if t.colon { ":".to_string() } else { t.op.as_str().to_string() };
        parts.push(format!("{}{}{}", t.field, op, v));
    }
    parts.extend(q.bare.iter().map(|b| if b.contains(' ') { format!("\"{b}\"") } else { b.clone() }));
    parts.join(" ")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Runtime,
    History,
}

pub fn needs_history(q: &Query) -> bool {
    q.terms.iter().any(|t| match t.field.as_str() {
        "exec" | "started" | "finished" | "by" => true,
        "state" => matches!(t.value.to_lowercase().as_str(), "finished" | "completed" | "terminated" | "any" | "all" | "ended"),
        _ => false,
    })
}

fn like_value(v: &str) -> String {
    let mut s = v.replace('*', "%");
    if !s.contains('%') {
        s = format!("%{s}%");
    }
    s
}

fn typed_value(v: &str, quoted: bool) -> Value {
    if quoted {
        return Value::String(v.to_string());
    }
    if let Ok(i) = v.parse::<i64>() {
        return Value::Number(i.into());
    }
    if let Ok(f) = v.parse::<f64>() {
        if let Some(n) = serde_json::Number::from_f64(f) {
            return Value::Number(n);
        }
    }
    match v {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        "null" => Value::Null,
        _ => Value::String(v.to_string()),
    }
}

/// Normalise a user date (`2026-10-01`, `2026-10-01T12:00`, `today`, `yesterday`, `-2h`, `-3d`) to Camunda's format.
pub fn date_value(v: &str) -> Option<String> {
    use chrono::{Local, NaiveDate, NaiveDateTime, TimeZone};
    let v = v.trim();
    let now = Local::now();
    let dt = match v {
        "now" => now,
        "today" => now.date_naive().and_hms_opt(0, 0, 0).and_then(|n| Local.from_local_datetime(&n).single())?,
        "yesterday" => (now - chrono::Duration::days(1)).date_naive().and_hms_opt(0, 0, 0).and_then(|n| Local.from_local_datetime(&n).single())?,
        _ => {
            if let Some(rest) = v.strip_prefix('-') {
                let (num, unit) = rest.split_at(rest.len().saturating_sub(1));
                let n: i64 = num.parse().ok()?;
                let d = match unit {
                    "m" => chrono::Duration::minutes(n),
                    "h" => chrono::Duration::hours(n),
                    "d" => chrono::Duration::days(n),
                    "w" => chrono::Duration::weeks(n),
                    _ => return None,
                };
                now - d
            } else if let Ok(d) = NaiveDate::parse_from_str(v, "%Y-%m-%d") {
                Local.from_local_datetime(&d.and_hms_opt(0, 0, 0)?).single()?
            } else if let Ok(d) = NaiveDateTime::parse_from_str(v, "%Y-%m-%dT%H:%M") {
                Local.from_local_datetime(&d).single()?
            } else if let Ok(d) = NaiveDateTime::parse_from_str(v, "%Y-%m-%dT%H:%M:%S") {
                Local.from_local_datetime(&d).single()?
            } else if let Some(d) = crate::util::parse_time(v) {
                d.with_timezone(&Local)
            } else {
                return None;
            }
        }
    };
    Some(crate::util::format_camunda(dt))
}

fn variables(q: &Query) -> Option<Vec<VariableCriterion>> {
    let mut v = vec![];
    for t in &q.terms {
        if let Some(name) = t.field.strip_prefix("var.") {
            let (op, value) = if t.op == Op::Like || (t.op == Op::Eq && !t.quoted && t.value.contains('*')) {
                ("like".to_string(), Value::String(like_value(&t.value)))
            } else {
                (t.op.camunda().to_string(), typed_value(&t.value, t.quoted))
            };
            v.push(VariableCriterion { name: name.to_string(), operator: op, value });
        }
    }
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}

fn first<'a>(q: &'a Query, field: &str) -> Option<&'a Term> {
    q.terms.iter().find(|t| t.field == field)
}

fn all<'a>(q: &'a Query, field: &'a str) -> impl Iterator<Item = &'a Term> + 'a {
    q.terms.iter().filter(move |t| t.field == field)
}

/// Resolve `def:key` + `ver:n` to a definition id using the known definition list.
fn definition_id(q: &Query, defs: &[camunda::ProcessDefinition]) -> Option<String> {
    let ver = first(q, "ver")?;
    let key = first(q, "def").map(|t| t.value.clone()).or_else(|| ver.value.split('@').nth(1).map(String::from))?;
    let v: i64 = ver.value.split('@').next()?.parse().ok()?;
    defs.iter().find(|d| d.key == key && d.version == v).map(|d| d.id.clone())
}

pub fn build_runtime(q: &Query, defs: &[camunda::ProcessDefinition]) -> ProcessInstanceQuery {
    let mut r = ProcessInstanceQuery { sorting: Some(vec![Sorting { sort_by: "instanceId".into(), sort_order: "asc".into() }]), ..Default::default() };
    if let Some(id) = definition_id(q, defs) {
        r.process_definition_id = Some(id);
    } else if let Some(t) = first(q, "def") {
        if let Some((k, v)) = t.value.split_once('@') {
            r.process_definition_id = defs.iter().find(|d| d.key == k && d.version.to_string() == v).map(|d| d.id.clone());
            if r.process_definition_id.is_none() {
                r.process_definition_key = Some(k.to_string());
            }
        } else if t.value.contains('*') {
            r.process_definition_key_in = Some(defs.iter().filter(|d| wildcard(&t.value, &d.key)).map(|d| d.key.clone()).collect());
        } else {
            r.process_definition_key = Some(t.value.clone());
        }
    }
    let ids: Vec<String> = all(q, "id").map(|t| t.value.clone()).chain(q.bare.iter().filter(|b| crate::util::is_uuid(b)).cloned()).collect();
    if !ids.is_empty() {
        r.process_instance_ids = Some(ids);
    }
    if let Some(t) = first(q, "bk") {
        if t.op == Op::Like || t.value.contains('*') {
            r.business_key_like = Some(like_value(&t.value));
        } else {
            r.business_key = Some(t.value.clone());
        }
    } else if let Some(b) = q.bare.iter().find(|b| !crate::util::is_uuid(b)) {
        r.business_key_like = Some(like_value(b));
    }
    let acts: Vec<String> = all(q, "act").map(|t| t.value.clone()).collect();
    if !acts.is_empty() {
        r.activity_id_in = Some(acts);
    }
    for t in all(q, "inc") {
        match (t.op, t.value.to_lowercase().as_str()) {
            (Op::Like, _) => r.incident_message_like = Some(like_value(&t.value)),
            (_, "yes" | "true" | "any") => r.with_incident = Some(true),
            (_, "no" | "false") => r.with_incident = Some(false),
            _ => r.incident_type = Some(t.value.clone()),
        }
    }
    if let Some(t) = first(q, "state") {
        match t.value.to_lowercase().as_str() {
            "active" | "running" => r.active = Some(true),
            "suspended" => r.suspended = Some(true),
            _ => {}
        }
    }
    if let Some(t) = first(q, "super") {
        r.super_process_instance = Some(t.value.clone());
    }
    let tenants: Vec<String> = all(q, "tenant").map(|t| t.value.clone()).collect();
    if !tenants.is_empty() {
        r.tenant_id_in = Some(tenants);
    }
    r.variables = variables(q);
    if r.variables.is_some() {
        r.variable_names_ignore_case = Some(true);
    }
    r
}

pub fn build_history(q: &Query, defs: &[camunda::ProcessDefinition]) -> HistoricProcessInstanceQuery {
    let mut h = HistoricProcessInstanceQuery { sorting: Some(vec![Sorting { sort_by: "startTime".into(), sort_order: "desc".into() }]), ..Default::default() };
    if let Some(id) = definition_id(q, defs) {
        h.process_definition_id = Some(id);
    } else if let Some(t) = first(q, "def") {
        if let Some((k, v)) = t.value.split_once('@') {
            h.process_definition_id = defs.iter().find(|d| d.key == k && d.version.to_string() == v).map(|d| d.id.clone());
            if h.process_definition_id.is_none() {
                h.process_definition_key = Some(k.to_string());
            }
        } else if t.value.contains('*') {
            h.process_definition_key_in = Some(defs.iter().filter(|d| wildcard(&t.value, &d.key)).map(|d| d.key.clone()).collect());
        } else {
            h.process_definition_key = Some(t.value.clone());
        }
    }
    let ids: Vec<String> = all(q, "id").map(|t| t.value.clone()).chain(q.bare.iter().filter(|b| crate::util::is_uuid(b)).cloned()).collect();
    if ids.len() == 1 {
        h.process_instance_id = Some(ids[0].clone());
    } else if !ids.is_empty() {
        h.process_instance_ids = Some(ids);
    }
    if let Some(t) = first(q, "bk") {
        if t.op == Op::Like || t.value.contains('*') {
            h.process_instance_business_key_like = Some(like_value(&t.value));
        } else {
            h.process_instance_business_key = Some(t.value.clone());
        }
    } else if let Some(b) = q.bare.iter().find(|b| !crate::util::is_uuid(b)) {
        h.process_instance_business_key_like = Some(like_value(b));
    }
    let acts: Vec<String> = all(q, "act").map(|t| t.value.clone()).collect();
    if !acts.is_empty() {
        h.active_activity_id_in = Some(acts);
    }
    let execs: Vec<String> = all(q, "exec").map(|t| t.value.clone()).collect();
    if !execs.is_empty() {
        h.executed_activity_id_in = Some(execs);
    }
    for t in all(q, "inc") {
        match (t.op, t.value.to_lowercase().as_str()) {
            (Op::Like, _) => h.incident_message_like = Some(like_value(&t.value)),
            (_, "yes" | "true" | "any") => h.with_incidents = Some(true),
            (_, "open") => h.incident_status = Some("open".into()),
            (_, "resolved") => h.incident_status = Some("resolved".into()),
            _ => h.incident_type = Some(t.value.clone()),
        }
    }
    match first(q, "state").map(|t| t.value.to_lowercase()) {
        Some(s) => match s.as_str() {
            "active" => h.active = Some(true),
            "running" | "unfinished" => h.unfinished = Some(true),
            "suspended" => h.suspended = Some(true),
            "finished" | "ended" => h.finished = Some(true),
            "completed" => h.completed = Some(true),
            "terminated" => h.externally_terminated = Some(true),
            "internally_terminated" => h.internally_terminated = Some(true),
            _ => {}
        },
        None => {
            // History mode without an explicit state: everything (running and finished).
        }
    }
    for t in all(q, "started") {
        if let Some(d) = date_value(&t.value) {
            match t.op {
                Op::Gt | Op::Gte => h.started_after = Some(d),
                Op::Lt | Op::Lte => h.started_before = Some(d),
                _ => h.started_after = Some(d),
            }
        }
    }
    for t in all(q, "finished") {
        if let Some(d) = date_value(&t.value) {
            match t.op {
                Op::Gt | Op::Gte => h.finished_after = Some(d),
                Op::Lt | Op::Lte => h.finished_before = Some(d),
                _ => h.finished_after = Some(d),
            }
            if h.finished.is_none() {
                h.finished = Some(true);
            }
        }
    }
    if let Some(t) = first(q, "by") {
        h.started_by = Some(t.value.clone());
    }
    if let Some(t) = first(q, "super") {
        h.super_process_instance_id = Some(t.value.clone());
    }
    let tenants: Vec<String> = all(q, "tenant").map(|t| t.value.clone()).collect();
    if !tenants.is_empty() {
        h.tenant_id_in = Some(tenants);
    }
    h.variables = variables(q);
    if h.variables.is_some() {
        h.variable_names_ignore_case = Some(true);
    }
    h
}

fn wildcard(pattern: &str, s: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    let mut pos = 0;
    for (i, p) in parts.iter().enumerate() {
        if p.is_empty() {
            continue;
        }
        match s[pos..].find(p) {
            Some(f) => {
                if i == 0 && f != 0 {
                    return false;
                }
                pos += f + p.len();
            }
            None => return false,
        }
    }
    pattern.ends_with('*') || pos == s.len()
}

// ---------------- autocompletion ----------------

#[derive(Clone, Debug, PartialEq)]
pub struct Suggestion {
    pub label: String,
    pub hint: String,
    /// Text that replaces the current token.
    pub insert: String,
    pub range: (usize, usize),
}

pub struct SuggestCtx<'a> {
    pub def_keys: &'a [(String, String)],
    /// (id, name) of activities of the definition named in the query, if any.
    pub activities: &'a [(String, String)],
    pub variable_names: &'a [String],
    pub users: &'a [String],
}

pub fn suggest(text: &str, caret: usize, ctx: &SuggestCtx) -> Vec<Suggestion> {
    let caret = caret.min(text.len());
    let toks = tokenize(text);
    let (start, end, tok) = match toks.iter().find(|(s, e, _)| *s <= caret && caret <= *e) {
        Some(t) => t.clone(),
        None => (caret, caret, String::new()),
    };
    let tok = tok[..caret.saturating_sub(start).min(tok.len())].to_string();
    let mut out = vec![];
    let mk = |label: String, hint: String, insert: String| Suggestion { label, hint, insert, range: (start, end) };
    let lower = tok.to_lowercase();
    let sep = tok.find(|c: char| [':', '=', '>', '<', '~', '!'].contains(&c));
    match sep {
        None => {
            if let Some(prefix) = tok.strip_prefix("var.") {
                for v in ctx.variable_names.iter().filter(|v| v.to_lowercase().starts_with(&prefix.to_lowercase())).take(12) {
                    out.push(mk(format!("var.{v}"), "variable".into(), format!("var.{v}=")));
                }
            } else {
                for (f, d) in FIELDS {
                    if f.starts_with(&lower) || lower.is_empty() {
                        let insert = if *f == "var." { "var.".to_string() } else { format!("{f}:") };
                        out.push(mk(f.to_string(), d.to_string(), insert));
                    }
                }
            }
        }
        Some(p) => {
            let field = &tok[..p];
            let val = tok[p..].trim_start_matches(|c: char| [':', '=', '>', '<', '~', '!'].contains(&c)).to_lowercase();
            let op = &tok[p..tok.len() - (tok.len() - p - (tok[p..].len() - tok[p..].trim_start_matches(|c: char| [':', '=', '>', '<', '~', '!'].contains(&c)).len()))];
            let finish = |v: &str| format!("{field}{op}{}", if v.contains(' ') { format!("\"{v}\"") } else { v.to_string() });
            match field {
                "def" => {
                    for (k, n) in ctx.def_keys.iter().filter(|(k, n)| k.to_lowercase().starts_with(&val) || n.to_lowercase().contains(&val)).take(12) {
                        out.push(mk(k.clone(), n.clone(), finish(k) + " "));
                    }
                }
                "act" | "exec" => {
                    for (id, n) in ctx.activities.iter().filter(|(id, n)| id.to_lowercase().contains(&val) || n.to_lowercase().contains(&val)).take(14) {
                        out.push(mk(id.clone(), n.clone(), finish(id) + " "));
                    }
                }
                "state" => {
                    for (v, d) in [("active", "running, not suspended"), ("suspended", ""), ("running", "unfinished (history)"), ("finished", "ended (history)"), ("completed", "ended normally (history)"), ("terminated", "cancelled (history)"), ("any", "running and finished (history)")] {
                        if v.starts_with(&val) {
                            out.push(mk(v.into(), d.into(), finish(v) + " "));
                        }
                    }
                }
                "inc" => {
                    for (v, d) in [("yes", "has open incidents"), ("failedJob", "incident type"), ("failedExternalTask", "incident type"), ("open", "history: open"), ("resolved", "history: resolved")] {
                        if v.to_lowercase().starts_with(&val) {
                            out.push(mk(v.into(), d.into(), finish(v) + " "));
                        }
                    }
                }
                "started" | "finished" => {
                    for (v, d) in [("today", ""), ("yesterday", ""), ("-1h", "last hour"), ("-24h", "last day"), ("-7d", "last week"), ("2026-01-31", "a date"), ("2026-01-31T14:30", "a date and time")] {
                        if v.starts_with(&val) || val.is_empty() {
                            out.push(mk(v.into(), d.into(), finish(v) + " "));
                        }
                    }
                }
                "by" => {
                    for u in ctx.users.iter().filter(|u| u.to_lowercase().starts_with(&val)).take(10) {
                        out.push(mk(u.clone(), "user".into(), finish(u) + " "));
                    }
                }
                _ => {}
            }
        }
    }
    out
}

/// Human description of a parsed query for the toolbar.
pub fn describe(q: &Query) -> String {
    let mut parts: Vec<String> = vec![];
    for t in &q.terms {
        parts.push(format!("{} {} {}", t.field, if t.colon { "=" } else { t.op.as_str() }, t.value));
    }
    for b in &q.bare {
        parts.push(if crate::util::is_uuid(b) { format!("id = {b}") } else { format!("business key ~ {b}") });
    }
    parts.join(" · ")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_terms() {
        let q = parse(r#"def:order_process var.amount>1000 bk:ORD-* state:active inc~"timeout x" "free text" 30f8285a-cc52-11f1-8f3a-c6104db61f7c"#);
        assert_eq!(q.terms.len(), 5);
        assert_eq!(q.terms[1].field, "var.amount");
        assert_eq!(q.terms[1].op, Op::Gt);
        assert_eq!(q.terms[4].value, "timeout x");
        assert!(q.terms[4].quoted);
        assert_eq!(q.bare, vec!["free text", "30f8285a-cc52-11f1-8f3a-c6104db61f7c"]);
        assert!(q.warnings.is_empty());
        let printed = print(&q);
        assert_eq!(parse(&printed), q);
    }
    #[test]
    fn maps_runtime() {
        let q = parse("def:order_process var.amount>1000 bk:ORD-2* inc:yes act:Task_Approve");
        assert!(!needs_history(&q));
        let r = build_runtime(&q, &[]);
        assert_eq!(r.process_definition_key.as_deref(), Some("order_process"));
        assert_eq!(r.business_key_like.as_deref(), Some("ORD-2%"));
        assert_eq!(r.with_incident, Some(true));
        assert_eq!(r.activity_id_in, Some(vec!["Task_Approve".to_string()]));
        let v = r.variables.unwrap();
        assert_eq!(v[0].operator, "gt");
        assert_eq!(v[0].value, serde_json::json!(1000));
    }
    #[test]
    fn maps_history() {
        let q = parse("exec:Task_Validate state:finished started>2026-10-01 var.customer=\"Big Corp 1\"");
        assert!(needs_history(&q));
        let h = build_history(&q, &[]);
        assert_eq!(h.executed_activity_id_in, Some(vec!["Task_Validate".to_string()]));
        assert_eq!(h.finished, Some(true));
        assert!(h.started_after.unwrap().starts_with("2026-10-01T00:00:00"));
        assert_eq!(h.variables.unwrap()[0].value, serde_json::json!("Big Corp 1"));
    }
    #[test]
    fn suggestions() {
        let keys = vec![("order_process".to_string(), "Order process".to_string())];
        let ctx = SuggestCtx { def_keys: &keys, activities: &[], variable_names: &["amount".into()], users: &[] };
        let s = suggest("de", 2, &ctx);
        assert_eq!(s[0].insert, "def:");
        let s = suggest("def:ord", 7, &ctx);
        assert_eq!(s[0].insert, "def:order_process ");
        let s = suggest("var.am", 6, &ctx);
        assert_eq!(s[0].insert, "var.amount=");
        assert!(wildcard("ord_*", "ord_main") && !wildcard("ord_*", "xord_"));
    }
}
