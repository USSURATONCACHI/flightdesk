//! The element inspector: every parameter of a BPMN element, in a fixed section order.

use crate::widgets::{kv, kv_link, kv_mono, section};
use bpmn::{Element, ElementKind, EventDef, Flow, Model, ParamValue, Ref};
use egui::{RichText, Ui};

#[derive(Clone, Debug, PartialEq)]
pub enum InspectorAction {
    Select(Ref),
    OpenDefinitionKey(String),
    /// Show running instances currently at this activity.
    InstancesAt(String),
    /// Find (historic) instances that executed this activity.
    ExecutedAt(String),
    IncidentsAt(String),
    /// Fit the whole diagram into view.
    FitDiagram,
    /// Open the document's XML view at this element (or the start).
    ViewXml(Option<String>),
    /// Open the document in an external program or copy `path:line:col` of the element.
    XmlExternal(Option<String>, crate::places::xml::External),
}

/// Runtime facts the owning place knows about an element.
#[derive(Default)]
pub struct RuntimeFacts {
    pub rows: Vec<(String, String)>,
    pub running: Option<u32>,
    pub incidents: Option<u32>,
    pub executed: Option<u32>,
}

pub fn show(ui: &mut Ui, model: &Model, target: Ref, facts: &RuntimeFacts, in_instance: bool) -> Vec<InspectorAction> {
    let mut actions = vec![];
    match target {
        Ref::Element(i) => element(ui, model, &model.elements[i], facts, in_instance, &mut actions),
        Ref::Flow(i) => flow(ui, model, &model.flows[i], &mut actions),
    }
    actions
}

