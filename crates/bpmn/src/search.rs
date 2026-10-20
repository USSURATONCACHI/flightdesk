//! Element search inside a diagram (Ctrl+F). Pure functions over the model.

use crate::model::*;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchOptions {
    pub case_sensitive: bool,
    /// Restrict to categories (see `ElementKind::category`) when non-empty.
    pub categories: Vec<String>,
    pub include_flows: bool,
}

/// Every searchable text of an element: id, name, type, documentation, implementation
/// attributes, expressions, extension values.
pub fn element_haystack(m: &Model, e: &Element) -> Vec<(&'static str, String)> {
    let mut v: Vec<(&'static str, String)> = vec![("id", e.id.clone()), ("type", e.kind.label())];
    if let Some(n) = &e.name {
        v.push(("name", n.clone()));
    }
    if let Some(d) = &e.documentation {
        v.push(("documentation", d.clone()));
    }
    for (k, val) in &e.attrs {
        if k != "id" && k != "name" {
            v.push(("attribute", format!("{k}={val}")));
        }
    }
    for d in &e.event_defs {
        match d {
            EventDef::Timer { time_date, time_duration, time_cycle } => {
                for t in [time_date, time_duration, time_cycle].into_iter().flatten() {
                    v.push(("timer", t.clone()));
                }
            }
            EventDef::Message { message_ref, name } => {
                v.push(("message", name.clone().or(message_ref.clone()).unwrap_or_default()));
            }
            EventDef::Signal { signal_ref, name } => v.push(("signal", name.clone().or(signal_ref.clone()).unwrap_or_default())),
            EventDef::Error { name, code, .. } => {
                v.push(("error", format!("{} {}", name.clone().unwrap_or_default(), code.clone().unwrap_or_default())));
            }
            EventDef::Escalation { name, code, .. } => {
                v.push(("escalation", format!("{} {}", name.clone().unwrap_or_default(), code.clone().unwrap_or_default())));
            }
            EventDef::Conditional { condition: Some(c), .. } => v.push(("condition", c.text.clone())),
            EventDef::Link { name: Some(n) } => v.push(("link", n.clone())),
            _ => {}
        }
    }
    if let Some(s) = &e.script {
        v.push(("script", s.clone()));
    }
    ext_haystack(&e.ext, &mut v);
    if let Some(mi) = &e.multi_instance {
        for t in [&mi.collection, &mi.element_variable, &mi.cardinality, &mi.completion_condition].into_iter().flatten() {
            v.push(("multi-instance", t.clone()));
        }
    }
    let _ = m;
    v
}

pub fn flow_haystack(f: &Flow) -> Vec<(&'static str, String)> {
    let mut v: Vec<(&'static str, String)> = vec![("id", f.id.clone()), ("type", f.kind.label().to_string())];
    if let Some(n) = &f.name {
        v.push(("name", n.clone()));
    }
    if let Some(c) = &f.condition {
        v.push(("condition", c.text.clone()));
    }
    if let Some(d) = &f.documentation {
        v.push(("documentation", d.clone()));
    }
    ext_haystack(&f.ext, &mut v);
    v
}

fn ext_haystack(ext: &Extensions, v: &mut Vec<(&'static str, String)>) {
    for p in ext.inputs.iter().chain(ext.outputs.iter()) {
        v.push(("io", format!("{} {}", p.name, param_text(&p.value))));
    }
    for l in &ext.listeners {
        let imp = l.class.clone().or(l.expression.clone()).or(l.delegate_expression.clone()).unwrap_or_default();
        v.push(("listener", format!("{} {imp}", l.event.clone().unwrap_or_default())));
    }
    for (k, val) in &ext.properties {
        v.push(("property", format!("{k}={val}")));
    }
    for f in &ext.fields {
        v.push(("field", format!("{} {}", f.name, f.string_value.clone().or(f.expression.clone()).unwrap_or_default())));
    }
    for f in &ext.form_fields {
        v.push(("form", format!("{} {}", f.id, f.label.clone().unwrap_or_default())));
    }
}

pub fn param_text(v: &ParamValue) -> String {
    match v {
        ParamValue::Text(t) => t.clone(),
        ParamValue::Script { source, .. } => source.clone(),
        ParamValue::List(items) => items.iter().map(param_text).collect::<Vec<_>>().join(" "),
        ParamValue::Map(entries) => entries.iter().map(|(k, v)| format!("{k} {}", param_text(v))).collect::<Vec<_>>().join(" "),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Match {
    pub target: Ref,
    /// Which field matched, e.g. "name", "attribute".
    pub field: &'static str,
    pub snippet: String,
}

/// Case-insensitive substring search with all whitespace-separated words required.
pub fn search(m: &Model, query: &str, opts: &SearchOptions) -> Vec<Match> {
    let words: Vec<String> = query
        .split_whitespace()
        .map(|w| if opts.case_sensitive { w.to_string() } else { w.to_lowercase() })
        .collect();
    if words.is_empty() {
        return vec![];
    }
    let norm = |s: &str| if opts.case_sensitive { s.to_string() } else { s.to_lowercase() };
    let mut out = vec![];
    let test = |hay: &[(&'static str, String)]| -> Option<(&'static str, String)> {
        // Every word must appear in some field; report the field of the first word.
        let mut first: Option<(&'static str, String)> = None;
        for w in &words {
            let hit = hay.iter().find(|(_, t)| norm(t).contains(w.as_str()));
            match hit {
                Some((f, t)) => {
                    if first.is_none() {
                        first = Some((f, t.clone()));
                    }
                }
                None => return None,
            }
        }
        first
    };
    for (i, e) in m.elements.iter().enumerate() {
        if !e.bounds.is_valid() {
            continue;
        }
        if !opts.categories.is_empty() && !opts.categories.iter().any(|c| c == e.kind.category()) {
            continue;
        }
        if let Some((field, snippet)) = test(&element_haystack(m, e)) {
            out.push(Match { target: Ref::Element(i), field, snippet });
        }
    }
    if opts.include_flows && (opts.categories.is_empty() || opts.categories.iter().any(|c| c == "flow")) {
        for (i, f) in m.flows.iter().enumerate() {
            if f.waypoints.is_empty() {
                continue;
            }
            if let Some((field, snippet)) = test(&flow_haystack(f)) {
                out.push(Match { target: Ref::Flow(i), field, snippet });
            }
        }
    }
    // Reading order for elements first, then flows.
    let pos: std::collections::HashMap<ElementIdx, usize> = m.reading_order.iter().enumerate().map(|(p, &i)| (i, p)).collect();
    out.sort_by_key(|mt| match mt.target {
        Ref::Element(i) => (0, pos.get(&i).copied().unwrap_or(usize::MAX)),
        Ref::Flow(i) => (1, i),
    });
    out
}
