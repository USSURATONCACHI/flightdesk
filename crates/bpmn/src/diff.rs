//! Semantic differences between two BPMN models: which elements and flows were added, removed,
//! changed (with every changed property) or only moved, plus process- and file-level changes.
//! Also a merged model for drawing removals as ghosts, and line/inline text diff helpers.

use crate::model::*;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum Status {
    Added,
    Removed,
    /// At least one semantic property differs (the layout may differ too).
    Changed,
    /// Only the layout differs (bounds, label bounds, waypoints).
    Moved,
}

/// One property that differs. `None` means the property is absent on that side.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PropChange {
    pub key: String,
    pub old: Option<String>,
    pub new: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ItemDiff {
    /// Id in the new model (the old id when removed).
    pub id: String,
    /// The old id when it differs (matched by name instead of id).
    pub old_id: Option<String>,
    pub is_flow: bool,
    pub old: Option<Ref>,
    pub new: Option<Ref>,
    pub status: Status,
    /// Semantic changes; empty for added, removed and moved items.
    pub props: Vec<PropChange>,
    pub layout_changed: bool,
    /// "Name (Kind)" for lists.
    pub label: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Counts {
    pub added: usize,
    pub removed: usize,
    pub changed: usize,
    pub moved: usize,
    /// Process- and file-level property changes.
    pub document: usize,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct ModelDiff {
    /// Items that differ: new items in reading order (each element followed by its outgoing
    /// flows), then removed items in the old model's order.
    pub items: Vec<ItemDiff>,
    /// Process-level (`process <id>: …`), root definitions (`message <id>` …) and
    /// `definitions <attr>` changes.
    pub document: Vec<PropChange>,
    #[serde(skip)]
    by_new: HashMap<Ref, usize>,
    #[serde(skip)]
    by_old: HashMap<Ref, usize>,
    #[serde(skip)]
    old_to_new: HashMap<Ref, Ref>,
    #[serde(skip)]
    new_to_old: HashMap<Ref, Ref>,
}

impl ModelDiff {
    pub fn for_new(&self, r: Ref) -> Option<&ItemDiff> {
        self.by_new.get(&r).map(|&i| &self.items[i])
    }
    pub fn for_old(&self, r: Ref) -> Option<&ItemDiff> {
        self.by_old.get(&r).map(|&i| &self.items[i])
    }
    /// The matching new item of an old one (unchanged items included).
    pub fn new_of_old(&self, r: Ref) -> Option<Ref> {
        self.old_to_new.get(&r).copied()
    }
    pub fn old_of_new(&self, r: Ref) -> Option<Ref> {
        self.new_to_old.get(&r).copied()
    }
    pub fn counts(&self) -> Counts {
        let mut c = Counts { document: self.document.len(), ..Default::default() };
        for it in &self.items {
            match it.status {
                Status::Added => c.added += 1,
                Status::Removed => c.removed += 1,
                Status::Changed => c.changed += 1,
                Status::Moved => c.moved += 1,
            }
        }
        c
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty() && self.document.is_empty()
    }
}

// ---------------------------------------------------------------- matching

fn ref_id(m: &Model, r: Ref) -> &str {
    match r {
        Ref::Element(i) => &m.elements[i].id,
        Ref::Flow(i) => &m.flows[i].id,
    }
}

/// Elements: by id, then by (kind, name) when unique among the leftovers on both sides.
/// Flows: by id, then by (kind, matched source, matched target) when unique.
fn match_items(old: &Model, new: &Model) -> HashMap<Ref, Ref> {
    let mut map: HashMap<Ref, Ref> = HashMap::new();
    let mut used_new: HashSet<Ref> = HashSet::new();
    for (i, e) in old.elements.iter().enumerate() {
        if let Some(j) = new.element_idx(&e.id) {
            map.insert(Ref::Element(i), Ref::Element(j));
            used_new.insert(Ref::Element(j));
        }
    }
    for (i, f) in old.flows.iter().enumerate() {
        if let Some(Ref::Flow(j)) = new.lookup(&f.id) {
            map.insert(Ref::Flow(i), Ref::Flow(j));
            used_new.insert(Ref::Flow(j));
        }
    }
    // Elements by kind + name.
    let name_key = |e: &Element| e.name.as_deref().map(str::trim).filter(|n| !n.is_empty()).map(|n| (e.kind.clone(), n.to_string()));
    let mut old_by: HashMap<(ElementKind, String), Vec<usize>> = HashMap::new();
    for (i, e) in old.elements.iter().enumerate() {
        if !map.contains_key(&Ref::Element(i)) {
            if let Some(k) = name_key(e) {
                old_by.entry(k).or_default().push(i);
            }
        }
    }
    let mut new_by: HashMap<(ElementKind, String), Vec<usize>> = HashMap::new();
    for (j, e) in new.elements.iter().enumerate() {
        if !used_new.contains(&Ref::Element(j)) {
            if let Some(k) = name_key(e) {
                new_by.entry(k).or_default().push(j);
            }
        }
    }
    for (k, olds) in &old_by {
        if let (1, Some(news)) = (olds.len(), new_by.get(k)) {
            if news.len() == 1 {
                map.insert(Ref::Element(olds[0]), Ref::Element(news[0]));
                used_new.insert(Ref::Element(news[0]));
            }
        }
    }
    // Flows by kind + endpoints (in new indices).
    let ep = |m: &HashMap<Ref, Ref>, x: Option<usize>| x.and_then(|x| m.get(&Ref::Element(x)).copied());
    let mut old_fk: HashMap<(FlowKind, Ref, Ref), Vec<usize>> = HashMap::new();
    for (i, f) in old.flows.iter().enumerate() {
        if !map.contains_key(&Ref::Flow(i)) {
            if let (Some(s), Some(t)) = (ep(&map, f.source), ep(&map, f.target)) {
                old_fk.entry((f.kind, s, t)).or_default().push(i);
            }
        }
    }
    let mut new_fk: HashMap<(FlowKind, Ref, Ref), Vec<usize>> = HashMap::new();
    for (j, f) in new.flows.iter().enumerate() {
        if !used_new.contains(&Ref::Flow(j)) {
            if let (Some(s), Some(t)) = (f.source, f.target) {
                new_fk.entry((f.kind, Ref::Element(s), Ref::Element(t))).or_default().push(j);
            }
        }
    }
    for (k, olds) in &old_fk {
        if let (1, Some(news)) = (olds.len(), new_fk.get(k)) {
            if news.len() == 1 {
                map.insert(Ref::Flow(olds[0]), Ref::Flow(news[0]));
            }
        }
    }
    map
}

// ---------------------------------------------------------------- property flattening

/// Ordered (key, value) list with unique keys: a repeated key gets a " #2", " #3"… suffix.
#[derive(Default)]
struct Props(Vec<(String, String)>);

impl Props {
    fn push(&mut self, key: impl Into<String>, value: impl AsRef<str>) {
        let key = key.into();
        let value = value.as_ref().trim().to_string();
        let mut k = key.clone();
        let mut n = 1;
        while self.0.iter().any(|(x, _)| *x == k) {
            n += 1;
            k = format!("{key} #{n}");
        }
        self.0.push((k, value));
    }
    fn opt(&mut self, key: &str, value: Option<&str>) {
        if let Some(v) = value {
            self.push(key, v);
        }
    }
}

/// Lines "label: value" for the present values.
fn lines(parts: &[(&str, Option<&str>)]) -> String {
    parts.iter().filter_map(|(k, v)| v.map(|v| format!("{k}: {}", v.trim()))).collect::<Vec<_>>().join("\n")
}

/// A parameter value as text. Children of lists and maps start on new lines indented by
/// `depth` levels; multi-line scripts continue at the indentation of the line they start on.
fn param_text(v: &ParamValue, depth: usize) -> String {
    let pad = "  ".repeat(depth);
    match v {
        ParamValue::Text(t) => t.trim().to_string(),
        ParamValue::Script { format, resource, source } => {
            let mut s = format!("script{}", format.as_ref().map(|f| format!(" ({f})")).unwrap_or_default());
            if let Some(r) = resource {
                s.push_str(&format!(" resource={r}"));
            }
            let cont = "  ".repeat(depth.saturating_sub(1));
            for line in source.trim().lines() {
                s.push_str(&format!("\n{cont}{line}"));
            }
            s
        }
        ParamValue::List(items) => {
            let mut s = format!("list [{}]", items.len());
            for it in items {
                s.push_str(&format!("\n{pad}- {}", param_text(it, depth + 1)));
            }
            s
        }
        ParamValue::Map(entries) => {
            let mut s = format!("map {{{}}}", entries.len());
            for (k, it) in entries {
                s.push_str(&format!("\n{pad}{k}: {}", param_text(it, depth + 1)));
            }
            s
        }
    }
}

fn field_text(f: &FieldInjection) -> String {
    match (&f.string_value, &f.expression) {
        (Some(s), _) => format!("string: {}", s.trim()),
        (None, Some(e)) => format!("expression: {}", e.trim()),
        (None, None) => String::new(),
    }
}

fn ext_props(p: &mut Props, x: &Extensions, prefix: &str) {
    for i in &x.inputs {
        p.push(format!("{prefix}input {}", i.name), param_text(&i.value, 1));
    }
    for o in &x.outputs {
        p.push(format!("{prefix}output {}", o.name), param_text(&o.value, 1));
    }
    for l in &x.listeners {
        let mut v = vec![];
        if let Some(c) = &l.class {
            v.push(format!("class: {c}"));
        }
        if let Some(c) = &l.delegate_expression {
            v.push(format!("delegateExpression: {c}"));
        }
        if let Some(c) = &l.expression {
            v.push(format!("expression: {c}"));
        }
        for f in &l.fields {
            v.push(format!("field {}: {}", f.name, field_text(f)));
        }
        if let Some((fmt, src)) = &l.script {
            v.push(format!("script{}", fmt.as_ref().map(|f| format!(" ({f})")).unwrap_or_default()));
            v.push(src.trim().to_string());
        }
        p.push(format!("{prefix}listener {}:{}", l.kind, l.event.as_deref().unwrap_or("")), v.join("\n"));
    }
    for (k, v) in &x.properties {
        p.push(format!("{prefix}property {k}"), v);
    }
    for f in &x.fields {
        p.push(format!("{prefix}field {}", f.name), field_text(f));
    }
    for f in &x.form_fields {
        let mut v = vec![lines(&[("label", f.label.as_deref()), ("type", f.field_type.as_deref()), ("default", f.default_value.as_deref())])];
        for (n, c) in &f.validation {
            v.push(format!("validation {n}{}", c.as_ref().map(|c| format!(": {c}")).unwrap_or_default()));
        }
        for (k, val) in &f.properties {
            v.push(format!("property {k}: {val}"));
        }
        for (id, name) in &f.values {
            v.push(format!("value {id}: {name}"));
        }
        v.retain(|s| !s.is_empty());
        p.push(format!("{prefix}form field {}", f.id), v.join("\n"));
    }
    p.opt(&format!("{prefix}retry time cycle"), x.failed_job_retry_time_cycle.as_deref());
    for vm in &x.variable_mappings {
        let what = vm.target.as_deref().or(vm.source.as_deref()).or(vm.source_expression.as_deref()).or(vm.variables.as_deref()).or(vm.business_key.as_deref()).unwrap_or("");
        let local = vm.local.then_some("true");
        let v = lines(&[("source", vm.source.as_deref()), ("sourceExpression", vm.source_expression.as_deref()), ("target", vm.target.as_deref()), ("variables", vm.variables.as_deref()), ("businessKey", vm.business_key.as_deref()), ("local", local)]);
        p.push(format!("{prefix}{} mapping {what}", vm.direction), v);
    }
    for raw in &x.other {
        p.push(format!("{prefix}extension"), crate::text::pretty_xml(raw));
    }
}

fn event_text(d: &EventDef) -> String {
    match d {
        EventDef::Timer { time_date, time_duration, time_cycle } => lines(&[("timeDate", time_date.as_deref()), ("timeDuration", time_duration.as_deref()), ("timeCycle", time_cycle.as_deref())]),
        EventDef::Message { message_ref, name } => lines(&[("messageRef", message_ref.as_deref()), ("name", name.as_deref())]),
        EventDef::Signal { signal_ref, name } => lines(&[("signalRef", signal_ref.as_deref()), ("name", name.as_deref())]),
        EventDef::Error { error_ref, name, code, code_variable, message_variable } => lines(&[("errorRef", error_ref.as_deref()), ("name", name.as_deref()), ("code", code.as_deref()), ("codeVariable", code_variable.as_deref()), ("messageVariable", message_variable.as_deref())]),
        EventDef::Escalation { escalation_ref, name, code, code_variable } => lines(&[("escalationRef", escalation_ref.as_deref()), ("name", name.as_deref()), ("code", code.as_deref()), ("codeVariable", code_variable.as_deref())]),
        EventDef::Conditional { condition, variable_name, variable_events } => lines(&[("condition", condition.as_ref().map(|c| c.text.as_str())), ("language", condition.as_ref().and_then(|c| c.language.as_deref())), ("variableName", variable_name.as_deref()), ("variableEvents", variable_events.as_deref())]),
        EventDef::Link { name } => lines(&[("name", name.as_deref())]),
        EventDef::Compensate { activity_ref, wait_for_completion } => {
            let w = wait_for_completion.map(|w| w.to_string());
            lines(&[("activityRef", activity_ref.as_deref()), ("waitForCompletion", w.as_deref())])
        }
        EventDef::Terminate | EventDef::Cancel => String::new(),
    }
}

/// Attributes holding ids of other items; their old values are translated through the matching
/// so a regenerated id does not report every reference to it as changed.
const REF_ATTRS: &[&str] = &["sourceRef", "targetRef", "default", "attachedToRef"];

/// Translates old ids to the ids of their matched new items (identity for new-side props).
type Rename<'a> = &'a dyn Fn(&str) -> String;

fn attr_props(p: &mut Props, attrs: &[(String, String)], rename: Rename) {
    for (k, v) in attrs {
        if k == "id" || k.starts_with("xmlns") {
            continue;
        }
        if REF_ATTRS.contains(&k.as_str()) {
            p.push(k.clone(), rename(v));
        } else {
            p.push(k.clone(), v);
        }
    }
}

fn element_props(m: &Model, e: &Element, rename: Rename) -> Props {
    let mut p = Props::default();
    p.push("type", e.kind.label());
    attr_props(&mut p, &e.attrs, rename);
    if e.attr("name").is_none() {
        // Text annotations keep their text in a child element.
        p.opt("text", e.name.as_deref());
    }
    p.opt("documentation", e.documentation.as_deref());
    if let Some(l) = e.lane {
        p.push("lane", rename(&m.elements[l].id));
    }
    if let Some(pi) = e.parent.filter(|&pi| m.elements[pi].kind.is_container()) {
        p.push("parent", rename(&m.elements[pi].id));
    }
    for d in &e.event_defs {
        p.push(format!("event {}", d.label()), event_text(d));
    }
    if let Some(mi) = &e.multi_instance {
        let mut v = vec![format!("sequential: {}", mi.sequential)];
        for (k, x) in [("collection", &mi.collection), ("elementVariable", &mi.element_variable), ("cardinality", &mi.cardinality), ("completionCondition", &mi.completion_condition)] {
            if let Some(x) = x {
                v.push(format!("{k}: {}", x.trim()));
            }
        }
        for (k, x) in &mi.attrs {
            if !matches!(k.as_str(), "camunda:collection" | "camunda:elementVariable" | "isSequential" | "id") {
                v.push(format!("{k}: {x}"));
            }
        }
        p.push("multi-instance", v.join("\n"));
    }
    if e.standard_loop {
        p.push("loop", "standard");
    }
    p.opt("script", e.script.as_deref());
    ext_props(&mut p, &e.ext, "");
    p
}

fn flow_props(f: &Flow, rename: Rename) -> Props {
    let mut p = Props::default();
    p.push("type", f.kind.label());
    attr_props(&mut p, &f.attrs, rename);
    p.opt("documentation", f.documentation.as_deref());
    if let Some(c) = &f.condition {
        p.push("condition", &c.text);
        p.opt("condition language", c.language.as_deref());
    }
    ext_props(&mut p, &f.ext, "");
    p
}

fn document_props(m: &Model) -> Props {
    let mut p = Props::default();
    for (k, v) in &m.definitions_attrs {
        if k != "id" && !k.starts_with("xmlns") {
            p.push(format!("definitions {k}"), v);
        }
    }
    for pr in &m.processes {
        let pre = format!("process {}: ", pr.id);
        for (k, v) in &pr.attrs {
            if k != "id" && !k.starts_with("xmlns") {
                p.push(format!("{pre}{k}"), v);
            }
        }
        p.opt(&format!("{pre}documentation"), pr.documentation.as_deref());
        ext_props(&mut p, &pr.ext, &pre);
    }
    for (what, map) in [("message", &m.messages), ("error", &m.errors), ("signal", &m.signals), ("escalation", &m.escalations)] {
        let mut defs: Vec<&NamedDef> = map.values().collect();
        defs.sort_by(|a, b| a.id.cmp(&b.id));
        for d in defs {
            p.push(format!("{what} {}", d.id), lines(&[("name", d.name.as_deref()), ("code", d.code.as_deref())]));
        }
    }
    p
}

/// Property changes between two flattened lists: new order, then keys only the old side has.
fn compare(old: &Props, new: &Props) -> Vec<PropChange> {
    let o: HashMap<&str, &str> = old.0.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let n: HashMap<&str, &str> = new.0.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let mut out = vec![];
    for (k, v) in &new.0 {
        match o.get(k.as_str()) {
            Some(ov) if *ov == v => {}
            ov => out.push(PropChange { key: k.clone(), old: ov.map(|s| s.to_string()), new: Some(v.clone()) }),
        }
    }
    for (k, v) in &old.0 {
        if !n.contains_key(k.as_str()) {
            out.push(PropChange { key: k.clone(), old: Some(v.clone()), new: None });
        }
    }
    out
}

// ---------------------------------------------------------------- layout

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() <= 0.5
}
fn rect_eq(a: &Rect, b: &Rect) -> bool {
    (!a.is_valid() && !b.is_valid()) || (near(a.x, b.x) && near(a.y, b.y) && near(a.w, b.w) && near(a.h, b.h))
}
fn opt_rect_eq(a: &Option<Rect>, b: &Option<Rect>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => rect_eq(a, b),
        _ => false,
    }
}
fn layout_changed(old: &Model, new: &Model, o: Ref, n: Ref) -> bool {
    match (o, n) {
        (Ref::Element(i), Ref::Element(j)) => {
            let (a, b) = (&old.elements[i], &new.elements[j]);
            !rect_eq(&a.bounds, &b.bounds) || !opt_rect_eq(&a.label_bounds, &b.label_bounds)
        }
        (Ref::Flow(i), Ref::Flow(j)) => {
            let (a, b) = (&old.flows[i], &new.flows[j]);
            a.waypoints.len() != b.waypoints.len() || a.waypoints.iter().zip(&b.waypoints).any(|(p, q)| !near(p.x, q.x) || !near(p.y, q.y)) || !opt_rect_eq(&a.label_bounds, &b.label_bounds)
        }
        _ => false,
    }
}