fn element(ui: &mut Ui, m: &Model, e: &Element, facts: &RuntimeFacts, in_instance: bool, actions: &mut Vec<InspectorAction>) {
    // Header
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(e.display_name()).size(15.0).strong());
        ui.label(RichText::new(crate::i18n::bpmn_kind(&e.kind)).weak());
        for d in &e.event_defs {
            ui.label(RichText::new(crate::i18n::bpmn_event(d)).weak());
        }
    });
    ui.add_space(4.0);

    section(ui, ("identity", &e.id), "Identity", true, |ui| {
        kv_mono(ui, "id", &e.id);
        if let Some(n) = &e.name {
            kv(ui, "name", n);
        }
        kv(ui, "type", &crate::i18n::bpmn_kind(&e.kind));
        if let Some(l) = e.lane {
            if kv_link(ui, "lane", m.elements[l].display_name()) {
                actions.push(InspectorAction::Select(Ref::Element(l)));
            }
        }
        if let Some(p) = e.parent {
            let pe = &m.elements[p];
            let key = if pe.kind == ElementKind::Participant { "pool" } else { "parent" };
            if kv_link(ui, key, pe.display_name()) {
                actions.push(InspectorAction::Select(Ref::Element(p)));
            }
        }
        if let Some(a) = &e.attached_to {
            if let Some(ai) = m.element_idx(a) {
                if kv_link(ui, "attached to", m.elements[ai].display_name()) {
                    actions.push(InspectorAction::Select(Ref::Element(ai)));
                }
            }
            kv(ui, "interrupting", &if e.interrupting { crate::t!("common-yes") } else { crate::t!("common-no") });
        }
        if !e.boundary_events.is_empty() {
            for &b in &e.boundary_events {
                let be = &m.elements[b];
                let label = format!("{} ({})", be.display_name(), be.event_defs.first().map(crate::i18n::bpmn_event).unwrap_or_else(|| crate::t!("bpmn-event-generic")));
                if kv_link(ui, "boundary event", &label) {
                    actions.push(InspectorAction::Select(Ref::Element(b)));
                }
            }
        }
    });

    if e.kind.is_flow_node() {
        section(ui, ("runtime", &e.id), if in_instance { "In this instance" } else { "Runtime" }, true, |ui| {
            for (k, v) in &facts.rows {
                kv(ui, k, v);
            }
            ui.horizontal_wrapped(|ui| {
                let running = facts.running.unwrap_or(0);
                if ui.add_enabled(running > 0 || !in_instance, egui::Button::new(crate::t!("inspector-running-here", n = running)).small()).on_hover_text(crate::t!("inspector-running-here-tip")).clicked() {
                    actions.push(InspectorAction::InstancesAt(e.id.clone()));
                }
                let inc = facts.incidents.unwrap_or(0);
                if inc > 0 && ui.add(egui::Button::new(RichText::new(crate::t!("n-incidents", n = inc)).color(crate::theme::INCIDENT)).small()).clicked() {
                    actions.push(InspectorAction::IncidentsAt(e.id.clone()));
                }
                if !in_instance && ui.add(egui::Button::new(crate::t!("ctx-instances-executed")).small()).on_hover_text(crate::t!("inspector-executed-tip")).clicked() {
                    actions.push(InspectorAction::ExecutedAt(e.id.clone()));
                }
            });
            if let Some(x) = facts.executed {
                kv(ui, "executed (history)", &format!("×{x}"));
            }
        });
    }

    // Execution
    let exec_keys = ["asyncBefore", "asyncAfter", "exclusive", "jobPriority", "historyTimeToLive"];
    let exec_rows: Vec<(&str, &str)> = exec_keys.iter().filter_map(|k| e.camunda(k).map(|v| (*k, v))).collect();
    if !exec_rows.is_empty() || e.ext.failed_job_retry_time_cycle.is_some() {
        section(ui, ("exec", &e.id), "Execution", true, |ui| {
            for (k, v) in exec_rows {
                kv(ui, k, v);
            }
            if let Some(r) = &e.ext.failed_job_retry_time_cycle {
                let h = bpmn::text::humanize_timer(r).map(|h| format!("{r}  ({h})")).unwrap_or(r.clone());
                kv(ui, "retry time cycle", &h);
            }
        });
    }

    // Implementation
    let impl_keys = [
        ("class", "class"),
        ("delegateExpression", "delegate expression"),
        ("expression", "expression"),
        ("resultVariable", "result variable"),
        ("type", "type"),
        ("topic", "topic"),
        ("taskPriority", "task priority"),
        ("decisionRef", "decision ref"),
        ("decisionRefBinding", "decision binding"),
        ("decisionRefVersion", "decision version"),
        ("decisionRefTenantId", "decision tenant"),
        ("mapDecisionResult", "map decision result"),
        ("calledElementBinding", "called binding"),
        ("calledElementVersion", "called version"),
        ("calledElementVersionTag", "called version tag"),
        ("calledElementTenantId", "called tenant"),
        ("variableMappingClass", "variable mapping class"),
        ("variableMappingDelegateExpression", "variable mapping delegate"),
        ("resource", "resource"),
    ];
    let mut impl_rows: Vec<(String, String)> = vec![];
    for (k, label) in impl_keys {
        if let Some(v) = e.camunda(k) {
            impl_rows.push((label.to_string(), v.to_string()));
        }
    }
    let called = e.attr("calledElement").map(String::from);
    let script_format = e.attr("scriptFormat").map(String::from);
    if !impl_rows.is_empty() || called.is_some() || e.script.is_some() || script_format.is_some() || !e.ext.variable_mappings.is_empty() || !e.ext.fields.is_empty() {
        section(ui, ("impl", &e.id), "Implementation", true, |ui| {
            if let Some(c) = &called {
                if kv_link(ui, "called element", c) {
                    actions.push(InspectorAction::OpenDefinitionKey(c.clone()));
                }
            }
            for (k, v) in &impl_rows {
                kv(ui, k, v);
            }
            if let Some(f) = &script_format {
                kv(ui, "script format", f);
            }
            if let Some(s) = &e.script {
                ui.label(RichText::new(crate::i18n::kv_label("script")).weak().size(12.0));
                code_block(ui, s);
            }
            for vm in &e.ext.variable_mappings {
                let mut parts = vec![];
                if let Some(v) = &vm.variables {
                    parts.push(format!("variables={v}"));
                }
                if let Some(s) = &vm.source {
                    parts.push(format!("source={s}"));
                }
                if let Some(s) = &vm.source_expression {
                    parts.push(format!("sourceExpression={s}"));
                }
                if let Some(t) = &vm.target {
                    parts.push(format!("target={t}"));
                }
                if let Some(b) = &vm.business_key {
                    parts.push(format!("businessKey={b}"));
                }
                if vm.local {
                    parts.push("local".into());
                }
                kv(ui, &format!("mapping {}", vm.direction), &parts.join("  "));
            }
            fields(ui, &e.ext.fields);
        });
    }

    // User task
    let ut_keys = [("assignee", "assignee"), ("candidateUsers", "candidate users"), ("candidateGroups", "candidate groups"), ("dueDate", "due date"), ("followUpDate", "follow-up"), ("priority", "priority"), ("formKey", "form key"), ("formRef", "form ref"), ("formRefBinding", "form binding"), ("formRefVersion", "form version")];
    let ut_rows: Vec<(&str, &str)> = ut_keys.iter().filter_map(|(k, l)| e.camunda(k).map(|v| (*l, v))).collect();
    if !ut_rows.is_empty() || !e.ext.form_fields.is_empty() {
        section(ui, ("user", &e.id), if e.kind == ElementKind::UserTask { "User task" } else { "Form" }, true, |ui| {
            for (k, v) in ut_rows {
                kv(ui, k, v);
            }
            for f in &e.ext.form_fields {
                let mut d = f.field_type.clone().unwrap_or_default();
                if let Some(l) = &f.label {
                    d.push_str(&format!("  “{l}”"));
                }
                if let Some(v) = &f.default_value {
                    d.push_str(&format!("  default={v}"));
                }
                for (n, c) in &f.validation {
                    d.push_str(&format!("  {n}{}", c.as_ref().map(|c| format!("={c}")).unwrap_or_default()));
                }
                if !f.values.is_empty() {
                    d.push_str(&format!("  values: {}", f.values.iter().map(|(i, n)| format!("{i}={n}")).collect::<Vec<_>>().join(", ")));
                }
                kv(ui, &format!("field {}", f.id), &d);
            }
        });
    }

    // Events
    if !e.event_defs.is_empty() {
        section(ui, ("events", &e.id), "Event definition", true, |ui| {
            for d in &e.event_defs {
                match d {
                    EventDef::Timer { time_date, time_duration, time_cycle } => {
                        for (k, v) in [("date", time_date), ("duration", time_duration), ("cycle", time_cycle)] {
                            if let Some(v) = v {
                                let h = bpmn::text::humanize_timer(v).map(|h| format!("{v}   ({h})")).unwrap_or(v.clone());
                                kv(ui, &format!("timer {k}"), &h);
                            }
                        }
                    }
                    EventDef::Message { message_ref, name } => {
                        kv(ui, "message", name.as_deref().unwrap_or("?"));
                        if let Some(r) = message_ref {
                            kv_mono(ui, "message ref", r);
                        }
                    }
                    EventDef::Signal { signal_ref, name } => {
                        kv(ui, "signal", name.as_deref().unwrap_or("?"));
                        if let Some(r) = signal_ref {
                            kv_mono(ui, "signal ref", r);
                        }
                    }
                    EventDef::Error { error_ref, name, code, code_variable, message_variable } => {
                        kv(ui, "error", name.as_deref().unwrap_or(error_ref.as_deref().unwrap_or("(any)")));
                        if let Some(c) = code {
                            kv_mono(ui, "error code", c);
                        }
                        if let Some(v) = code_variable {
                            kv_mono(ui, "code variable", v);
                        }
                        if let Some(v) = message_variable {
                            kv_mono(ui, "message variable", v);
                        }
                    }
                    EventDef::Escalation { escalation_ref, name, code, code_variable } => {
                        kv(ui, "escalation", name.as_deref().unwrap_or(escalation_ref.as_deref().unwrap_or("(any)")));
                        if let Some(c) = code {
                            kv_mono(ui, "escalation code", c);
                        }
                        if let Some(v) = code_variable {
                            kv_mono(ui, "code variable", v);
                        }
                    }
                    EventDef::Conditional { condition, variable_name, variable_events } => {
                        if let Some(c) = condition {
                            kv_mono(ui, "condition", &c.text);
                            if let Some(l) = &c.language {
                                kv(ui, "language", l);
                            }
                        }
                        if let Some(v) = variable_name {
                            kv_mono(ui, "variable name", v);
                        }
                        if let Some(v) = variable_events {
                            kv(ui, "variable events", v);
                        }
                    }
                    EventDef::Link { name } => {
                        kv(ui, "link", name.as_deref().unwrap_or("?"));
                    }
                    EventDef::Compensate { activity_ref, wait_for_completion } => {
                        if let Some(a) = activity_ref {
                            if let Some(ai) = m.element_idx(a) {
                                if kv_link(ui, "compensates", m.elements[ai].display_name()) {
                                    actions.push(InspectorAction::Select(Ref::Element(ai)));
                                }
                            }
                        }
                        if let Some(w) = wait_for_completion {
                            kv(ui, "wait for completion", &if *w { crate::t!("common-yes") } else { crate::t!("common-no") });
                        }
                    }
                    EventDef::Terminate => {
                        kv(ui, "terminate", "ends the whole scope");
                    }
                    EventDef::Cancel => {
                        kv(ui, "cancel", "transaction cancel");
                    }
                }
            }
        });
    }

    // Gateway / outgoing flows table
    if e.kind.is_gateway() || e.outgoing.len() > 1 {
        section(ui, ("gw", &e.id), "Outgoing flows", true, |ui| {
            if let Some(d) = &e.default_flow {
                if let Some(f) = m.flow_by_id(d) {
                    if kv_link(ui, "default flow", f.display_name()) {
                        if let Some(bpmn::Ref::Flow(fi)) = m.lookup(d) {
                            actions.push(InspectorAction::Select(Ref::Flow(fi)));
                        }
                    }
                }
            }
            egui::Grid::new(("gw-grid", &e.id)).num_columns(3).striped(true).spacing([10.0, 4.0]).show(ui, |ui| {
                ui.label(RichText::new(crate::i18n::col("flow")).weak().size(11.0));
                ui.label(RichText::new(crate::i18n::col("target")).weak().size(11.0));
                ui.label(RichText::new(crate::i18n::col("condition")).weak().size(11.0));
                ui.end_row();
                for &fi in &e.outgoing {
                    let f = &m.flows[fi];
                    if f.kind != bpmn::FlowKind::SequenceFlow {
                        continue;
                    }
                    if ui.link(RichText::new(f.display_name()).size(12.0)).clicked() {
                        actions.push(InspectorAction::Select(Ref::Flow(fi)));
                    }
                    if let Some(t) = f.target {
                        if ui.link(RichText::new(m.elements[t].display_name()).size(12.0)).clicked() {
                            actions.push(InspectorAction::Select(Ref::Element(t)));
                        }
                    } else {
                        ui.label("?");
                    }
                    let cond = if f.is_default { "(default)".to_string() } else { f.condition.as_ref().map(|c| c.text.clone()).unwrap_or_else(|| "—".into()) };
                    let r = ui.add(egui::Label::new(RichText::new(&cond).monospace().size(11.5)).wrap().sense(egui::Sense::click()));
                    if r.clicked() {
                        ui.ctx().copy_text(cond.clone());
                    }
                    ui.end_row();
                }
            });
        });
    } else if !e.outgoing.is_empty() || !e.incoming.is_empty() {
        section(ui, ("conn", &e.id), "Connections", false, |ui| {
            for &fi in &e.incoming {
                if let Some(s) = m.flows[fi].source {
                    if kv_link(ui, "from", m.elements[s].display_name()) {
                        actions.push(InspectorAction::Select(Ref::Element(s)));
                    }
                }
            }
            for &fi in &e.outgoing {
                if let Some(t) = m.flows[fi].target {
                    if kv_link(ui, "to", m.elements[t].display_name()) {
                        actions.push(InspectorAction::Select(Ref::Element(t)));
                    }
                }
            }
        });
    }

    // Multi-instance
    if let Some(mi) = &e.multi_instance {
        section(ui, ("mi", &e.id), "Multi-instance", true, |ui| {
            kv(ui, "mode", &if mi.sequential { crate::t!("mi-sequential") } else { crate::t!("mi-parallel") });
            if let Some(v) = &mi.collection {
                kv_mono(ui, "collection", v);
            }
            if let Some(v) = &mi.element_variable {
                kv_mono(ui, "element variable", v);
            }
            if let Some(v) = &mi.cardinality {
                kv_mono(ui, "cardinality", v);
            }
            if let Some(v) = &mi.completion_condition {
                kv_mono(ui, "completion condition", v);
            }
            for (k, v) in &mi.attrs {
                if k.starts_with("camunda:") && !matches!(k.as_str(), "camunda:collection" | "camunda:elementVariable") {
                    kv(ui, k.trim_start_matches("camunda:"), v);
                }
            }
        });
    }

    // I/O
    if !e.ext.inputs.is_empty() || !e.ext.outputs.is_empty() {
        section(ui, ("io", &e.id), "Input / output mapping", true, |ui| {
            if !e.ext.inputs.is_empty() {
                ui.label(RichText::new(crate::t!("io-inputs")).weak().size(12.0));
                for p in &e.ext.inputs {
                    param(ui, &p.name, &p.value, 0);
                }
            }
            if !e.ext.outputs.is_empty() {
                ui.label(RichText::new(crate::t!("io-outputs")).weak().size(12.0));
                for p in &e.ext.outputs {
                    param(ui, &p.name, &p.value, 0);
                }
            }
        });
    }

    // Listeners
    if !e.ext.listeners.is_empty() {
        section(ui, ("listeners", &e.id), "Listeners", true, |ui| {
            for l in &e.ext.listeners {
                let imp = if let Some(c) = &l.class {
                    format!("class {c}")
                } else if let Some(c) = &l.delegate_expression {
                    format!("delegate {c}")
                } else if let Some(c) = &l.expression {
                    format!("expression {c}")
                } else if let Some((f, _)) = &l.script {
                    format!("script ({})", f.clone().unwrap_or_default())
                } else {
                    "?".into()
                };
                kv(ui, &format!("{} · {}", l.kind, l.event.clone().unwrap_or_default()), &imp);
                if let Some((_, src)) = &l.script {
                    code_block(ui, src);
                }
                fields(ui, &l.fields);
            }
        });
    }

    // Properties
    if !e.ext.properties.is_empty() {
        section(ui, ("props", &e.id), "Extension properties", true, |ui| {
            for (k, v) in &e.ext.properties {
                kv(ui, k, v);
            }
        });
    }

    // Other attributes not shown above
    let shown: &[&str] = &["id", "name", "default", "attachedToRef", "cancelActivity", "isInterrupting", "calledElement", "scriptFormat", "isForCompensation", "triggeredByEvent", "camunda:asyncBefore", "camunda:asyncAfter", "camunda:exclusive", "camunda:jobPriority", "camunda:historyTimeToLive"];
    let other: Vec<&(String, String)> = e
        .attrs
        .iter()
        .filter(|(k, _)| {
            let camunda_key = k.strip_prefix("camunda:");
            let shown_impl = camunda_key.map(|c| impl_keys.iter().any(|(ik, _)| *ik == c)).unwrap_or(false);
            let shown_task = camunda_key.map(|c| ut_keys.iter().any(|(uk, _)| *uk == c)).unwrap_or(false);
            !shown.contains(&k.as_str()) && !shown_impl && !shown_task && !k.starts_with("xmlns")
        })
        .collect();
    if !other.is_empty() || !e.ext.other.is_empty() {
        section(ui, ("other", &e.id), "Other attributes", false, |ui| {
            for (k, v) in other {
                kv(ui, k, v);
            }
            for raw in &e.ext.other {
                code_block(ui, &bpmn::text::pretty_xml(raw));
            }
        });
    }

    section(ui, ("xml", &e.id), "XML", false, |ui| {
        let xml = bpmn::text::pretty_xml(m.xml_of_element(e));
        if ui.small_button(crate::t!("inspector-copy-xml")).clicked() {
            ui.ctx().copy_text(xml.clone());
        }
        code_block(ui, &xml);
    });
    // Last, so a long text never pushes the fields above out of sight.
    if let Some(d) = &e.documentation {
        crate::widgets::doc_block(ui, ("doc", &e.id), "documentation", d);
    }
}

