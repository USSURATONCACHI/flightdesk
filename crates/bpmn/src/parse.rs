//! BPMN 2.0 XML → `Model`. Matches elements by local name so files with a default namespace,
//! `bpmn:` or `bpmn2:` prefixes all work.

use crate::model::*;
use roxmltree::{Document, Node};
use std::collections::HashMap;
use std::sync::Arc;

const CAMUNDA_NS: &str = "http://camunda.org/schema/1.0/bpmn";

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("invalid XML: {0}")]
    Xml(#[from] roxmltree::Error),
    #[error("not a BPMN file: root element is <{0}>, expected <definitions>")]
    NotBpmn(String),
}

pub fn parse(source: &str) -> Result<Model, ParseError> {
    let doc = Document::parse_with_options(
        source,
        roxmltree::ParsingOptions { allow_dtd: true, nodes_limit: u32::MAX },
    )?;
    let root = doc.root_element();
    if root.tag_name().name() != "definitions" {
        return Err(ParseError::NotBpmn(root.tag_name().name().to_string()));
    }
    let mut p = Parser {
        source,
        model: Model {
            source: Arc::from(source),
            definitions_attrs: attrs_of(&root),
            processes: vec![],
            elements: vec![],
            flows: vec![],
            messages: HashMap::new(),
            errors: HashMap::new(),
            signals: HashMap::new(),
            escalations: HashMap::new(),
            by_id: HashMap::new(),
            bounds: Rect::EMPTY,
            draw_order: vec![],
            reading_order: vec![],
            warnings: vec![],
        },
        lane_members: vec![],
        element_parent_id: vec![],
    };
    p.parse_root(&root);
    p.finish();
    Ok(p.model)
}

struct Parser<'a> {
    #[allow(dead_code)]
    source: &'a str,
    model: Model,
    /// (lane element index, flowNodeRef id)
    lane_members: Vec<(ElementIdx, String)>,
    /// parent element id for each element (same indexing as model.elements).
    element_parent_id: Vec<Option<String>>,
}

fn attrs_of(node: &Node) -> Vec<(String, String)> {
    node.attributes()
        .map(|a| {
            let name = match a.namespace() {
                Some(ns) => {
                    let prefix = if ns == CAMUNDA_NS {
                        Some("camunda")
                    } else {
                        node.lookup_prefix(ns)
                    };
                    match prefix {
                        Some(p) if !p.is_empty() => format!("{p}:{}", a.name()),
                        _ => a.name().to_string(),
                    }
                }
                None => a.name().to_string(),
            };
            (name, a.value().to_string())
        })
        .collect()
}

fn attr<'a>(node: &'a Node, name: &str) -> Option<&'a str> {
    node.attributes().find(|a| a.name() == name && a.namespace().is_none()).map(|a| a.value())
}

fn camunda_attr<'a>(node: &'a Node, name: &str) -> Option<&'a str> {
    node.attributes()
        .find(|a| a.name() == name && a.namespace() == Some(CAMUNDA_NS))
        .map(|a| a.value())
}

fn child<'a, 'b>(node: &Node<'a, 'b>, name: &str) -> Option<Node<'a, 'b>> {
    node.children().find(|c| c.is_element() && c.tag_name().name() == name)
}

fn children<'a, 'b, 'n>(node: &Node<'a, 'b>, name: &'n str) -> impl Iterator<Item = Node<'a, 'b>> + 'n where 'a: 'n, 'b: 'n {
    node.children().filter(move |c| c.is_element() && c.tag_name().name() == name)
}