// ---------------------------------------------------------------- the diff

fn label_of(m: &Model, r: Ref) -> String {
    match r {
        Ref::Element(i) => {
            let e = &m.elements[i];
            format!("{} ({})", e.display_name(), e.kind.label())
        }
        Ref::Flow(i) => {
            let f = &m.flows[i];
            let name = match f.name.as_deref().map(str::trim).filter(|n| !n.is_empty()) {
                Some(n) => n.to_string(),
                None => {
                    let end = |x: Option<usize>| x.map(|x| m.elements[x].display_name().to_string()).unwrap_or_else(|| "?".into());
                    format!("{} → {}", end(f.source), end(f.target))
                }
            };
            format!("{name} ({})", f.kind.label())
        }
    }
}

/// Every element and flow once: reading order, then the remaining drawn elements, then the rest;
/// each element followed by its outgoing flows; unattached flows last.
fn ordered_refs(m: &Model) -> Vec<Ref> {
    let mut seen_e = vec![false; m.elements.len()];
    let mut seen_f = vec![false; m.flows.len()];
    let mut out = Vec::with_capacity(m.elements.len() + m.flows.len());
    let all = m.reading_order.iter().chain(m.draw_order.iter()).copied().chain(0..m.elements.len());
    for i in all {
        if std::mem::replace(&mut seen_e[i], true) {
            continue;
        }
        out.push(Ref::Element(i));
        for &f in &m.elements[i].outgoing {
            if !std::mem::replace(&mut seen_f[f], true) {
                out.push(Ref::Flow(f));
            }
        }
    }
    out.extend((0..m.flows.len()).filter(|&f| !seen_f[f]).map(Ref::Flow));
    out
}

