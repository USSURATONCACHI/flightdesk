//! MCP tool catalogue and dispatch. Every tool runs on the UI thread with `&mut App`;
//! engine calls are spawned on the I/O runtime and answer through the `Reply` handle.

use super::{McpCall, Reply};
use crate::app::App;
use crate::state::{Nav, NavMode, Place};
use base64::Engine as _;
use serde_json::{json, Value};

struct Spec {
    name: &'static str,
    description: &'static str,
    schema: Value,
}

fn obj(props: Value, required: &[&str]) -> Value {
    json!({"type": "object", "properties": props, "required": required, "additionalProperties": false})
}

fn specs_vec() -> Vec<Spec> {
    let s = |name: &'static str, description: &'static str, schema: Value| Spec { name, description, schema };
    let inst = json!({"type": "string", "description": "process instance id (uuid)"});
    let def = json!({"type": "string", "description": "process definition id (`key:version:uuid`) or key (latest version)"});
    vec![
        // ---------- engine: read ----------
        s("engine_status", "Connection status, engine version, URL, MCP stats and whether the UI is connected.", obj(json!({}), &[])),
        s("list_definitions", "Process definitions with running-instance and incident counts (latest version per key unless all_versions).", obj(json!({"filter": {"type": "string", "description": "substring of key or name"}, "all_versions": {"type": "boolean"}}), &[])),
        s("get_definition", "Definition metadata, versions and per-activity statistics (running tokens, incidents).", obj(json!({"definition": def}), &["definition"])),
        s("list_elements", "Elements of a definition's BPMN diagram (id, name, type, lane, implementation summary, in/out). Use to understand process structure.", obj(json!({"definition": def, "type": {"type": "string", "description": "category filter: task | gateway | event | call | subprocess | lane | pool | flow"}, "filter": {"type": "string", "description": "substring over id/name"}, "limit": {"type": "integer"}}), &["definition"])),
        s("get_element", "Everything about one BPMN element: attributes, Camunda extensions (I/O, listeners, properties, fields, forms), event definitions, multi-instance, incoming/outgoing flows with conditions, and raw XML.", obj(json!({"definition": def, "element_id": {"type": "string"}}), &["definition", "element_id"])),
        s("find_elements", "Search elements of a diagram by name, id, type, expression, implementation or documentation (same engine as Ctrl+F).", obj(json!({"definition": def, "query": {"type": "string"}, "include_flows": {"type": "boolean"}, "category": {"type": "string"}}), &["definition", "query"])),
        s("search_instances", "Search process instances with the query language: `def:key ver:3 id:uuid bk:ABC* act:Task_x exec:Task_y var.amount>1000 var.name=\"x y\" state:active|suspended|running|finished|completed|terminated|any inc:yes|failedJob|~msg started>2026-10-01 finished<-1d by:user super:uuid tenant:t`. Routes to the runtime or history API automatically.", obj(json!({"query": {"type": "string"}, "page": {"type": "integer"}, "page_size": {"type": "integer", "default": 50}, "mode": {"type": "string", "enum": ["auto", "runtime", "history"]}}), &["query"])),
        s("get_instance", "Process instance overview: state, times, definition, tokens (active activities), open incidents, user tasks, jobs, external tasks, called instances, variable names.", obj(json!({"id": inst}), &["id"])),
        s("get_variables", "Variables of an instance with full values (runtime for running, history for finished).", obj(json!({"id": inst, "names": {"type": "array", "items": {"type": "string"}}}), &["id"])),
        s("get_activity_history", "Historic activity instances of an instance (the path it took) with start/end/duration/assignee/cancelled.", obj(json!({"id": inst}), &["id"])),
        s("list_incidents", "Open incidents, optionally filtered by instance or definition.", obj(json!({"process_instance_id": {"type": "string"}, "definition_id": {"type": "string"}, "limit": {"type": "integer"}}), &[])),
        s("get_error_details", "Stack trace of a failed job or error details of an external task.", obj(json!({"job_id": {"type": "string"}, "external_task_id": {"type": "string"}}), &[])),
        s("list_tasks", "Open user tasks, filtered by instance id, assignee or candidate group.", obj(json!({"process_instance_id": {"type": "string"}, "assignee": {"type": "string"}, "candidate_group": {"type": "string"}, "limit": {"type": "integer"}}), &[])),
        s("list_jobs", "Jobs (timers, async continuations) of an instance, with due dates, retries and failures.", obj(json!({"process_instance_id": inst}), &["process_instance_id"])),
        s("list_external_tasks", "External tasks by instance or topic.", obj(json!({"process_instance_id": {"type": "string"}, "topic": {"type": "string"}, "limit": {"type": "integer"}}), &[])),
        s("list_batches", "Running batches with progress, and recently finished ones.", obj(json!({}), &[])),
        // ---------- engine: write ----------
        s("set_variable", "Set a process variable (creates or overwrites). Types: String, Integer, Long, Double, Boolean, Date, Json, Xml, Object, Null. For Json/Object pass the JSON as a string or object.", obj(json!({"id": inst, "name": {"type": "string"}, "type": {"type": "string", "default": "String"}, "value": {}}), &["id", "name", "value"])),
        s("delete_variable", "Delete a process variable.", obj(json!({"id": inst, "name": {"type": "string"}, "confirm": {"type": "boolean"}}), &["id", "name", "confirm"])),
        s("start_instance", "Start a new process instance by definition key (optionally at specific activities).", obj(json!({"key": {"type": "string"}, "business_key": {"type": "string"}, "variables": {"type": "object", "description": "name → value (typed automatically) or name → {value,type}"}, "start_before": {"type": "array", "items": {"type": "string"}, "description": "activity ids to start before instead of the start event"}}), &["key"])),
        s("suspend_instance", "Suspend or activate a process instance.", obj(json!({"id": inst, "suspended": {"type": "boolean"}}), &["id", "suspended"])),
        s("delete_instance", "Delete (cancel) a running process instance. Irreversible; requires confirm.", obj(json!({"id": inst, "reason": {"type": "string"}, "skip_custom_listeners": {"type": "boolean"}, "confirm": {"type": "boolean"}}), &["id", "confirm"])),
        s("modify_instance", "Token modification: instructions like {type: startBeforeActivity|startAfterActivity|cancel, activityId, variables?}. Requires confirm.", obj(json!({"id": inst, "instructions": {"type": "array", "items": {"type": "object"}}, "skip_custom_listeners": {"type": "boolean"}, "skip_io_mappings": {"type": "boolean"}, "annotation": {"type": "string"}, "confirm": {"type": "boolean"}}), &["id", "instructions", "confirm"])),
        s("restart_instance", "Restart a finished instance from given activities (keeps its variables). Requires confirm.", obj(json!({"id": inst, "start_before": {"type": "array", "items": {"type": "string"}}, "start_after": {"type": "array", "items": {"type": "string"}}, "confirm": {"type": "boolean"}}), &["id", "confirm"])),
        s("retry_job", "Set retries of a job (1 retries a failed job).", obj(json!({"job_id": {"type": "string"}, "retries": {"type": "integer", "default": 1}}), &["job_id"])),
        s("execute_job", "Execute a job now (timer or async continuation).", obj(json!({"job_id": {"type": "string"}}), &["job_id"])),
        s("set_job_duedate", "Set a job's due date (ISO 8601, or `now`, `-1h`, `2026-10-20T12:00`).", obj(json!({"job_id": {"type": "string"}, "due": {"type": "string"}}), &["job_id", "due"])),
        s("retry_external_task", "Set retries of an external task.", obj(json!({"external_task_id": {"type": "string"}, "retries": {"type": "integer", "default": 1}}), &["external_task_id"])),
        s("unlock_external_task", "Unlock an external task held by a worker.", obj(json!({"external_task_id": {"type": "string"}}), &["external_task_id"])),
        s("set_task_assignee", "Assign a user task (omit user to unassign).", obj(json!({"task_id": {"type": "string"}, "user": {"type": "string"}}), &["task_id"])),
        s("complete_task", "Complete a user task with optional variables.", obj(json!({"task_id": {"type": "string"}, "variables": {"type": "object"}, "confirm": {"type": "boolean"}}), &["task_id", "confirm"])),
        s("correlate_message", "Correlate a BPMN message (by name) to waiting instances.", obj(json!({"message_name": {"type": "string"}, "business_key": {"type": "string"}, "process_instance_id": {"type": "string"}, "variables": {"type": "object"}, "all": {"type": "boolean"}}), &["message_name"])),
        s("resolve_incident", "Delete an incident (does not fix the cause).", obj(json!({"incident_id": {"type": "string"}, "confirm": {"type": "boolean"}}), &["incident_id", "confirm"])),
        s("annotate_incident", "Set the annotation text of an incident.", obj(json!({"incident_id": {"type": "string"}, "annotation": {"type": "string"}}), &["incident_id", "annotation"])),
        s("suspend_definition", "Suspend or activate a definition and its instances.", obj(json!({"definition_id": {"type": "string"}, "suspended": {"type": "boolean"}, "confirm": {"type": "boolean"}}), &["definition_id", "suspended", "confirm"])),
        s("batch", "Bulk operation on many instances as a Camunda batch: suspend | activate | retry_jobs | delete | set_variables.", obj(json!({"operation": {"type": "string", "enum": ["suspend", "activate", "retry_jobs", "delete", "set_variables"]}, "instance_ids": {"type": "array", "items": {"type": "string"}}, "variables": {"type": "object"}, "reason": {"type": "string"}, "confirm": {"type": "boolean"}}), &["operation", "instance_ids", "confirm"])),
        // ---------- UI ----------
        s("ui_state", "What the UI shows now: tabs, active place, selection, camera, dock tab, find state, filters, pending confirmation, recent toasts.", obj(json!({}), &[])),
        s("ui_navigate", "Open a place: `definitions`, `def:<key or id>`, `instance:<id>`, `search:<query>`, `incidents`, `batches`, `settings`.", obj(json!({"to": {"type": "string"}, "new_tab": {"type": "boolean"}}), &["to"])),
        s("ui_tabs", "Manage tabs: list | activate | close | new | back | forward.", obj(json!({"action": {"type": "string", "enum": ["list", "activate", "close", "new", "back", "forward"]}, "index": {"type": "integer"}}), &["action"])),
        s("ui_select", "Select an element on the current diagram by id (reveals it); omit to clear the selection.", obj(json!({"element_id": {"type": "string"}, "fit": {"type": "boolean", "description": "zoom to it"}}), &[])),
        s("ui_fit", "Fit the camera: all | selection | tokens | element (with element_id).", obj(json!({"what": {"type": "string", "enum": ["all", "selection", "tokens", "element"]}, "element_id": {"type": "string"}}), &["what"])),
        s("ui_zoom", "Set the zoom level (1 = 100%) or multiply by factor, around the viewport centre.", obj(json!({"level": {"type": "number"}, "factor": {"type": "number"}}), &[])),
        s("ui_pan", "Pan the diagram by screen pixels.", obj(json!({"dx": {"type": "number"}, "dy": {"type": "number"}}), &["dx", "dy"])),
        s("ui_find", "Open the in-diagram find bar with a query (returns matches), or close it with an empty query.", obj(json!({"query": {"type": "string"}, "include_flows": {"type": "boolean"}}), &[])),
        s("ui_dock", "Show a dock tab by name or index (e.g. Variables, Incidents, Timeline, Modify) or toggle the dock.", obj(json!({"tab": {"type": "string"}, "visible": {"type": "boolean"}}), &[])),
        s("ui_filter", "Set the current place's filter / query text (definitions filter, instance list filter, variables filter, or the search query which is executed).", obj(json!({"text": {"type": "string"}}), &["text"])),
        s("ui_command", "Run a named command (see ui_commands): e.g. refresh, toggle_theme, toggle_dock, back, forward, palette, help.", obj(json!({"name": {"type": "string"}}), &["name"])),
        s("ui_commands", "List commands available to ui_command with their shortcuts and enabled state.", obj(json!({}), &[])),
        s("ui_key", "Press a key (egui key name, e.g. Tab, Enter, Escape, ArrowDown, F, Num0, Plus) with optional modifiers; handled by the focused view like a real key press.", obj(json!({"key": {"type": "string"}, "ctrl": {"type": "boolean"}, "shift": {"type": "boolean"}, "alt": {"type": "boolean"}, "repeat": {"type": "integer", "default": 1}}), &["key"])),
        s("ui_type", "Type text into the focused text field.", obj(json!({"text": {"type": "string"}}), &["text"])),
        s("ui_hit_test", "What is under a screen point (x, y in logical pixels) on the diagram.", obj(json!({"x": {"type": "number"}, "y": {"type": "number"}}), &["x", "y"])),
        s("ui_screenshot", "Screenshot of the window as PNG (returned as image; also saved to path if given). max_width downsizes to save tokens.", obj(json!({"path": {"type": "string"}, "max_width": {"type": "integer", "default": 1280}}), &[])),
        s("ui_confirm", "Accept or cancel the pending confirmation dialog.", obj(json!({"accept": {"type": "boolean"}}), &["accept"])),
        s("ui_modification_queue", "Read or edit the Modify tab of the open instance: list | add (instruction) | clear | apply.", obj(json!({"action": {"type": "string", "enum": ["list", "add", "clear", "apply"]}, "instruction": {"type": "object"}}), &["action"])),
        s("ui_settings", "Read or change settings: theme (system|light|dark), density, wheel_zooms, inertia, pointer_wrap (auto|off), refresh_instance_secs, refresh_list_secs, page_size, paused.", obj(json!({"set": {"type": "object"}}), &[])),
        s("ui_connect", "Connect to an engine (URL, user, password) or disconnect.", obj(json!({"url": {"type": "string"}, "user": {"type": "string"}, "password": {"type": "string"}, "disconnect": {"type": "boolean"}}), &[])),
    ]
}