fn flow(ui: &mut Ui, m: &Model, f: &Flow, actions: &mut Vec<InspectorAction>) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(f.display_name()).size(15.0).strong());
        ui.label(RichText::new(crate::i18n::bpmn_flow(f.kind)).weak());
        if f.is_default {
            ui.label(RichText::new(crate::t!("tip-default-flow")).weak());
        }
    });
    ui.add_space(4.0);
    section(ui, ("fid", &f.id), "Identity", true, |ui| {
        kv_mono(ui, "id", &f.id);
        if let Some(n) = &f.name {
            kv(ui, "name", n);
        }
        if let Some(s) = f.source {
            if kv_link(ui, "from", m.elements[s].display_name()) {
                actions.push(InspectorAction::Select(Ref::Element(s)));
            }
        }
        if let Some(t) = f.target {
            if kv_link(ui, "to", m.elements[t].display_name()) {
                actions.push(InspectorAction::Select(Ref::Element(t)));
            }
        }
    });
    if f.condition.is_some() || f.is_default {
        section(ui, ("fcond", &f.id), "Condition", true, |ui| {
            if f.is_default {
                kv(ui, "default", "taken when no other condition matches");
            }
            if let Some(c) = &f.condition {
                kv_mono(ui, "expression", &c.text);
                if let Some(l) = &c.language {
                    kv(ui, "language", l);
                }
            }
        });
    }
    if !f.ext.listeners.is_empty() {
        section(ui, ("flist", &f.id), "Listeners", true, |ui| {
            for l in &f.ext.listeners {
                let imp = l.class.clone().or(l.delegate_expression.clone()).or(l.expression.clone()).unwrap_or_default();
                kv(ui, &format!("{} · {}", l.kind, l.event.clone().unwrap_or_default()), &imp);
            }
        });
    }
    if !f.ext.properties.is_empty() {
        section(ui, ("fprops", &f.id), "Extension properties", true, |ui| {
            for (k, v) in &f.ext.properties {
                kv(ui, k, v);
            }
        });
    }
    let other: Vec<&(String, String)> = f.attrs.iter().filter(|(k, _)| !matches!(k.as_str(), "id" | "name" | "sourceRef" | "targetRef")).collect();
    if !other.is_empty() {
        section(ui, ("fother", &f.id), "Other attributes", false, |ui| {
            for (k, v) in other {
                kv(ui, k, v);
            }
        });
    }
    section(ui, ("fxml", &f.id), "XML", false, |ui| {
        let xml = bpmn::text::pretty_xml(m.xml_of_flow(f));
        if ui.small_button(crate::t!("inspector-copy-xml")).clicked() {
            ui.ctx().copy_text(xml.clone());
        }
        code_block(ui, &xml);
    });
    if let Some(d) = &f.documentation {
        crate::widgets::doc_block(ui, ("doc", &f.id), "documentation", d);
    }
}