pub fn diff(old: &Model, new: &Model) -> ModelDiff {
    let old_to_new = match_items(old, new);
    let new_to_old: HashMap<Ref, Ref> = old_to_new.iter().map(|(&o, &n)| (n, o)).collect();
    // Old element/flow ids renamed by matching → their new ids.
    let renamed: HashMap<&str, &str> = old_to_new.iter().map(|(&o, &n)| (ref_id(old, o), ref_id(new, n))).filter(|(a, b)| a != b).collect();
    let rename_old = |id: &str| renamed.get(id).copied().unwrap_or(id).to_string();
    let same = |id: &str| id.to_string();

    let mut d = ModelDiff { old_to_new: old_to_new.clone(), new_to_old: new_to_old.clone(), ..Default::default() };
    for n in ordered_refs(new) {
        let id = ref_id(new, n).to_string();
        let is_flow = matches!(n, Ref::Flow(_));
        let item = match new_to_old.get(&n) {
            None => ItemDiff { id, old_id: None, is_flow, old: None, new: Some(n), status: Status::Added, props: vec![], layout_changed: false, label: label_of(new, n) },
            Some(&o) => {
                let (op, np) = match (o, n) {
                    (Ref::Element(i), Ref::Element(j)) => (element_props(old, &old.elements[i], &rename_old), element_props(new, &new.elements[j], &same)),
                    (Ref::Flow(i), Ref::Flow(j)) => (flow_props(&old.flows[i], &rename_old), flow_props(&new.flows[j], &same)),
                    _ => unreachable!("matching pairs elements with elements and flows with flows"),
                };
                let mut props = compare(&op, &np);
                let old_id = ref_id(old, o);
                let old_id = (old_id != id).then(|| old_id.to_string());
                if let Some(oid) = &old_id {
                    props.insert(0, PropChange { key: "id".into(), old: Some(oid.clone()), new: Some(id.clone()) });
                }
                let layout = layout_changed(old, new, o, n);
                let status = if !props.is_empty() {
                    Status::Changed
                } else if layout {
                    Status::Moved
                } else {
                    continue;
                };
                ItemDiff { id, old_id, is_flow, old: Some(o), new: Some(n), status, props, layout_changed: layout, label: label_of(new, n) }
            }
        };
        d.by_new.insert(n, d.items.len());
        if let Some(o) = item.old {
            d.by_old.insert(o, d.items.len());
        }
        d.items.push(item);
    }
    for o in ordered_refs(old) {
        if old_to_new.contains_key(&o) {
            continue;
        }
        d.by_old.insert(o, d.items.len());
        d.items.push(ItemDiff { id: ref_id(old, o).to_string(), old_id: None, is_flow: matches!(o, Ref::Flow(_)), old: Some(o), new: None, status: Status::Removed, props: vec![], layout_changed: false, label: label_of(old, o) });
    }
    d.document = compare(&document_props(old), &document_props(new));
    d
}