pub fn specs() -> Vec<Value> {
    specs_vec().into_iter().map(|s| json!({"name": s.name, "description": s.description, "inputSchema": s.schema})).collect()
}

pub fn exists(name: &str) -> bool {
    specs_vec().iter().any(|s| s.name == name)
}

pub fn prompts() -> Vec<Value> {
    vec![
        json!({"name": "triage_incidents", "title": "Triage incidents", "description": "Walk through open incidents, group by root cause, propose retries or fixes."}),
        json!({"name": "explain_instance", "title": "Explain an instance", "description": "Explain where a process instance is, how it got there and what blocks it.", "arguments": [{"name": "id", "required": true}]}),
    ]
}

pub fn prompt(name: &str) -> Option<Value> {
    let text = match name {
        "triage_incidents" => "Use list_incidents, then get_error_details for each distinct failure, group them by root cause, and propose for each group whether to retry (retry_job / batch retry_jobs), fix data (set_variable) or escalate. Show the user the affected instances with ui_navigate to a search `inc:yes`.".to_string(),
        "explain_instance" => "Call get_instance and get_activity_history for the given id, then get_element for the activities where tokens wait, and explain in plain words where the instance is, which path it took, what it waits for (timer due, task assignee, external worker) and any incidents. Open it in the UI with ui_navigate and ui_fit tokens.".to_string(),
        _ => return None,
    };
    Some(json!({"messages": [{"role": "user", "content": {"type": "text", "text": text}}]}))
}

// ---------------- helpers ----------------

fn arg_str<'a>(a: &'a Value, k: &str) -> Option<&'a str> {
    a.get(k).and_then(|v| v.as_str()).filter(|s| !s.is_empty())
}
fn arg_bool(a: &Value, k: &str) -> Option<bool> {
    a.get(k).and_then(|v| v.as_bool())
}
fn arg_i(a: &Value, k: &str) -> Option<i64> {
    a.get(k).and_then(|v| v.as_i64())
}
fn arg_f(a: &Value, k: &str) -> Option<f64> {
    a.get(k).and_then(|v| v.as_f64())
}
fn need<'a>(a: &'a Value, k: &str) -> Result<&'a str, String> {
    arg_str(a, k).ok_or_else(|| format!("missing argument `{k}`"))
}
fn confirmed(a: &Value) -> Result<(), String> {
    if arg_bool(a, "confirm") == Some(true) {
        Ok(())
    } else {
        Err("this tool changes or destroys data; call again with confirm: true".into())
    }
}

/// Run an engine call and answer through the reply (JSON result).
fn engine<T, F, Fut>(app: &mut App, reply: Reply, f: F)
where
    T: serde::Serialize + Send + 'static,
    F: FnOnce(std::sync::Arc<camunda::Client>) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<T, camunda::ApiError>> + Send + 'static,
{
    let Some(net) = &app.net else {
        reply.err("not connected to an engine (use ui_connect)");
        return;
    };
    net.run(f, move |app, r| {
        match r {
            Ok(v) => reply.ok(serde_json::to_value(v).unwrap_or(Value::Null)),
            Err(e) => reply.err(e.to_string()),
        }
        app.bump_refresh();
    });
}

fn element_summary(m: &bpmn::Model, i: usize) -> Value {
    let e = &m.elements[i];
    json!({
        "id": e.id,
        "name": e.name,
        "type": e.kind.label(),
        "category": e.kind.category(),
        "lane": e.lane.map(|l| m.elements[l].display_name().to_string()),
        "parent": e.parent.map(|p| m.elements[p].id.clone()),
        "implementation": crate::viewport::widget::implementation_summary(e),
        "incoming": e.incoming.iter().filter_map(|&f| m.flows[f].source.map(|s| m.elements[s].id.clone())).collect::<Vec<_>>(),
        "outgoing": e.outgoing.iter().filter_map(|&f| m.flows[f].target.map(|t| m.elements[t].id.clone())).collect::<Vec<_>>(),
        "bounds": e.bounds,
    })
}