fn fields(ui: &mut Ui, fields: &[bpmn::FieldInjection]) {
    for f in fields {
        let v = if let Some(s) = &f.string_value { format!("\"{s}\"") } else { f.expression.clone().unwrap_or_default() };
        kv(ui, &format!("field {}", f.name), &v);
    }
}

fn param(ui: &mut Ui, name: &str, v: &ParamValue, depth: usize) {
    match v {
        ParamValue::Text(t) => {
            kv(ui, name, t);
        }
        ParamValue::Script { format, resource, source } => {
            kv(ui, name, &format!("script{}{}", format.as_ref().map(|f| format!(" ({f})")).unwrap_or_default(), resource.as_ref().map(|r| format!(" resource={r}")).unwrap_or_default()));
            if !source.is_empty() {
                code_block(ui, source);
            }
        }
        ParamValue::List(items) => {
            kv(ui, name, &format!("list [{}]", items.len()));
            ui.indent((name, depth), |ui| {
                for (i, it) in items.iter().enumerate() {
                    param(ui, &format!("[{i}]"), it, depth + 1);
                }
            });
        }
        ParamValue::Map(entries) => {
            kv(ui, name, &format!("map {{{}}}", entries.len()));
            ui.indent((name, depth), |ui| {
                for (k, it) in entries {
                    param(ui, k, it, depth + 1);
                }
            });
        }
    }
}

pub fn code_block(ui: &mut Ui, text: &str) {
    egui::Frame::new().fill(ui.visuals().extreme_bg_color).corner_radius(4.0).inner_margin(6.0).show(ui, |ui| {
        ui.set_width(ui.available_width());
        let r = ui.add(egui::Label::new(RichText::new(text).monospace().size(11.5)).wrap().sense(egui::Sense::click()));
        if r.clicked() {
            ui.ctx().copy_text(text.to_string());
        }
        r.on_hover_text(crate::t!("common-click-to-copy"));
    });
}