// ---------------------------------------------------------------- merged model

/// The new model plus every removed element and flow of the old one, for drawing removals as
/// ghosts in place. Items of the new model keep their indices, so a new `Ref` is a merged `Ref`.
///
/// `model.source` is the new source followed by a newline and the old source; ghosts'
/// `xml_range`s point into that old part, so `xml_of_element`/`xml_of_flow` work for every item.
/// Use the original models (not this one) when the whole document text is needed.
pub struct Merged {
    pub model: Model,
    /// Old ref of a removed item → its ghost in `model`.
    pub removed: HashMap<Ref, Ref>,
    /// Ghost in `model` → old ref.
    pub ghosts: HashMap<Ref, Ref>,
    /// Length of the new source at the start of `model.source`.
    pub new_source_len: usize,
}

impl Merged {
    pub fn is_ghost(&self, r: Ref) -> bool {
        self.ghosts.contains_key(&r)
    }
}

pub fn merge(old: &Model, new: &Model, d: &ModelDiff) -> Merged {
    let mut m = new.clone();
    let shift = new.source.len() + 1;
    m.source = Arc::from(format!("{}\n{}", new.source, old.source));
    let mut removed: HashMap<Ref, Ref> = HashMap::new();
    let removed_items: Vec<Ref> = d.items.iter().filter(|it| it.status == Status::Removed).filter_map(|it| it.old).collect();
    // Assign ghost indices first so parents, lanes and flow endpoints can be remapped.
    let (mut ne, mut nf) = (m.elements.len(), m.flows.len());
    for &o in &removed_items {
        let g = match o {
            Ref::Element(_) => {
                ne += 1;
                Ref::Element(ne - 1)
            }
            Ref::Flow(_) => {
                nf += 1;
                Ref::Flow(nf - 1)
            }
        };
        removed.insert(o, g);
    }
    let map_el = |i: ElementIdx| -> Option<ElementIdx> {
        match d.new_of_old(Ref::Element(i)).or_else(|| removed.get(&Ref::Element(i)).copied()) {
            Some(Ref::Element(x)) => Some(x),
            _ => None,
        }
    };
    let shift_range = |r: &Range<usize>| r.start + shift..r.end + shift;
    for &o in &removed_items {
        if let Ref::Element(i) = o {
            let mut e = old.elements[i].clone();
            e.parent = e.parent.and_then(map_el);
            e.lane = e.lane.and_then(map_el);
            e.incoming.clear();
            e.outgoing.clear();
            e.boundary_events.clear();
            e.xml_range = shift_range(&e.xml_range);
            m.elements.push(e);
        }
    }
    for &o in &removed_items {
        if let Ref::Flow(i) = o {
            let mut f = old.flows[i].clone();
            f.source = f.source.and_then(map_el);
            f.target = f.target.and_then(map_el);
            f.source_id = f.source.map(|s| m.elements[s].id.clone()).or(f.source_id);
            f.target_id = f.target.map(|t| m.elements[t].id.clone()).or(f.target_id);
            f.xml_range = shift_range(&f.xml_range);
            let fi = m.flows.len();
            if let Some(s) = f.source {
                m.elements[s].outgoing.push(fi);
            }
            if let Some(t) = f.target {
                m.elements[t].incoming.push(fi);
            }
            m.flows.push(f);
        }
    }
    // Ghost boundary events hang off their (surviving or ghost) host.
    for &o in &removed_items {
        if let (Ref::Element(i), Some(&Ref::Element(g))) = (o, removed.get(&o)) {
            if let Some(host) = old.elements[i].attached_to.as_deref().and_then(|a| old.element_idx(a)).and_then(map_el) {
                m.elements[host].boundary_events.push(g);
            }
        }
    }
    // Ids: new items win over ghosts with the same id.
    for (&o, &g) in &removed {
        let id = ref_id(old, o).to_string();
        m.by_id.entry(id).or_insert(g);
    }
    for g in removed.values() {
        let b = match *g {
            Ref::Element(i) => {
                let e = &m.elements[i];
                e.label_bounds.map(|l| e.bounds.union(&l)).unwrap_or(e.bounds)
            }
            Ref::Flow(i) => m.flows[i].bounds,
        };
        if b.is_valid() {
            m.bounds = m.bounds.union(&b);
        }
    }
    m.draw_order = crate::parse::draw_order(&m);
    m.reading_order = crate::parse::reading_order(&m);
    let ghosts = removed.iter().map(|(&o, &g)| (g, o)).collect();
    Merged { model: m, removed, ghosts, new_source_len: new.source.len() }
}