fn element_full(m: &bpmn::Model, i: usize) -> Value {
    let e = &m.elements[i];
    let flows: Vec<Value> = e
        .outgoing
        .iter()
        .map(|&f| {
            let fl = &m.flows[f];
            json!({"id": fl.id, "name": fl.name, "kind": fl.kind.label(), "target": fl.target.map(|t| m.elements[t].id.clone()), "condition": fl.condition.as_ref().map(|c| c.text.clone()), "default": fl.is_default})
        })
        .collect();
    let incoming: Vec<Value> = e.incoming.iter().map(|&f| json!({"id": m.flows[f].id, "source": m.flows[f].source.map(|s| m.elements[s].id.clone())})).collect();
    json!({
        "id": e.id, "name": e.name, "type": e.kind.label(), "documentation": e.documentation,
        "attributes": e.attrs.iter().map(|(k, v)| json!({"name": k, "value": v})).collect::<Vec<_>>(),
        "extensions": e.ext, "event_definitions": e.event_defs, "attached_to": e.attached_to, "interrupting": e.interrupting,
        "default_flow": e.default_flow, "multi_instance": e.multi_instance, "script": e.script,
        "lane": e.lane.map(|l| m.elements[l].display_name().to_string()), "parent": e.parent.map(|p| m.elements[p].id.clone()),
        "boundary_events": e.boundary_events.iter().map(|&b| m.elements[b].id.clone()).collect::<Vec<_>>(),
        "outgoing": flows, "incoming": incoming,
        "xml": bpmn::text::pretty_xml(m.xml_of_element(e)),
    })
}

/// Resolve a definition id or key to an id using the cache (keys need the definition list).
fn resolve_definition(app: &App, s: &str) -> Option<String> {
    if s.contains(':') {
        return Some(s.to_string());
    }
    app.cache.defs.data.as_ref().and_then(|d| d.iter().find(|d| d.key == s)).map(|d| d.id.clone())
}