fn text_of(node: &Node) -> Option<String> {
    let t: String = node.children().filter_map(|c| c.text()).collect();
    let t = t.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

fn child_text(node: &Node, name: &str) -> Option<String> {
    child(node, name).and_then(|c| text_of(&c))
}

fn documentation(node: &Node) -> Option<String> {
    let docs: Vec<String> = children(node, "documentation").filter_map(|d| text_of(&d)).collect();
    if docs.is_empty() {
        None
    } else {
        Some(docs.join("\n\n"))
    }
}

fn expression(node: &Node) -> Expression {
    Expression { text: text_of(node).unwrap_or_default(), language: attr(node, "language").map(String::from) }
}

fn parse_bool(s: Option<&str>, default: bool) -> bool {
    match s {
        Some(v) => v.eq_ignore_ascii_case("true"),
        None => default,
    }
}

fn parse_param_value(node: &Node) -> ParamValue {
    // A parameter is either plain text or exactly one of camunda:script / camunda:list / camunda:map.
    if let Some(c) = node.children().find(|c| c.is_element()) {
        match c.tag_name().name() {
            "script" => ParamValue::Script {
                format: attr(&c, "scriptFormat").map(String::from),
                resource: attr(&c, "resource").map(String::from),
                source: text_of(&c).unwrap_or_default(),
            },
            "list" => ParamValue::List(children(&c, "value").map(|v| parse_param_value(&v)).collect()),
            "map" => ParamValue::Map(
                children(&c, "entry")
                    .map(|e| (attr(&e, "key").unwrap_or("").to_string(), parse_param_value(&e)))
                    .collect(),
            ),
            _ => ParamValue::Text(text_of(node).unwrap_or_default()),
        }
    } else {
        ParamValue::Text(text_of(node).unwrap_or_default())
    }
}

fn parse_fields(node: &Node) -> Vec<FieldInjection> {
    children(node, "field")
        .map(|f| FieldInjection {
            name: attr(&f, "name").unwrap_or("").to_string(),
            string_value: attr(&f, "stringValue").map(String::from).or_else(|| child_text(&f, "string")),
            expression: attr(&f, "expression").map(String::from).or_else(|| child_text(&f, "expression")),
        })
        .collect()
}

fn parse_extensions(source: &str, node: &Node) -> Extensions {
    let mut ext = Extensions::default();
    let Some(ee) = child(node, "extensionElements") else { return ext };
    for c in ee.children().filter(|c| c.is_element()) {
        match c.tag_name().name() {
            "inputOutput" => {
                for p in c.children().filter(|c| c.is_element()) {
                    let param = IoParam { name: attr(&p, "name").unwrap_or("").to_string(), value: parse_param_value(&p) };
                    match p.tag_name().name() {
                        "inputParameter" => ext.inputs.push(param),
                        "outputParameter" => ext.outputs.push(param),
                        _ => {}
                    }
                }
            }
            kind @ ("executionListener" | "taskListener") => {
                ext.listeners.push(Listener {
                    kind: kind.trim_end_matches("Listener").to_string(),
                    event: attr(&c, "event").map(String::from),
                    class: attr(&c, "class").map(String::from),
                    expression: attr(&c, "expression").map(String::from),
                    delegate_expression: attr(&c, "delegateExpression").map(String::from),
                    script: child(&c, "script").map(|s| (attr(&s, "scriptFormat").map(String::from), text_of(&s).unwrap_or_default())),
                    fields: parse_fields(&c),
                });
            }
            "properties" => {
                for p in children(&c, "property") {
                    ext.properties.push((
                        attr(&p, "name").or(attr(&p, "id")).unwrap_or("").to_string(),
                        attr(&p, "value").map(String::from).or_else(|| text_of(&p)).unwrap_or_default(),
                    ));
                }
            }
            "field" => {
                ext.fields.push(FieldInjection {
                    name: attr(&c, "name").unwrap_or("").to_string(),
                    string_value: attr(&c, "stringValue").map(String::from).or_else(|| child_text(&c, "string")),
                    expression: attr(&c, "expression").map(String::from).or_else(|| child_text(&c, "expression")),
                });
            }
            "formData" => {
                for f in children(&c, "formField") {
                    ext.form_fields.push(FormField {
                        id: attr(&f, "id").unwrap_or("").to_string(),
                        label: attr(&f, "label").map(String::from),
                        field_type: attr(&f, "type").map(String::from),
                        default_value: attr(&f, "defaultValue").map(String::from),
                        validation: child(&f, "validation")
                            .map(|v| children(&v, "constraint").map(|k| (attr(&k, "name").unwrap_or("").to_string(), attr(&k, "config").map(String::from))).collect())
                            .unwrap_or_default(),
                        properties: child(&f, "properties")
                            .map(|v| children(&v, "property").map(|k| (attr(&k, "id").unwrap_or("").to_string(), attr(&k, "value").unwrap_or("").to_string())).collect())
                            .unwrap_or_default(),
                        values: children(&f, "value").map(|k| (attr(&k, "id").unwrap_or("").to_string(), attr(&k, "name").unwrap_or("").to_string())).collect(),
                    });
                }
            }
            "failedJobRetryTimeCycle" => ext.failed_job_retry_time_cycle = text_of(&c),
            dir @ ("in" | "out") => ext.variable_mappings.push(VariableMapping {
                direction: dir.to_string(),
                source: attr(&c, "source").map(String::from),
                source_expression: attr(&c, "sourceExpression").map(String::from),
                target: attr(&c, "target").map(String::from),
                variables: attr(&c, "variables").map(String::from),
                business_key: attr(&c, "businessKey").map(String::from),
                local: parse_bool(attr(&c, "local"), false),
            }),
            _ => {
                let r = c.range();
                ext.other.push(source[r].to_string());
            }
        }
    }
    ext
}

fn parse_event_defs(node: &Node) -> Vec<EventDef> {
    let mut out = vec![];
    for c in node.children().filter(|c| c.is_element()) {
        let d = match c.tag_name().name() {
            "timerEventDefinition" => EventDef::Timer {
                time_date: child_text(&c, "timeDate"),
                time_duration: child_text(&c, "timeDuration"),
                time_cycle: child_text(&c, "timeCycle"),
            },
            "messageEventDefinition" => EventDef::Message { message_ref: attr(&c, "messageRef").map(String::from), name: None },
            "signalEventDefinition" => EventDef::Signal { signal_ref: attr(&c, "signalRef").map(String::from), name: None },
            "errorEventDefinition" => EventDef::Error {
                error_ref: attr(&c, "errorRef").map(String::from),
                name: None,
                code: None,
                code_variable: camunda_attr(&c, "errorCodeVariable").map(String::from),
                message_variable: camunda_attr(&c, "errorMessageVariable").map(String::from),
            },
            "escalationEventDefinition" => EventDef::Escalation {
                escalation_ref: attr(&c, "escalationRef").map(String::from),
                name: None,
                code: None,
                code_variable: camunda_attr(&c, "escalationCodeVariable").map(String::from),
            },
            "conditionalEventDefinition" => EventDef::Conditional {
                condition: child(&c, "condition").map(|e| expression(&e)),
                variable_name: camunda_attr(&c, "variableName").map(String::from),
                variable_events: camunda_attr(&c, "variableEvents").map(String::from),
            },
            "linkEventDefinition" => EventDef::Link { name: attr(&c, "name").map(String::from) },
            "compensateEventDefinition" => EventDef::Compensate {
                activity_ref: attr(&c, "activityRef").map(String::from),
                wait_for_completion: attr(&c, "waitForCompletion").map(|v| v == "true"),
            },
            "terminateEventDefinition" => EventDef::Terminate,
            "cancelEventDefinition" => EventDef::Cancel,
            _ => continue,
        };
        out.push(d);
    }
    out
}

impl<'a> Parser<'a> {
    fn parse_root(&mut self, root: &Node) {
        for c in root.children().filter(|c| c.is_element()) {
            match c.tag_name().name() {
                "message" => {
                    self.model.messages.insert(attr(&c, "id").unwrap_or("").to_string(), NamedDef { id: attr(&c, "id").unwrap_or("").into(), name: attr(&c, "name").map(String::from), code: None });
                }
                "error" => {
                    self.model.errors.insert(attr(&c, "id").unwrap_or("").to_string(), NamedDef { id: attr(&c, "id").unwrap_or("").into(), name: attr(&c, "name").map(String::from), code: attr(&c, "errorCode").map(String::from) });
                }
                "signal" => {
                    self.model.signals.insert(attr(&c, "id").unwrap_or("").to_string(), NamedDef { id: attr(&c, "id").unwrap_or("").into(), name: attr(&c, "name").map(String::from), code: None });
                }
                "escalation" => {
                    self.model.escalations.insert(attr(&c, "id").unwrap_or("").to_string(), NamedDef { id: attr(&c, "id").unwrap_or("").into(), name: attr(&c, "name").map(String::from), code: attr(&c, "escalationCode").map(String::from) });
                }
                _ => {}
            }
        }
        for c in root.children().filter(|c| c.is_element()) {
            match c.tag_name().name() {
                "process" => self.parse_process(&c),
                "collaboration" => self.parse_collaboration(&c),
                _ => {}
            }
        }
        for c in root.children().filter(|c| c.is_element() && c.tag_name().name() == "BPMNDiagram") {
            self.parse_diagram(&c);
        }
    }

    fn parse_process(&mut self, node: &Node) {
        let id = attr(node, "id").unwrap_or("").to_string();
        self.model.processes.push(Process {
            id: id.clone(),
            name: attr(node, "name").map(String::from),
            is_executable: parse_bool(attr(node, "isExecutable"), false),
            attrs: attrs_of(node),
            documentation: documentation(node),
            ext: parse_extensions(self.source, node),
        });
        self.parse_flow_elements(node, &id, None, 2);
    }

    fn parse_collaboration(&mut self, node: &Node) {
        for c in node.children().filter(|c| c.is_element()) {
            match c.tag_name().name() {
                "participant" => {
                    let idx = self.push_element(&c, ElementKind::Participant, None, None, 0);
                    self.model.elements[idx].process_id = attr(&c, "processRef").map(String::from);
                }
                "messageFlow" => {
                    self.push_flow(&c, FlowKind::MessageFlow);
                }
                "association" => {
                    self.push_flow(&c, FlowKind::Association);
                }
                "textAnnotation" => {
                    let idx = self.push_element(&c, ElementKind::TextAnnotation, None, None, 2);
                    self.model.elements[idx].name = child_text(&c, "text");
                }
                "group" => {
                    self.push_element(&c, ElementKind::Group, None, None, 1);
                }
                _ => {}
            }
        }
    }

    fn parse_flow_elements(&mut self, node: &Node, process_id: &str, parent_id: Option<&str>, depth: u16) {
        for c in node.children().filter(|c| c.is_element()) {
            let tag = c.tag_name().name();
            match tag {
                "sequenceFlow" => {
                    self.push_flow(&c, FlowKind::SequenceFlow);
                }
                "association" => {
                    self.push_flow(&c, FlowKind::Association);
                }
                "laneSet" | "childLaneSet" => self.parse_lane_set(&c, process_id, parent_id),
                "textAnnotation" => {
                    let idx = self.push_element(&c, ElementKind::TextAnnotation, Some(process_id), parent_id, depth);
                    self.model.elements[idx].name = child_text(&c, "text");
                }
                "dataObject" | "message" | "error" | "signal" | "extensionElements" | "documentation" | "ioSpecification" | "property" => {}
                _ => {
                    let kind = ElementKind::from_tag(tag);
                    if matches!(kind, ElementKind::Unknown(_)) {
                        continue;
                    }
                    let idx = self.push_element(&c, kind.clone(), Some(process_id), parent_id, depth);
                    self.fill_node_details(idx, &c);
                    // data associations hang off activities
                    for da in c.children().filter(|c| c.is_element()) {
                        match da.tag_name().name() {
                            "dataInputAssociation" => {
                                let fidx = self.push_flow(&da, FlowKind::DataInputAssociation);
                                let f = &mut self.model.flows[fidx];
                                f.source_id = child_text(&da, "sourceRef");
                                f.target_id = Some(self.model.elements[idx].id.clone());
                            }
                            "dataOutputAssociation" => {
                                let fidx = self.push_flow(&da, FlowKind::DataOutputAssociation);
                                let f = &mut self.model.flows[fidx];
                                f.source_id = Some(self.model.elements[idx].id.clone());
                                f.target_id = child_text(&da, "targetRef");
                            }
                            _ => {}
                        }
                    }
                    if kind.is_container() {
                        let my_id = self.model.elements[idx].id.clone();
                        self.parse_flow_elements(&c, process_id, Some(&my_id), depth + 1);
                    }
                }
            }
        }
    }

    fn parse_lane_set(&mut self, node: &Node, process_id: &str, parent_id: Option<&str>) {
        for lane in children(node, "lane") {
            let idx = self.push_element(&lane, ElementKind::Lane, Some(process_id), parent_id, 1);
            for r in children(&lane, "flowNodeRef") {
                if let Some(t) = text_of(&r) {
                    self.lane_members.push((idx, t));
                }
            }
            if let Some(cls) = child(&lane, "childLaneSet") {
                let my_id = self.model.elements[idx].id.clone();
                self.parse_lane_set(&cls, process_id, Some(&my_id));
            }
        }
    }

    fn push_element(&mut self, node: &Node, kind: ElementKind, process_id: Option<&str>, parent_id: Option<&str>, depth: u16) -> ElementIdx {
        let e = Element {
            id: attr(node, "id").unwrap_or("").to_string(),
            name: attr(node, "name").map(String::from),
            kind,
            bounds: Rect::EMPTY,
            label_bounds: None,
            documentation: documentation(node),
            attrs: attrs_of(node),
            ext: parse_extensions(self.source, node),
            event_defs: vec![],
            attached_to: None,
            interrupting: true,
            default_flow: None,
            multi_instance: None,
            standard_loop: false,
            is_for_compensation: false,
            is_expanded: true,
            triggered_by_event: false,
            is_horizontal: true,
            marker_visible: true,
            script: None,
            process_id: process_id.map(String::from),
            parent: None,
            lane: None,
            depth,
            incoming: vec![],
            outgoing: vec![],
            boundary_events: vec![],
            xml_range: node.range(),
        };
        self.model.elements.push(e);
        self.element_parent_id.push(parent_id.map(String::from));
        self.model.elements.len() - 1
    }

    fn fill_node_details(&mut self, idx: ElementIdx, node: &Node) {
        let mut defs = parse_event_defs(node);
        for d in &mut defs {
            match d {
                EventDef::Message { message_ref: Some(r), name } => *name = self.model.messages.get(r).and_then(|m| m.name.clone()),
                EventDef::Signal { signal_ref: Some(r), name } => *name = self.model.signals.get(r).and_then(|m| m.name.clone()),
                EventDef::Error { error_ref: Some(r), name, code, .. } => {
                    if let Some(e) = self.model.errors.get(r) {
                        *name = e.name.clone();
                        *code = e.code.clone();
                    }
                }
                EventDef::Escalation { escalation_ref: Some(r), name, code, .. } => {
                    if let Some(e) = self.model.escalations.get(r) {
                        *name = e.name.clone();
                        *code = e.code.clone();
                    }
                }
                _ => {}
            }
        }
        let e = &mut self.model.elements[idx];
        e.event_defs = defs;
        e.attached_to = attr(node, "attachedToRef").map(String::from);
        e.interrupting = match e.kind {
            ElementKind::BoundaryEvent => parse_bool(attr(node, "cancelActivity"), true),
            ElementKind::StartEvent => parse_bool(attr(node, "isInterrupting"), true),
            _ => true,
        };
        e.default_flow = attr(node, "default").map(String::from);
        e.is_for_compensation = parse_bool(attr(node, "isForCompensation"), false);
        e.triggered_by_event = parse_bool(attr(node, "triggeredByEvent"), false);
        e.script = child_text(node, "script");
        if let Some(mi) = child(node, "multiInstanceLoopCharacteristics") {
            e.multi_instance = Some(MultiInstance {
                sequential: parse_bool(attr(&mi, "isSequential"), false),
                collection: camunda_attr(&mi, "collection").map(String::from),
                element_variable: camunda_attr(&mi, "elementVariable").map(String::from),
                cardinality: child_text(&mi, "loopCardinality"),
                completion_condition: child_text(&mi, "completionCondition"),
                attrs: attrs_of(&mi),
            });
        }
        if child(node, "standardLoopCharacteristics").is_some() {
            e.standard_loop = true;
        }
    }

    fn push_flow(&mut self, node: &Node, kind: FlowKind) -> FlowIdx {
        let f = Flow {
            id: attr(node, "id").unwrap_or("").to_string(),
            name: attr(node, "name").map(String::from),
            kind,
            source_id: attr(node, "sourceRef").map(String::from),
            target_id: attr(node, "targetRef").map(String::from),
            source: None,
            target: None,
            waypoints: vec![],
            label_bounds: None,
            condition: child(node, "conditionExpression").map(|c| expression(&c)),
            is_default: false,
            documentation: documentation(node),
            attrs: attrs_of(node),
            ext: parse_extensions(self.source, node),
            xml_range: node.range(),
            bounds: Rect::EMPTY,
        };
        self.model.flows.push(f);
        self.model.flows.len() - 1
    }

    fn parse_diagram(&mut self, node: &Node) {
        // Build a temporary id index for DI application.
        let mut idx: HashMap<String, Ref> = HashMap::new();
        for (i, e) in self.model.elements.iter().enumerate() {
            idx.insert(e.id.clone(), Ref::Element(i));
        }
        for (i, f) in self.model.flows.iter().enumerate() {
            idx.insert(f.id.clone(), Ref::Flow(i));
        }
        for plane in children(node, "BPMNPlane") {
            for s in plane.children().filter(|c| c.is_element()) {
                match s.tag_name().name() {
                    "BPMNShape" => {
                        let Some(target) = attr(&s, "bpmnElement") else { continue };
                        let bounds = child(&s, "Bounds").and_then(|b| parse_bounds(&b));
                        let label = child(&s, "BPMNLabel").and_then(|l| child(&l, "Bounds")).and_then(|b| parse_bounds(&b));
                        match idx.get(target) {
                            Some(Ref::Element(i)) => {
                                let e = &mut self.model.elements[*i];
                                if let Some(b) = bounds {
                                    e.bounds = b;
                                }
                                e.label_bounds = label;
                                if let Some(v) = attr(&s, "isExpanded") {
                                    e.is_expanded = v == "true";
                                }
                                if let Some(v) = attr(&s, "isHorizontal") {
                                    e.is_horizontal = v == "true";
                                }
                                if let Some(v) = attr(&s, "isMarkerVisible") {
                                    e.marker_visible = v == "true";
                                }
                            }
                            _ => self.model.warnings.push(format!("DI shape {} references unknown element {target}", attr(&s, "id").unwrap_or("?"))),
                        }
                    }
                    "BPMNEdge" => {
                        let Some(target) = attr(&s, "bpmnElement") else { continue };
                        let wps: Vec<Pos> = children(&s, "waypoint")
                            .filter_map(|w| Some(Pos { x: attr(&w, "x")?.parse().ok()?, y: attr(&w, "y")?.parse().ok()? }))
                            .collect();
                        let label = child(&s, "BPMNLabel").and_then(|l| child(&l, "Bounds")).and_then(|b| parse_bounds(&b));
                        match idx.get(target) {
                            Some(Ref::Flow(i)) => {
                                let f = &mut self.model.flows[*i];
                                f.waypoints = wps;
                                f.label_bounds = label;
                            }
                            _ => self.model.warnings.push(format!("DI edge {} references unknown flow {target}", attr(&s, "id").unwrap_or("?"))),
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn finish(&mut self) {
        self.model.build_indexes();
        let m = &mut self.model;

        // Resolve flow endpoints and adjacency.
        for fi in 0..m.flows.len() {
            let (s, t) = {
                let f = &m.flows[fi];
                (f.source_id.as_deref().and_then(|id| m.element_idx(id)), f.target_id.as_deref().and_then(|id| m.element_idx(id)))
            };
            m.flows[fi].source = s;
            m.flows[fi].target = t;
            if let Some(s) = s {
                m.elements[s].outgoing.push(fi);
            }
            if let Some(t) = t {
                m.elements[t].incoming.push(fi);
            }
            let mut b = Rect::EMPTY;
            for w in &m.flows[fi].waypoints {
                b = b.union_pos(*w);
            }
            if let Some(l) = m.flows[fi].label_bounds {
                b = b.union(&l);
            }
            m.flows[fi].bounds = b;
        }
        // Default flows.
        for ei in 0..m.elements.len() {
            if let Some(d) = m.elements[ei].default_flow.clone() {
                if let Some(Ref::Flow(fi)) = m.lookup(&d) {
                    m.flows[fi].is_default = true;
                }
            }
        }
        // Parents: explicit subprocess parent, else the participant of the process.
        let mut participant_of_process: HashMap<String, ElementIdx> = HashMap::new();
        for (i, e) in m.elements.iter().enumerate() {
            if e.kind == ElementKind::Participant {
                if let Some(p) = &e.process_id {
                    participant_of_process.insert(p.clone(), i);
                }
            }
        }
        for ei in 0..m.elements.len() {
            let parent = match &self.element_parent_id[ei] {
                Some(pid) => m.element_idx(pid),
                None => m.elements[ei].process_id.as_ref().and_then(|p| participant_of_process.get(p).copied()).filter(|&p| p != ei),
            };
            m.elements[ei].parent = parent;
        }
        // Boundary events: attach, inherit depth + 1.
        for ei in 0..m.elements.len() {
            if let Some(a) = m.elements[ei].attached_to.clone() {
                if let Some(ai) = m.element_idx(&a) {
                    m.elements[ai].boundary_events.push(ei);
                    let d = m.elements[ai].depth + 1;
                    m.elements[ei].depth = d;
                    m.elements[ei].parent = m.elements[ai].parent;
                }
            }
        }
        // Lanes.
        for (lane_idx, member) in std::mem::take(&mut self.lane_members) {
            if let Some(mi) = m.element_idx(&member) {
                m.elements[mi].lane = Some(lane_idx);
            }
        }
        // Lanes without their own parent get the participant of their process.
        for ei in 0..m.elements.len() {
            if m.elements[ei].kind == ElementKind::Lane && m.elements[ei].parent.is_none() {
                m.elements[ei].parent = m.elements[ei].process_id.as_ref().and_then(|p| participant_of_process.get(p).copied());
            }
        }
        // Bounds and warnings.
        let mut bounds = Rect::EMPTY;
        for e in &m.elements {
            if e.bounds.is_valid() {
                bounds = bounds.union(&e.bounds);
                if let Some(l) = e.label_bounds {
                    bounds = bounds.union(&l);
                }
            } else if e.kind.is_flow_node() {
                m.warnings.push(format!("element {} ({}) has no diagram shape", e.id, e.kind.label()));
            }
        }
        for f in &m.flows {
            if f.bounds.is_valid() {
                bounds = bounds.union(&f.bounds);
            }
        }
        m.bounds = if bounds.is_valid() { bounds } else { Rect::new(0.0, 0.0, 0.0, 0.0) };

        // Draw order: by depth, containers first within the same depth, boundary events last.
        let mut order: Vec<ElementIdx> = (0..m.elements.len()).filter(|&i| m.elements[i].bounds.is_valid()).collect();
        order.sort_by_key(|&i| {
            let e = &m.elements[i];
            let k = match e.kind {
                ElementKind::Participant => 0,
                ElementKind::Lane => 1,
                ElementKind::Group => 2,
                _ if e.kind.is_container() => 3,
                ElementKind::BoundaryEvent => 9,
                _ => 5,
            };
            (e.depth, k)
        });
        m.draw_order = order;

        // Reading order for flow nodes: walk the graph from start events (top-most first) along
        // sequence flows, visiting boundary events right after their host; anything unreachable is
        // appended in geometric order (lane, row, x). This is what Tab cycles through.
        let geo_key = |m: &Model, i: ElementIdx| -> (i64, i64, i64) {
            let e = &m.elements[i];
            let mut cur = e.lane.or(e.parent);
            let mut lane_y = i64::MIN;
            while let Some(c) = cur {
                let ce = &m.elements[c];
                if matches!(ce.kind, ElementKind::Lane | ElementKind::Participant) && ce.bounds.is_valid() {
                    lane_y = ce.bounds.y as i64;
                    break;
                }
                cur = ce.parent;
            }
            let c = e.bounds.center();
            (lane_y, (c.y / 40.0).round() as i64, c.x as i64)
        };
        let mut all: Vec<ElementIdx> = m.flow_nodes().filter(|(_, e)| e.bounds.is_valid()).map(|(i, _)| i).collect();
        all.sort_by_key(|&i| geo_key(m, i));
        let mut visited = vec![false; m.elements.len()];
        let mut ro: Vec<ElementIdx> = Vec::with_capacity(all.len());
        let mut roots: Vec<ElementIdx> = all.iter().copied().filter(|&i| m.elements[i].kind == ElementKind::StartEvent && m.elements[i].attached_to.is_none()).collect();
        // Nodes without incoming flows are roots too (after real start events).
        roots.extend(all.iter().copied().filter(|&i| m.elements[i].kind != ElementKind::StartEvent && m.elements[i].attached_to.is_none() && m.predecessors(i).is_empty()));
        roots.extend(all.iter().copied());
        for root in roots {
            if visited[root] {
                continue;
            }
            let mut stack = vec![root];
            while let Some(i) = stack.pop() {
                if visited[i] {
                    continue;
                }
                visited[i] = true;
                ro.push(i);
                let mut next: Vec<ElementIdx> = vec![];
                // children of a container first, then boundary events, then successors
                if m.elements[i].kind.is_container() {
                    let mut kids: Vec<ElementIdx> = all.iter().copied().filter(|&k| m.elements[k].parent == Some(i) && m.elements[k].attached_to.is_none()).collect();
                    kids.sort_by_key(|&k| (m.elements[k].kind != ElementKind::StartEvent, geo_key(m, k)));
                    next.extend(kids);
                }
                let mut be: Vec<ElementIdx> = m.elements[i].boundary_events.clone();
                be.sort_by_key(|&k| geo_key(m, k));
                next.extend(be);
                let mut succ: Vec<ElementIdx> = m.successors(i).into_iter().map(|(_, t)| t).collect();
                succ.sort_by_key(|&k| geo_key(m, k));
                next.extend(succ);
                // push in reverse so the first candidate is visited first (DFS keeps paths together)
                for n in next.into_iter().rev() {
                    if !visited[n] {
                        stack.push(n);
                    }
                }
            }
        }
        m.reading_order = ro;
    }
}

fn parse_bounds(b: &Node) -> Option<Rect> {
    Some(Rect {
        x: attr(b, "x")?.parse().ok()?,
        y: attr(b, "y")?.parse().ok()?,
        w: attr(b, "width")?.parse().ok()?,
        h: attr(b, "height")?.parse().ok()?,
    })
}