// ---------------------------------------------------------------- text diffs

/// How long a text diff may search before settling for a coarser (still correct) answer.
const TEXT_DEADLINE: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum LineTag {
    Equal,
    Delete,
    Insert,
}

/// One row of a line diff; line numbers are 0-based indices into `split_lines` of each side.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct TextRow {
    pub tag: LineTag,
    pub old_line: Option<usize>,
    pub new_line: Option<usize>,
}

/// Lines as `text_rows` numbers them: split on '\n', a trailing '\r' stripped, no empty last
/// line for a trailing newline (like `str::lines`).
pub fn split_lines(s: &str) -> Vec<&str> {
    s.lines().collect()
}

/// Line-level diff of two whole texts (deletions before insertions within a replaced block).
pub fn text_rows(old: &str, new: &str) -> Vec<TextRow> {
    let (a, b) = (split_lines(old), split_lines(new));
    let ops = similar::capture_diff_slices_deadline(similar::Algorithm::Patience, &a, &b, Some(Instant::now() + TEXT_DEADLINE));
    let mut rows = Vec::with_capacity(a.len().max(b.len()));
    let del = |rows: &mut Vec<TextRow>, r: Range<usize>| rows.extend(r.map(|i| TextRow { tag: LineTag::Delete, old_line: Some(i), new_line: None }));
    let ins = |rows: &mut Vec<TextRow>, r: Range<usize>| rows.extend(r.map(|j| TextRow { tag: LineTag::Insert, old_line: None, new_line: Some(j) }));
    for op in ops {
        match op {
            similar::DiffOp::Equal { old_index, new_index, len } => rows.extend((0..len).map(|k| TextRow { tag: LineTag::Equal, old_line: Some(old_index + k), new_line: Some(new_index + k) })),
            similar::DiffOp::Delete { old_index, old_len, .. } => del(&mut rows, old_index..old_index + old_len),
            similar::DiffOp::Insert { new_index, new_len, .. } => ins(&mut rows, new_index..new_index + new_len),
            similar::DiffOp::Replace { old_index, old_len, new_index, new_len } => {
                del(&mut rows, old_index..old_index + old_len);
                ins(&mut rows, new_index..new_index + new_len);
            }
        }
    }
    rows
}