fn variable_value(v: &Value, type_: Option<&str>) -> camunda::VariableValue {
    use camunda::VariableValue as VV;
    match type_ {
        Some(t) if t != "auto" => match t {
            "Json" | "Object" => {
                let s = match v {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                let mut vv = VV::new(t, Value::String(s));
                if t == "Object" {
                    vv.value_info = json!({"serializationDataFormat": "application/json", "objectTypeName": "java.util.LinkedHashMap"});
                }
                vv
            }
            "Null" => VV { type_: Some("Null".into()), value: Value::Null, value_info: json!({}) },
            _ => VV::new(t, v.clone()),
        },
        _ => match v {
            Value::String(s) => VV::string(s),
            Value::Bool(b) => VV::new("Boolean", json!(b)),
            Value::Number(n) if n.is_i64() => VV::new("Long", v.clone()),
            Value::Number(_) => VV::new("Double", v.clone()),
            Value::Null => VV { type_: Some("Null".into()), value: Value::Null, value_info: json!({}) },
            other => VV::new("Json", Value::String(other.to_string())),
        },
    }
}

fn variables_map(v: Option<&Value>) -> serde_json::Map<String, Value> {
    let mut out = serde_json::Map::new();
    if let Some(Value::Object(m)) = v {
        for (k, val) in m {
            let vv = match val {
                Value::Object(o) if o.contains_key("value") => variable_value(&o["value"], o.get("type").and_then(|t| t.as_str())),
                other => variable_value(other, None),
            };
            out.insert(k.clone(), serde_json::to_value(vv).unwrap());
        }
    }
    out
}

fn place_json(app: &App) -> Value {
    let Some(tab) = app.tabs.get(app.active) else { return json!(null) };
    match tab.place() {
        Place::Definitions(p) => json!({"kind": "definitions", "filter": p.filter, "cursor": p.cursor}),
        Place::Definition(p) => json!({
            "kind": "definition", "definition_id": p.def_id, "key": p.def.data.as_ref().map(|d| d.key.clone()), "name": p.def.data.as_ref().map(|d| d.display_name().to_string()),
            "dock_tab": crate::places::definition::DOCK_TABS.get(p.dock_tab), "activity_filter": p.activity_filter, "instance_filter": p.instance_filter,
            "diagram": diagram_json(app, &p.view, &p.def_id),
            "running_by_activity": p.overlays.running, "incidents_by_activity": p.overlays.incidents,
        }),
        Place::Instance(p) => json!({
            "kind": "instance", "id": p.id, "business_key": p.hist.data.as_ref().and_then(|h| h.business_key.clone()), "state": p.hist.data.as_ref().and_then(|h| h.state.clone()),
            "definition_id": p.hist.data.as_ref().map(|h| h.process_definition_id.clone()),
            "dock_tab": crate::places::instance::DOCK_TABS.get(p.dock_tab), "variables_filter": p.var_filter,
            "active_activities": p.overlays.active, "incidents_by_activity": p.overlays.incidents, "visited": p.overlays.completed,
            "modification_queue": p.modification.instructions, "pick_mode": p.view.pick.is_some(), "trace_open": p.trace.as_ref().map(|t| t.title.clone()),
            "diagram": p.hist.data.as_ref().map(|h| diagram_json(app, &p.view, &h.process_definition_id)),
        }),
        Place::Search(p) => json!({"kind": "search", "query": p.query, "executed": p.executed, "result_count": p.count.data, "rows_loaded": p.rows.data.as_ref().map(|r| r.len()), "cursor": p.cursor, "selected": p.selected.len(), "variable_columns": p.var_columns}),
        Place::Incidents(p) => json!({"kind": "incidents", "filter": p.filter, "count": p.list.data.as_ref().map(|l| l.len())}),
        Place::Batches(_) => json!({"kind": "batches"}),
        Place::Settings(_) => json!({"kind": "settings"}),
        Place::File(p) => json!({"kind": "file", "path": p.path, "diagram": p.model.as_ref().map(|m| diagram_json_model(&p.view, m))}),
    }
}

fn diagram_json(app: &App, view: &crate::viewport::DiagramView, def_id: &str) -> Value {
    match app.cache.diagrams.get(def_id).and_then(|l| l.data.clone()) {
        Some(m) => diagram_json_model(view, &m),
        None => json!({"loaded": false}),
    }
}

fn diagram_json_model(view: &crate::viewport::DiagramView, m: &bpmn::Model) -> Value {
    let sel = view.selection.map(|r| match r {
        bpmn::Ref::Element(i) => json!({"element_id": m.elements[i].id, "name": m.elements[i].display_name(), "type": m.elements[i].kind.label()}),
        bpmn::Ref::Flow(i) => json!({"flow_id": m.flows[i].id}),
    });
    json!({
        "loaded": true, "elements": m.flow_nodes().count(), "flows": m.flows.len(),
        "selection": sel, "zoom": view.camera.target.zoom, "center": [view.camera.target.center.x, view.camera.target.center.y],
        "camera_animating": view.camera.is_animating(),
        "find": if view.find.open { json!({"query": view.find.query, "matches": view.find.matches.len(), "current": view.find.cursor}) } else { Value::Null },
        "minimap": view.show_minimap, "show_ids": view.show_ids, "focused": view.focused,
    })
}

fn with_diagram<R>(app: &mut App, f: impl FnOnce(&mut crate::viewport::DiagramView, &bpmn::Model) -> R) -> Result<R, String> {
    let active = app.active;
    let def_id = match app.tabs.get(active).map(|t| t.place()) {
        Some(Place::Definition(p)) => p.def_id.clone(),
        Some(Place::Instance(p)) => p.hist.data.as_ref().map(|h| h.process_definition_id.clone()).ok_or(RETRY)?,
        Some(Place::File(p)) => {
            let m = p.model.clone().ok_or("file not parsed")?;
            if let Some(Place::File(p)) = app.tabs.get_mut(active).map(|t| t.place_mut()) {
                return Ok(f(&mut p.view, &m));
            }
            return Err("no diagram".into());
        }
        _ => return Err("the active place has no diagram (open a definition or instance first)".into()),
    };
    let m = app.cache.diagrams.get(&def_id).and_then(|l| l.data.clone()).ok_or(RETRY)?;
    match app.tabs.get_mut(active).map(|t| t.place_mut()) {
        Some(Place::Definition(p)) => Ok(f(&mut p.view, &m)),
        Some(Place::Instance(p)) => Ok(f(&mut p.view, &m)),
        _ => Err("no diagram".into()),
    }
}

fn parse_key(name: &str) -> Option<egui::Key> {
    egui::Key::from_name(name).or_else(|| match name.to_lowercase().as_str() {
        "esc" => Some(egui::Key::Escape),
        "return" => Some(egui::Key::Enter),
        "up" => Some(egui::Key::ArrowUp),
        "down" => Some(egui::Key::ArrowDown),
        "left" => Some(egui::Key::ArrowLeft),
        "right" => Some(egui::Key::ArrowRight),
        "0" => Some(egui::Key::Num0),
        "1" => Some(egui::Key::Num1),
        "+" => Some(egui::Key::Plus),
        "-" => Some(egui::Key::Minus),
        "/" => Some(egui::Key::Slash),
        _ => None,
    })
}

/// Not ready yet (diagram downloading, definitions not listed): re-queue the call for a later
/// frame instead of failing, up to ~15 s.
fn retry_later(app: &mut App, tool: String, mut args: Value, reply: Reply, why: &str) {
    let attempt = args.get("_attempt").and_then(|v| v.as_u64()).unwrap_or(0) + 1;
    if attempt > 150 {
        reply.err(format!("gave up waiting: {why}"));
        return;
    }
    if let Some(o) = args.as_object_mut() {
        o.insert("_attempt".into(), json!(attempt));
    }
    app.mcp_queue.lock().unwrap().push_back(McpCall { tool, args, reply });
    if let Some(ctx) = &app.ctx {
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}

// ---------------- dispatch ----------------

pub fn dispatch(app: &mut App, call: McpCall) {
    let McpCall { tool, args: a, reply } = call;
    // Make sure the definition list exists for key → id resolution.
    if matches!(tool.as_str(), "list_elements" | "get_element" | "find_elements") {
        if let Some(d) = arg_str(&a, "definition") {
            if !d.contains(':') && app.cache.defs.data.is_none() {
                app.ensure_definitions();
                return retry_later(app, tool, a, reply, "definition list not loaded");
            }
        }
    }
    let r: Result<Option<Value>, String> = (|| {
        match tool.as_str() {
            // ----- engine: read -----
            "engine_status" => {
                let st = app.mcp.as_ref().map(|m| m.stats.lock().unwrap().clone());
                Ok(Some(json!({
                    "connected": app.net.is_some(), "status": format!("{:?}", app.conn), "url": app.profile.url, "user": app.profile.user,
                    "platform": crate::platform::describe(), "pointer_warp_supported": crate::platform::warp_supported(),
                    "latency_ms": app.latency_ms, "auto_refresh_paused": app.paused, "tabs": app.tabs.len(),
                    "mcp": st.map(|s| json!({"requests": s.requests, "tool_calls": s.tool_calls, "errors": s.errors, "last_tool": s.last_tool})),
                })))
            }
            "list_definitions" => {
                let all = arg_bool(&a, "all_versions").unwrap_or(false);
                let filter = arg_str(&a, "filter").map(|s| s.to_lowercase());
                engine(app, reply.clone(), move |c| async move {
                    let defs = c.process_definitions(!all).await?;
                    let stats = c.process_definition_statistics().await.unwrap_or_default();
                    let mut out = vec![];
                    for d in defs {
                        if let Some(f) = &filter {
                            if !d.key.to_lowercase().contains(f) && !d.name.as_deref().unwrap_or("").to_lowercase().contains(f) {
                                continue;
                            }
                        }
                        let (running, incidents): (i64, i64) = stats.iter().filter(|s| if all { s.id == d.id } else { s.definition.key == d.key }).fold((0, 0), |acc, s| (acc.0 + s.instances, acc.1 + s.incidents.iter().map(|i| i.incident_count).sum::<i64>()));
                        out.push(json!({"id": d.id, "key": d.key, "name": d.name, "version": d.version, "suspended": d.suspended, "running": running, "incidents": incidents, "version_tag": d.version_tag}));
                    }
                    Ok(out)
                });
                Ok(None)
            }
            "get_definition" => {
                let d = need(&a, "definition")?.to_string();
                engine(app, reply.clone(), move |c| async move {
                    let def = if d.contains(':') { c.process_definition(&d).await? } else { c.process_definition_by_key(&d).await? };
                    let versions = c.process_definition_versions(&def.key).await.unwrap_or_default();
                    let stats = c.activity_statistics(&def.id).await.unwrap_or_default();
                    Ok(json!({"definition": def, "versions": versions.iter().map(|v| json!({"id": v.id, "version": v.version, "suspended": v.suspended})).collect::<Vec<_>>(), "activity_statistics": stats}))
                });
                Ok(None)
            }
            "list_elements" | "get_element" | "find_elements" => {
                let d = need(&a, "definition")?;
                let def_id = resolve_definition(app, d).ok_or("unknown definition key; call list_definitions first or pass the full id")?;
                let Some(m) = app.diagram(&def_id) else {
                    if let Some(e) = app.cache.diagram_error(&def_id) {
                        return Err(format!("cannot load diagram: {e}"));
                    }
                    return Err(RETRY.into());
                };
                let v = match tool.as_str() {
                    "list_elements" => {
                        let cat = arg_str(&a, "type");
                        let f = arg_str(&a, "filter").map(|s| s.to_lowercase());
                        let limit = arg_i(&a, "limit").unwrap_or(500) as usize;
                        let mut out = vec![];
                        if cat == Some("flow") {
                            for fl in &m.flows {
                                out.push(json!({"id": fl.id, "name": fl.name, "kind": fl.kind.label(), "source": fl.source.map(|s| m.elements[s].id.clone()), "target": fl.target.map(|t| m.elements[t].id.clone()), "condition": fl.condition.as_ref().map(|c| c.text.clone()), "default": fl.is_default}));
                            }
                        } else {
                            for &i in &m.reading_order {
                                let e = &m.elements[i];
                                if let Some(c) = cat {
                                    if e.kind.category() != c {
                                        continue;
                                    }
                                }
                                if let Some(f) = &f {
                                    if !e.id.to_lowercase().contains(f) && !e.name.as_deref().unwrap_or("").to_lowercase().contains(f) {
                                        continue;
                                    }
                                }
                                out.push(element_summary(&m, i));
                                if out.len() >= limit {
                                    break;
                                }
                            }
                            if cat.map(|c| c == "lane" || c == "pool").unwrap_or(false) {
                                for (i, e) in m.elements.iter().enumerate() {
                                    if e.kind.category() == cat.unwrap() {
                                        out.push(element_summary(&m, i));
                                    }
                                }
                            }
                        }
                        json!({"definition_id": def_id, "count": out.len(), "elements": out})
                    }
                    "get_element" => {
                        let id = need(&a, "element_id")?;
                        match m.lookup(id) {
                            Some(bpmn::Ref::Element(i)) => element_full(&m, i),
                            Some(bpmn::Ref::Flow(i)) => {
                                let f = &m.flows[i];
                                json!({"id": f.id, "name": f.name, "kind": f.kind.label(), "source": f.source.map(|s| m.elements[s].id.clone()), "target": f.target.map(|t| m.elements[t].id.clone()), "condition": f.condition, "default": f.is_default, "attributes": f.attrs, "extensions": f.ext, "xml": bpmn::text::pretty_xml(m.xml_of_flow(f))})
                            }
                            None => return Err(format!("no element {id} in {def_id}")),
                        }
                    }
                    _ => {
                        let q = need(&a, "query")?;
                        let opts = bpmn::search::SearchOptions { case_sensitive: false, categories: arg_str(&a, "category").map(|c| vec![c.to_string()]).unwrap_or_default(), include_flows: arg_bool(&a, "include_flows").unwrap_or(true) };
                        let res = bpmn::search::search(&m, q, &opts);
                        json!({"count": res.len(), "matches": res.iter().take(200).map(|r| match r.target {
                            bpmn::Ref::Element(i) => json!({"element_id": m.elements[i].id, "name": m.elements[i].display_name(), "type": m.elements[i].kind.label(), "field": r.field, "snippet": crate::util::ellipsize(&r.snippet, 160)}),
                            bpmn::Ref::Flow(i) => json!({"flow_id": m.flows[i].id, "name": m.flows[i].display_name(), "field": r.field, "snippet": crate::util::ellipsize(&r.snippet, 160)}),
                        }).collect::<Vec<_>>()})
                    }
                };
                Ok(Some(v))
            }
            "search_instances" => {
                let q = crate::query::parse(need(&a, "query")?);
                let mode = match arg_str(&a, "mode") {
                    Some("runtime") => crate::query::Mode::Runtime,
                    Some("history") => crate::query::Mode::History,
                    _ => {
                        if crate::query::needs_history(&q) {
                            crate::query::Mode::History
                        } else {
                            crate::query::Mode::Runtime
                        }
                    }
                };
                let page = arg_i(&a, "page").unwrap_or(0).max(0) as usize;
                let size = arg_i(&a, "page_size").unwrap_or(50).clamp(1, 500) as usize;
                let defs = app.cache.all_defs.data.clone().unwrap_or_default();
                let warnings = q.warnings.clone();
                let normalized = crate::query::print(&q);
                match mode {
                    crate::query::Mode::Runtime => {
                        let rq = crate::query::build_runtime(&q, &defs);
                        engine(app, reply.clone(), move |c| async move {
                            let rows = c.process_instances(&rq, page * size, size).await?;
                            let count = c.process_instances_count(&rq).await?;
                            Ok(json!({"mode": "runtime", "count": count, "page": page, "page_size": size, "query": normalized, "warnings": warnings, "instances": rows, "request": rq}))
                        });
                    }
                    crate::query::Mode::History => {
                        let hq = crate::query::build_history(&q, &defs);
                        engine(app, reply.clone(), move |c| async move {
                            let rows = c.history_process_instances(&hq, page * size, size).await?;
                            let count = c.history_process_instances_count(&hq).await?;
                            Ok(json!({"mode": "history", "count": count, "page": page, "page_size": size, "query": normalized, "warnings": warnings, "instances": rows, "request": hq}))
                        });
                    }
                }
                Ok(None)
            }
            "get_instance" => {
                let id = need(&a, "id")?.to_string();
                engine(app, reply.clone(), move |c| async move {
                    let h = c.history_process_instance(&id).await?;
                    let finished = h.end_time.is_some();
                    let (tokens, suspended) = if finished {
                        (vec![], false)
                    } else {
                        let ai = c.activity_instances(&id).await.ok();
                        let rt = c.process_instance(&id).await.ok();
                        (ai.map(|a| a.tokens()).unwrap_or_default(), rt.map(|r| r.suspended).unwrap_or(false))
                    };
                    let incidents = if finished { c.history_incidents(&id).await.unwrap_or_default() } else { c.incidents(&[("processInstanceId", id.clone())]).await.unwrap_or_default() };
                    let tasks = if finished { vec![] } else { c.tasks(&[("processInstanceId", id.clone())]).await.unwrap_or_default() };
                    let jobs = if finished { vec![] } else { c.jobs(&[("processInstanceId", id.clone())]).await.unwrap_or_default() };
                    let ext = if finished { vec![] } else { c.external_tasks(&[("processInstanceId", id.clone())]).await.unwrap_or_default() };
                    let called = c.called_process_instances(&id).await.unwrap_or_default();
                    let vars = if finished { c.history_variables(&id).await.unwrap_or_default() } else { c.variables(&id).await.unwrap_or_default() };
                    Ok(json!({
                        "instance": h, "finished": finished, "suspended": suspended,
                        "tokens": tokens.iter().map(|t| json!({"activity_id": t.activity_id, "activity_type": t.activity_type, "activity_instance_id": t.id, "async_continuation": t.is_transition, "scope": t.path, "incident_ids": t.incident_ids})).collect::<Vec<_>>(),
                        "incidents": incidents.iter().map(|i| json!({"id": i.id, "type": i.incident_type, "activity_id": i.activity_id, "message": i.incident_message, "time": i.time(), "job_or_external_task_id": i.configuration, "resolved": i.end_time.is_some()})).collect::<Vec<_>>(),
                        "user_tasks": tasks.iter().map(|t| json!({"id": t.id, "name": t.name, "activity_id": t.task_definition_key, "assignee": t.assignee, "created": t.created, "due": t.due, "priority": t.priority})).collect::<Vec<_>>(),
                        "jobs": jobs.iter().map(|j| json!({"id": j.id, "due": j.due_date, "retries": j.retries, "exception": j.exception_message, "failed_activity_id": j.failed_activity_id, "suspended": j.suspended})).collect::<Vec<_>>(),
                        "external_tasks": ext.iter().map(|e| json!({"id": e.id, "activity_id": e.activity_id, "topic": e.topic_name, "worker": e.worker_id, "retries": e.retries, "error": e.error_message, "lock_expiration": e.lock_expiration_time})).collect::<Vec<_>>(),
                        "called_instances": called.iter().map(|p| json!({"id": p.id, "definition_key": p.process_definition_key, "business_key": p.business_key, "state": p.state})).collect::<Vec<_>>(),
                        "variables": vars.iter().map(|v| json!({"name": v.name, "type": v.type_, "preview": crate::util::value_preview(&v.value, v.type_.as_deref())})).collect::<Vec<_>>(),
                    }))
                });
                Ok(None)
            }
            "get_variables" => {
                let id = need(&a, "id")?.to_string();
                let names: Option<Vec<String>> = a.get("names").and_then(|v| v.as_array()).map(|v| v.iter().filter_map(|x| x.as_str().map(String::from)).collect());
                engine(app, reply.clone(), move |c| async move {
                    let h = c.history_process_instance(&id).await?;
                    let vars = if h.end_time.is_some() { c.history_variables(&id).await? } else { c.variables(&id).await? };
                    Ok(vars.into_iter().filter(|v| names.as_ref().map(|n| n.contains(&v.name)).unwrap_or(true)).map(|v| {
                        let parsed = match (&v.value, v.type_.as_deref()) {
                            (Value::String(s), Some("Json") | Some("Object")) => serde_json::from_str::<Value>(s).ok(),
                            _ => None,
                        };
                        json!({"name": v.name, "type": v.type_, "value": v.value, "parsed": parsed, "value_info": v.value_info, "execution_id": v.execution_id, "activity_instance_id": v.activity_instance_id})
                    }).collect::<Vec<_>>())
                });
                Ok(None)
            }
            "get_activity_history" => {
                let id = need(&a, "id")?.to_string();
                engine(app, reply.clone(), move |c| async move {
                    let acts = c.history_activity_instances(&id).await?;
                    Ok(acts.iter().map(|x| json!({"activity_id": x.base_activity_id(), "name": x.activity_name, "type": x.activity_type, "start": x.start_time, "end": x.end_time, "duration_ms": x.duration_in_millis, "assignee": x.assignee, "cancelled": x.canceled, "called_instance": x.called_process_instance_id, "task_id": x.task_id})).collect::<Vec<_>>())
                });
                Ok(None)
            }
            "list_incidents" => {
                let mut q: Vec<(&'static str, String)> = vec![];
                if let Some(p) = arg_str(&a, "process_instance_id") {
                    q.push(("processInstanceId", p.to_string()));
                }
                if let Some(d) = arg_str(&a, "definition_id") {
                    q.push(("processDefinitionId", d.to_string()));
                }
                q.push(("maxResults", arg_i(&a, "limit").unwrap_or(200).to_string()));
                engine(app, reply.clone(), move |c| async move { c.incidents(&q).await });
                Ok(None)
            }
            "get_error_details" => {
                if let Some(j) = arg_str(&a, "job_id").map(String::from) {
                    engine(app, reply.clone(), move |c| async move { c.job_stacktrace(&j).await });
                } else if let Some(e) = arg_str(&a, "external_task_id").map(String::from) {
                    engine(app, reply.clone(), move |c| async move { c.external_task_error_details(&e).await });
                } else {
                    return Err("pass job_id or external_task_id".into());
                }
                Ok(None)
            }
            "list_tasks" => {
                let mut q: Vec<(&'static str, String)> = vec![];
                if let Some(p) = arg_str(&a, "process_instance_id") {
                    q.push(("processInstanceId", p.to_string()));
                }
                if let Some(p) = arg_str(&a, "assignee") {
                    q.push(("assignee", p.to_string()));
                }
                if let Some(p) = arg_str(&a, "candidate_group") {
                    q.push(("candidateGroup", p.to_string()));
                }
                q.push(("maxResults", arg_i(&a, "limit").unwrap_or(200).to_string()));
                engine(app, reply.clone(), move |c| async move { c.tasks(&q).await });
                Ok(None)
            }
            "list_jobs" => {
                let p = need(&a, "process_instance_id")?.to_string();
                engine(app, reply.clone(), move |c| async move { c.jobs(&[("processInstanceId", p)]).await });
                Ok(None)
            }
            "list_external_tasks" => {
                let mut q: Vec<(&'static str, String)> = vec![];
                if let Some(p) = arg_str(&a, "process_instance_id") {
                    q.push(("processInstanceId", p.to_string()));
                }
                if let Some(p) = arg_str(&a, "topic") {
                    q.push(("topicName", p.to_string()));
                }
                q.push(("maxResults", arg_i(&a, "limit").unwrap_or(200).to_string()));
                engine(app, reply.clone(), move |c| async move { c.external_tasks(&q).await });
                Ok(None)
            }
            "list_batches" => {
                engine(app, reply.clone(), move |c| async move {
                    let running = c.batches().await?;
                    let hist = c.history_batches().await.unwrap_or_default();
                    Ok(json!({"running": running, "history": hist}))
                });
                Ok(None)
            }
            // ----- engine: write -----
            "set_variable" => {
                let id = need(&a, "id")?.to_string();
                let name = need(&a, "name")?.to_string();
                let val = a.get("value").cloned().ok_or("missing value")?;
                let vv = variable_value(&val, arg_str(&a, "type"));
                engine(app, reply.clone(), move |c| async move {
                    c.put_process_variable(&id, &name, &vv).await?;
                    Ok(json!({"ok": true, "name": name, "sent": vv}))
                });
                Ok(None)
            }
            "delete_variable" => {
                confirmed(&a)?;
                let id = need(&a, "id")?.to_string();
                let name = need(&a, "name")?.to_string();
                engine(app, reply.clone(), move |c| async move {
                    c.delete_process_variable(&id, &name).await?;
                    Ok(json!({"ok": true}))
                });
                Ok(None)
            }
            "start_instance" => {
                let key = need(&a, "key")?.to_string();
                let mut body = json!({"variables": variables_map(a.get("variables"))});
                if let Some(bk) = arg_str(&a, "business_key") {
                    body["businessKey"] = json!(bk);
                }
                if let Some(sb) = a.get("start_before").and_then(|v| v.as_array()) {
                    body["startInstructions"] = Value::Array(sb.iter().filter_map(|x| x.as_str()).map(|id| json!({"type": "startBeforeActivity", "activityId": id})).collect());
                }
                engine(app, reply.clone(), move |c| async move {
                    let d = c.process_definition_by_key(&key).await?;
                    c.start_instance(&d.id, &body).await
                });
                Ok(None)
            }
            "suspend_instance" => {
                let id = need(&a, "id")?.to_string();
                let s = arg_bool(&a, "suspended").ok_or("missing suspended")?;
                engine(app, reply.clone(), move |c| async move {
                    c.suspend_instance(&id, s).await?;
                    Ok(json!({"ok": true, "suspended": s}))
                });
                Ok(None)
            }
            "delete_instance" => {
                confirmed(&a)?;
                let id = need(&a, "id")?.to_string();
                let reason = arg_str(&a, "reason").unwrap_or("deleted via Flightdeck MCP").to_string();
                let skip = arg_bool(&a, "skip_custom_listeners").unwrap_or(false);
                engine(app, reply.clone(), move |c| async move {
                    c.delete_instance(&id, Some(&reason), skip, false, false).await?;
                    Ok(json!({"ok": true}))
                });
                Ok(None)
            }
            "modify_instance" => {
                confirmed(&a)?;
                let id = need(&a, "id")?.to_string();
                let instructions: Vec<camunda::ModificationInstruction> = serde_json::from_value(a.get("instructions").cloned().unwrap_or(json!([]))).map_err(|e| format!("bad instructions: {e}"))?;
                if instructions.is_empty() {
                    return Err("instructions must not be empty".into());
                }
                let req = camunda::ModificationRequest { instructions, skip_custom_listeners: arg_bool(&a, "skip_custom_listeners"), skip_io_mappings: arg_bool(&a, "skip_io_mappings"), annotation: arg_str(&a, "annotation").map(String::from) };
                engine(app, reply.clone(), move |c| async move {
                    c.modify_instance(&id, &req, false).await?;
                    Ok(json!({"ok": true}))
                });
                Ok(None)
            }
            "restart_instance" => {
                confirmed(&a)?;
                let id = need(&a, "id")?.to_string();
                let mut instructions = vec![];
                for (k, mk) in [("start_before", camunda::ModificationInstruction::start_before as fn(&str) -> camunda::ModificationInstruction), ("start_after", camunda::ModificationInstruction::start_after)] {
                    if let Some(arr) = a.get(k).and_then(|v| v.as_array()) {
                        for x in arr.iter().filter_map(|x| x.as_str()) {
                            instructions.push(mk(x));
                        }
                    }
                }
                if instructions.is_empty() {
                    return Err("give start_before and/or start_after activity ids".into());
                }
                engine(app, reply.clone(), move |c| async move {
                    let h = c.history_process_instance(&id).await?;
                    let req = camunda::RestartRequest { process_instance_ids: vec![id.clone()], instructions, initial_variables: Some(false), skip_custom_listeners: None, skip_io_mappings: None, without_business_key: Some(false) };
                    c.restart_instances(&h.process_definition_id, &req, false).await?;
                    Ok(json!({"ok": true}))
                });
                Ok(None)
            }
            "retry_job" => {
                let j = need(&a, "job_id")?.to_string();
                let n = arg_i(&a, "retries").unwrap_or(1);
                engine(app, reply.clone(), move |c| async move {
                    c.set_job_retries(&j, n).await?;
                    Ok(json!({"ok": true, "retries": n}))
                });
                Ok(None)
            }
            "execute_job" => {
                let j = need(&a, "job_id")?.to_string();
                engine(app, reply.clone(), move |c| async move {
                    c.execute_job(&j).await?;
                    Ok(json!({"ok": true}))
                });
                Ok(None)
            }
            "set_job_duedate" => {
                let j = need(&a, "job_id")?.to_string();
                let due = crate::query::date_value(need(&a, "due")?).ok_or("unrecognised date")?;
                engine(app, reply.clone(), move |c| async move {
                    c.set_job_duedate(&j, Some(&due)).await?;
                    Ok(json!({"ok": true, "due": due}))
                });
                Ok(None)
            }
            "retry_external_task" => {
                let e = need(&a, "external_task_id")?.to_string();
                let n = arg_i(&a, "retries").unwrap_or(1);
                engine(app, reply.clone(), move |c| async move {
                    c.set_external_task_retries(&e, n).await?;
                    Ok(json!({"ok": true}))
                });
                Ok(None)
            }
            "unlock_external_task" => {
                let e = need(&a, "external_task_id")?.to_string();
                engine(app, reply.clone(), move |c| async move {
                    c.unlock_external_task(&e).await?;
                    Ok(json!({"ok": true}))
                });
                Ok(None)
            }
            "set_task_assignee" => {
                let t = need(&a, "task_id")?.to_string();
                let u = arg_str(&a, "user").map(String::from);
                engine(app, reply.clone(), move |c| async move {
                    c.set_assignee(&t, u.as_deref()).await?;
                    Ok(json!({"ok": true}))
                });
                Ok(None)
            }
            "complete_task" => {
                confirmed(&a)?;
                let t = need(&a, "task_id")?.to_string();
                let vars = Value::Object(variables_map(a.get("variables")));
                engine(app, reply.clone(), move |c| async move {
                    c.complete_task(&t, &vars).await?;
                    Ok(json!({"ok": true}))
                });
                Ok(None)
            }
            "correlate_message" => {
                let mut body = json!({"messageName": need(&a, "message_name")?, "resultEnabled": true, "processVariables": variables_map(a.get("variables"))});
                if let Some(bk) = arg_str(&a, "business_key") {
                    body["businessKey"] = json!(bk);
                }
                if let Some(p) = arg_str(&a, "process_instance_id") {
                    body["processInstanceId"] = json!(p);
                }
                if arg_bool(&a, "all") == Some(true) {
                    body["all"] = json!(true);
                }
                engine(app, reply.clone(), move |c| async move { c.correlate_message(&body).await });
                Ok(None)
            }
            "resolve_incident" => {
                confirmed(&a)?;
                let i = need(&a, "incident_id")?.to_string();
                engine(app, reply.clone(), move |c| async move {
                    c.resolve_incident(&i).await?;
                    Ok(json!({"ok": true}))
                });
                Ok(None)
            }
            "annotate_incident" => {
                let i = need(&a, "incident_id")?.to_string();
                let t = need(&a, "annotation")?.to_string();
                engine(app, reply.clone(), move |c| async move {
                    c.set_incident_annotation(&i, &t).await?;
                    Ok(json!({"ok": true}))
                });
                Ok(None)
            }
            "suspend_definition" => {
                confirmed(&a)?;
                let d = need(&a, "definition_id")?.to_string();
                let s = arg_bool(&a, "suspended").ok_or("missing suspended")?;
                engine(app, reply.clone(), move |c| async move {
                    c.suspend_definition(&d, s, true).await?;
                    Ok(json!({"ok": true}))
                });
                Ok(None)
            }
            "batch" => {
                confirmed(&a)?;
                let op = need(&a, "operation")?.to_string();
                let ids: Vec<String> = a.get("instance_ids").and_then(|v| v.as_array()).map(|v| v.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
                if ids.is_empty() {
                    return Err("instance_ids is empty".into());
                }
                let vars = Value::Object(variables_map(a.get("variables")));
                let reason = arg_str(&a, "reason").unwrap_or("batch via Flightdeck MCP").to_string();
                engine(app, reply.clone(), move |c| async move {
                    match op.as_str() {
                        "suspend" => c.suspend_async(&ids, true).await,
                        "activate" => c.suspend_async(&ids, false).await,
                        "retry_jobs" => c.set_retries_async(&ids, 1).await,
                        "delete" => c.delete_async(&ids, &reason, false, false).await,
                        "set_variables" => c.set_variables_async(&ids, &vars).await,
                        other => Err(camunda::ApiError::Decode(format!("unknown operation {other}"))),
                    }
                });
                Ok(None)
            }
            // ----- UI -----
            "ui_state" => {
                let tabs: Vec<Value> = app.tabs.iter().enumerate().map(|(i, t)| json!({"index": i, "title": t.place().title(), "active": i == app.active, "can_back": t.can_back(), "can_forward": t.can_forward()})).collect();
                let toasts: Vec<Value> = app.toasts.items.iter().rev().take(5).map(|t| json!({"kind": format!("{:?}", t.kind), "text": t.text})).collect();
                Ok(Some(json!({
                    "connected": app.net.is_some(), "tabs": tabs, "place": place_json(app), "dock_visible": app.dock_visible,
                    "pending_confirmation": app.confirm.as_ref().map(|c| json!({"title": c.title, "body": c.body, "confirm_label": c.confirm_label})),
                    "palette_open": app.palette.open, "help_open": app.help_open, "theme": if app.tokens.dark { "dark" } else { "light" }, "theme_mode": format!("{:?}", app.theme_mode()), "auto_refresh_paused": app.paused,
                    "popup_open": app.ctx.as_ref().map(egui::Popup::is_any_open), "window_focused": app.window_focused, "frame": app.frame_number(),
                    "toasts": toasts, "window": app.ctx.as_ref().map(|c| { let r = c.content_rect(); json!({"width": r.width(), "height": r.height()}) }),
                })))
            }
            "ui_navigate" => {
                let to = need(&a, "to")?;
                let mode = if arg_bool(&a, "new_tab").unwrap_or(false) { NavMode::NewTab } else { NavMode::Here };
                let nav = if to == "definitions" {
                    Nav::Definitions
                } else if let Some(d) = to.strip_prefix("def:") {
                    if d.contains(':') {
                        Nav::Definition { id: d.to_string() }
                    } else {
                        Nav::DefinitionKey { key: d.to_string() }
                    }
                } else if let Some(i) = to.strip_prefix("instance:") {
                    Nav::Instance { id: i.to_string() }
                } else if let Some(q) = to.strip_prefix("search:") {
                    Nav::Search { query: q.to_string() }
                } else if to == "incidents" {
                    Nav::Incidents
                } else if to == "batches" {
                    Nav::Batches
                } else if to == "settings" {
                    Nav::Settings
                } else if crate::util::is_uuid(to) {
                    Nav::Instance { id: to.to_string() }
                } else {
                    return Err("unknown target; use definitions | def:<key|id> | instance:<id> | search:<query> | incidents | batches | settings".into());
                };
                app.navigate(nav, mode);
                Ok(Some(json!({"ok": true, "hint": "call ui_state after a moment to see the loaded place"})))
            }
            "ui_tabs" => {
                match need(&a, "action")? {
                    "list" => {}
                    "activate" => {
                        let i = arg_i(&a, "index").ok_or("index")? as usize;
                        if i < app.tabs.len() {
                            app.active = i;
                        }
                    }
                    "close" => {
                        let i = arg_i(&a, "index").map(|i| i as usize).unwrap_or(app.active);
                        app.close_tab(i);
                    }
                    "new" => app.navigate(Nav::Definitions, NavMode::NewTab),
                    "back" => crate::commands::Cmd::Back.run(app),
                    "forward" => crate::commands::Cmd::Forward.run(app),
                    other => return Err(format!("unknown action {other}")),
                }
                Ok(Some(json!({"tabs": app.tabs.iter().enumerate().map(|(i, t)| json!({"index": i, "title": t.place().title(), "active": i == app.active})).collect::<Vec<_>>()})))
            }
            "ui_select" => {
                let id = arg_str(&a, "element_id").map(String::from);
                let fit = arg_bool(&a, "fit").unwrap_or(true);
                let r = with_diagram(app, |view, m| match id {
                    None => {
                        view.select(None);
                        Ok(json!({"ok": true, "selection": null}))
                    }
                    Some(id) => match m.lookup(&id) {
                        Some(r) => {
                            view.select(Some(r));
                            if fit {
                                view.fit_refs(m, std::iter::once(r));
                            }
                            Ok(json!({"ok": true, "selection": id}))
                        }
                        None => Err(format!("no element {id}")),
                    },
                })??;
                Ok(Some(r))
            }
            "ui_fit" => {
                let what = need(&a, "what")?.to_string();
                let eid = arg_str(&a, "element_id").map(String::from);
                with_diagram(app, |view, m| match what.as_str() {
                    "all" => {
                        view.fit_all();
                        Ok(())
                    }
                    "selection" => {
                        if let Some(r) = view.selection {
                            view.fit_refs(m, std::iter::once(r));
                        }
                        Ok(())
                    }
                    "tokens" => {
                        view.fit_tokens_marker = true;
                        Ok(())
                    }
                    "element" => {
                        let id = eid.ok_or("element_id required")?;
                        let i = m.element_idx(&id).ok_or(format!("no element {id}"))?;
                        view.fit_element(m, i);
                        Ok(())
                    }
                    other => Err(format!("unknown fit target {other}")),
                })??;
                Ok(Some(json!({"ok": true})))
            }
            "ui_zoom" => {
                let level = arg_f(&a, "level");
                let factor = arg_f(&a, "factor");
                let z = with_diagram(app, |view, _| {
                    if let Some(l) = level {
                        view.camera.target.zoom = (l as f32).clamp(crate::viewport::camera::MIN_ZOOM, crate::viewport::camera::MAX_ZOOM);
                    }
                    if let Some(f) = factor {
                        view.camera.target.zoom = (view.camera.target.zoom * f as f32).clamp(crate::viewport::camera::MIN_ZOOM, crate::viewport::camera::MAX_ZOOM);
                    }
                    view.camera.target.zoom
                })?;
                Ok(Some(json!({"zoom": z})))
            }
            "ui_pan" => {
                let dx = arg_f(&a, "dx").unwrap_or(0.0) as f32;
                let dy = arg_f(&a, "dy").unwrap_or(0.0) as f32;
                let c = with_diagram(app, |view, _| {
                    view.camera.pan_screen(egui::Vec2::new(-dx, -dy));
                    view.camera.target.center
                })?;
                Ok(Some(json!({"center": [c.x, c.y]})))
            }
            "ui_find" => {
                let q = arg_str(&a, "query").unwrap_or("").to_string();
                let flows = arg_bool(&a, "include_flows");
                let r = with_diagram(app, |view, m| {
                    if q.is_empty() {
                        view.close_find();
                        return json!({"ok": true, "closed": true});
                    }
                    view.open_find_with(&q);
                    if let Some(f) = flows {
                        view.find.include_flows = f;
                    }
                    let opts = bpmn::search::SearchOptions { case_sensitive: view.find.case_sensitive, categories: view.find.categories.clone(), include_flows: view.find.include_flows };
                    let res = bpmn::search::search(m, &q, &opts);
                    json!({"ok": true, "matches": res.iter().take(100).map(|r| match r.target { bpmn::Ref::Element(i) => json!({"element_id": m.elements[i].id, "name": m.elements[i].display_name(), "field": r.field}), bpmn::Ref::Flow(i) => json!({"flow_id": m.flows[i].id, "field": r.field}) }).collect::<Vec<_>>(), "count": res.len()})
                })?;
                Ok(Some(r))
            }
            "ui_dock" => {
                if let Some(v) = arg_bool(&a, "visible") {
                    app.dock_visible = v;
                }
                if let Some(t) = arg_str(&a, "tab") {
                    app.dock_visible = true;
                    let find = |names: &[&str]| -> Result<usize, String> {
                        if let Ok(i) = t.parse::<usize>() {
                            return Ok(i.min(names.len() - 1));
                        }
                        names.iter().position(|n| n.eq_ignore_ascii_case(t) || n.to_lowercase().starts_with(&t.to_lowercase())).ok_or(format!("no dock tab {t}; available: {}", names.join(", ")))
                    };
                    match app.tabs.get_mut(app.active).map(|t| t.place_mut()) {
                        Some(Place::Definition(p)) => p.dock_tab = find(&crate::places::definition::DOCK_TABS)?,
                        Some(Place::Instance(p)) => p.dock_tab = find(&crate::places::instance::DOCK_TABS)?,
                        Some(Place::File(p)) => p.dock_tab = find(&["Inspector", "Matches"])?,
                        _ => return Err("this place has no dock tabs".into()),
                    }
                }
                Ok(Some(json!({"ok": true, "dock_visible": app.dock_visible})))
            }
            "ui_filter" => {
                let text = need(&a, "text")?.to_string();
                match app.tabs.get_mut(app.active).map(|t| t.place_mut()) {
                    Some(Place::Definitions(p)) => p.filter = text,
                    Some(Place::Definition(p)) => {
                        p.instance_filter = text;
                        p.dock_tab = 1;
                        app.dock_visible = true;
                    }
                    Some(Place::Instance(p)) => {
                        p.var_filter = text;
                        p.dock_tab = 2;
                        app.dock_visible = true;
                    }
                    Some(Place::Search(p)) => {
                        p.query = text;
                        p.run_query();
                    }
                    Some(Place::Incidents(p)) => p.filter = text,
                    _ => return Err("this place has no filter".into()),
                }
                Ok(Some(json!({"ok": true})))
            }
            "ui_commands" => Ok(Some(json!(crate::commands::Cmd::ALL.iter().map(|c| json!({"name": format!("{:?}", c), "label": c.info().name, "keys": c.info().keys_label, "enabled": c.enabled(app)})).collect::<Vec<_>>()))),
            "ui_command" => {
                let n = need(&a, "name")?.to_lowercase().replace(['_', '-', ' '], "");
                let cmd = crate::commands::Cmd::ALL.iter().copied().find(|c| format!("{c:?}").to_lowercase() == n || c.info().name.to_lowercase().replace([' ', '/', '-'], "").contains(&n)).ok_or("unknown command; see ui_commands")?;
                if !cmd.enabled(app) {
                    return Err("command is not enabled right now".into());
                }
                cmd.run(app);
                Ok(Some(json!({"ok": true, "ran": format!("{cmd:?}")})))
            }
            "ui_key" => {
                let k = need(&a, "key")?;
                let key = parse_key(k).ok_or(format!("unknown key {k}"))?;
                let mods = egui::Modifiers { ctrl: arg_bool(&a, "ctrl").unwrap_or(false), shift: arg_bool(&a, "shift").unwrap_or(false), alt: arg_bool(&a, "alt").unwrap_or(false), command: arg_bool(&a, "ctrl").unwrap_or(false), mac_cmd: false };
                let n = arg_i(&a, "repeat").unwrap_or(1).clamp(1, 50);
                for _ in 0..n {
                    app.synthetic_events.push(egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: mods });
                    app.synthetic_events.push(egui::Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers: mods });
                }
                Ok(Some(json!({"ok": true, "note": "applied on the next frames; keys that read raw key-down state (continuous panning) are not simulated, use ui_pan"})))
            }
            "ui_type" => {
                let t = need(&a, "text")?.to_string();
                app.synthetic_events.push(egui::Event::Text(t));
                Ok(Some(json!({"ok": true})))
            }
            "ui_hit_test" => {
                let x = arg_f(&a, "x").ok_or("x")? as f32;
                let y = arg_f(&a, "y").ok_or("y")? as f32;
                let vp = app.last_diagram_viewport;
                let r = with_diagram(app, |view, m| {
                    let w = view.camera.screen_to_world(vp, egui::Pos2::new(x, y));
                    match crate::viewport::hit::hit_test(m, w, 6.0 / view.camera.zoom()) {
                        Some(bpmn::Ref::Element(i)) => json!({"element_id": m.elements[i].id, "name": m.elements[i].display_name(), "type": m.elements[i].kind.label(), "world": [w.x, w.y]}),
                        Some(bpmn::Ref::Flow(i)) => json!({"flow_id": m.flows[i].id, "world": [w.x, w.y]}),
                        None => json!({"empty": true, "world": [w.x, w.y], "inside_diagram_viewport": vp.contains(egui::Pos2::new(x, y))}),
                    }
                })?;
                Ok(Some(r))
            }
            "ui_screenshot" => {
                let Some(ctx) = app.ctx.clone() else { return Err("no context".into()) };
                let path = arg_str(&a, "path").map(std::path::PathBuf::from);
                let max_w = arg_i(&a, "max_width").unwrap_or(1280).max(200) as u32;
                let reply2 = reply.clone();
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::ScreenshotCallback::new(move |img: std::sync::Arc<egui::ColorImage>| {
                    let [w, h] = img.size;
                    let Some(mut im) = image::RgbaImage::from_raw(w as u32, h as u32, img.as_raw().to_vec()) else {
                        reply2.err("bad image buffer");
                        return;
                    };
                    if im.width() > max_w {
                        let nh = (im.height() as f32 * max_w as f32 / im.width() as f32) as u32;
                        im = image::imageops::resize(&im, max_w, nh.max(1), image::imageops::FilterType::Triangle);
                    }
                    let mut buf = std::io::Cursor::new(Vec::new());
                    if let Err(e) = im.write_to(&mut buf, image::ImageFormat::Png) {
                        reply2.err(format!("png encode: {e}"));
                        return;
                    }
                    if let Some(p) = &path {
                        let _ = std::fs::write(p, buf.get_ref());
                    }
                    let b64 = base64::engine::general_purpose::STANDARD.encode(buf.get_ref());
                    reply2.ok(json!({"_png_base64": b64, "width": im.width(), "height": im.height(), "saved_to": path}));
                })));
                ctx.request_repaint();
                Ok(None)
            }
            "ui_confirm" => {
                let accept = arg_bool(&a, "accept").ok_or("accept")?;
                match app.confirm.take() {
                    None => Err("no pending confirmation".into()),
                    Some(mut c) => {
                        if accept {
                            if let Some(act) = c.action.take() {
                                act(app);
                            }
                        }
                        Ok(Some(json!({"ok": true, "accepted": accept, "title": c.title})))
                    }
                }
            }
            "ui_modification_queue" => {
                let action = need(&a, "action")?.to_string();
                let ins: Option<camunda::ModificationInstruction> = a.get("instruction").cloned().map(serde_json::from_value).transpose().map_err(|e| format!("bad instruction: {e}"))?;
                let Some(Place::Instance(p)) = app.tabs.get_mut(app.active).map(|t| t.place_mut()) else { return Err("open an instance first".into()) };
                match action.as_str() {
                    "list" => {}
                    "add" => p.modification.instructions.push(ins.ok_or("instruction required")?),
                    "clear" => p.modification.instructions.clear(),
                    "apply" => {
                        p.dock_tab = 9;
                        app.dock_visible = true;
                        return Ok(Some(json!({"hint": "use modify_instance with the same instructions and confirm:true to apply without the dialog, or press the Apply button in the Modify tab (ui_key Enter after ui_dock Modify)"})));
                    }
                    other => return Err(format!("unknown action {other}")),
                }
                Ok(Some(json!({"instructions": p.modification.instructions})))
            }
            "ui_settings" => {
                if let Some(Value::Object(set)) = a.get("set") {
                    for (k, v) in set {
                        match k.as_str() {
                            "theme" => {
                                app.settings.theme = match v.as_str().unwrap_or("system") {
                                    "dark" => crate::theme::ThemeMode::Dark,
                                    "light" => crate::theme::ThemeMode::Light,
                                    _ => crate::theme::ThemeMode::System,
                                };
                                app.frame_theme_dirty = true;
                            }
                            "density" => {
                                app.settings.density = v.as_f64().unwrap_or(1.0) as f32;
                                app.frame_theme_dirty = true;
                            }
                            "wheel_zooms" => app.settings.wheel_zooms = v.as_bool().unwrap_or(true),
                            "inertia" => app.settings.inertia = v.as_bool().unwrap_or(true),
                            "pointer_wrap" => {
                                app.settings.pointer_wrap = match v.as_str().unwrap_or("auto") {
                                    "off" => crate::viewport::WrapMode::Off,
                                    _ => crate::viewport::WrapMode::Auto,
                                }
                            }
                            "refresh_instance_secs" => app.settings.refresh_instance_secs = v.as_u64().unwrap_or(5),
                            "refresh_list_secs" => app.settings.refresh_list_secs = v.as_u64().unwrap_or(10),
                            "page_size" => app.settings.page_size = v.as_u64().unwrap_or(100) as usize,
                            "paused" => app.paused = v.as_bool().unwrap_or(false),
                            other => return Err(format!("unknown setting {other}")),
                        }
                    }
                    app.settings_dirty = true;
                }
                Ok(Some(json!({"theme": format!("{:?}", app.settings.theme), "density": app.settings.density, "wheel_zooms": app.settings.wheel_zooms, "inertia": app.settings.inertia, "pointer_wrap": format!("{:?}", app.settings.pointer_wrap), "refresh_instance_secs": app.settings.refresh_instance_secs, "refresh_list_secs": app.settings.refresh_list_secs, "page_size": app.settings.page_size, "paused": app.paused, "mcp_port": app.settings.mcp_port})))
            }
            "ui_connect" => {
                if arg_bool(&a, "disconnect") == Some(true) {
                    app.disconnect();
                    return Ok(Some(json!({"ok": true, "connected": false})));
                }
                let url = need(&a, "url")?;
                let p = crate::settings::EngineProfile { name: "mcp".into(), url: camunda::normalize_base_url(url), user: arg_str(&a, "user").unwrap_or("").into(), password: arg_str(&a, "password").unwrap_or("").into(), ..Default::default() };
                app.connect.profile = p.clone();
                app.connect.connecting = true;
                app.disconnect();
                app.connect_to(p);
                Ok(Some(json!({"ok": true, "hint": "call engine_status to see the result"})))
            }
            other => Err(format!("unknown tool {other}")),
        }
    })();
    match r {
        Ok(Some(v)) => reply.ok(v),
        Ok(None) => {} // answered asynchronously
        Err(e) if e == RETRY => retry_later(app, tool, a, reply, "diagram or instance still loading"),
        Err(e) => reply.err(e),
    }
}

const RETRY: &str = "__retry__";