/// Tokens for inline diffs: runs of word characters, runs of whitespace, single other chars.
fn tokens(s: &str) -> Vec<&str> {
    #[derive(PartialEq)]
    enum C {
        Word,
        Space,
        Other,
    }
    let class = |c: char| if c.is_alphanumeric() || c == '_' { C::Word } else if c.is_whitespace() { C::Space } else { C::Other };
    let mut out = vec![];
    let mut start = 0;
    let mut prev: Option<C> = None;
    for (i, c) in s.char_indices() {
        let k = class(c);
        if let Some(p) = &prev {
            if *p != k || k == C::Other {
                out.push(&s[start..i]);
                start = i;
            }
        }
        prev = Some(k);
    }
    if start < s.len() {
        out.push(&s[start..]);
    }
    out
}

fn push_range(v: &mut Vec<Range<usize>>, r: Range<usize>) {
    if r.is_empty() {
        return;
    }
    match v.last_mut() {
        Some(last) if last.end == r.start => last.end = r.end,
        _ => v.push(r),
    }
}

/// Byte ranges that differ within a changed pair of lines (word granularity), for emphasis.
pub fn inline_ranges(old_line: &str, new_line: &str) -> (Vec<Range<usize>>, Vec<Range<usize>>) {
    let (a, b) = (tokens(old_line), tokens(new_line));
    let offsets = |t: &[&str]| {
        let mut o = Vec::with_capacity(t.len() + 1);
        let mut acc = 0;
        o.push(0);
        for x in t {
            acc += x.len();
            o.push(acc);
        }
        o
    };
    let (oa, ob) = (offsets(&a), offsets(&b));
    let ops = similar::capture_diff_slices_deadline(similar::Algorithm::Myers, &a, &b, Some(Instant::now() + TEXT_DEADLINE));
    let (mut ra, mut rb) = (vec![], vec![]);
    for op in ops {
        match op {
            similar::DiffOp::Equal { .. } => {}
            similar::DiffOp::Delete { old_index, old_len, .. } => push_range(&mut ra, oa[old_index]..oa[old_index + old_len]),
            similar::DiffOp::Insert { new_index, new_len, .. } => push_range(&mut rb, ob[new_index]..ob[new_index + new_len]),
            similar::DiffOp::Replace { old_index, old_len, new_index, new_len } => {
                push_range(&mut ra, oa[old_index]..oa[old_index + old_len]);
                push_range(&mut rb, ob[new_index]..ob[new_index + new_len]);
            }
        }
    }
    (ra, rb)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn props_get_unique_keys_in_order() {
        let mut p = Props::default();
        p.push("listener execution:start", " a ");
        p.push("listener execution:start", "b");
        p.push("listener execution:start", "c");
        assert_eq!(p.0.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>(), ["listener execution:start=a", "listener execution:start #2=b", "listener execution:start #3=c"]);
    }

    #[test]
    fn compare_reports_changed_added_and_dropped_keys() {
        let mut a = Props::default();
        a.push("name", "x");
        a.push("gone", "1");
        a.push("same", "s");
        let mut b = Props::default();
        b.push("same", "s");
        b.push("name", "y");
        b.push("fresh", "2");
        let c = compare(&a, &b);
        assert_eq!(
            c,
            vec![
                PropChange { key: "name".into(), old: Some("x".into()), new: Some("y".into()) },
                PropChange { key: "fresh".into(), old: None, new: Some("2".into()) },
                PropChange { key: "gone".into(), old: Some("1".into()), new: None },
            ]
        );
    }

    #[test]
    fn nested_params_render_as_indented_lines() {
        let v = ParamValue::Map(vec![("a".into(), ParamValue::Text("1".into())), ("l".into(), ParamValue::List(vec![ParamValue::Text("x".into()), ParamValue::Text("y".into())]))]);
        assert_eq!(param_text(&v, 1), "map {2}\n  a: 1\n  l: list [2]\n    - x\n    - y");
    }

    #[test]
    fn tokens_cover_the_input_exactly() {
        let s = r#"<task name="Ship order" camunda:a="${x.y}">  é"#;
        let t = tokens(s);
        assert_eq!(t.concat(), s);
        assert!(t.contains(&"Ship") && t.contains(&"camunda") && t.contains(&"  "));
    }
}
