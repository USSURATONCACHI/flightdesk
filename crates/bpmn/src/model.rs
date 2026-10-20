//! Data model produced by the BPMN parser. Pure data, no I/O, no UI types.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct Pos {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const EMPTY: Rect = Rect { x: f32::INFINITY, y: f32::INFINITY, w: f32::NEG_INFINITY, h: f32::NEG_INFINITY };
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
    pub fn min(&self) -> Pos {
        Pos { x: self.x, y: self.y }
    }
    pub fn max(&self) -> Pos {
        Pos { x: self.x + self.w, y: self.y + self.h }
    }
    pub fn center(&self) -> Pos {
        Pos { x: self.x + self.w / 2.0, y: self.y + self.h / 2.0 }
    }
    pub fn contains(&self, p: Pos) -> bool {
        p.x >= self.x && p.y >= self.y && p.x <= self.x + self.w && p.y <= self.y + self.h
    }
    pub fn is_valid(&self) -> bool {
        self.w.is_finite() && self.h.is_finite() && self.w >= 0.0 && self.h >= 0.0
    }
    pub fn union(&self, o: &Rect) -> Rect {
        if !self.is_valid() {
            return *o;
        }
        if !o.is_valid() {
            return *self;
        }
        let x0 = self.x.min(o.x);
        let y0 = self.y.min(o.y);
        let x1 = (self.x + self.w).max(o.x + o.w);
        let y1 = (self.y + self.h).max(o.y + o.h);
        Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
    }
    pub fn union_pos(&self, p: Pos) -> Rect {
        self.union(&Rect { x: p.x, y: p.y, w: 0.0, h: 0.0 })
    }
    pub fn expand(&self, m: f32) -> Rect {
        Rect { x: self.x - m, y: self.y - m, w: self.w + 2.0 * m, h: self.h + 2.0 * m }
    }
    pub fn intersects(&self, o: &Rect) -> bool {
        self.x <= o.x + o.w && o.x <= self.x + self.w && self.y <= o.y + o.h && o.y <= self.y + self.h
    }
}

/// Index of an element in `Model::elements`.
pub type ElementIdx = usize;
/// Index of a flow in `Model::flows`.
pub type FlowIdx = usize;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum ElementKind {
    StartEvent,
    EndEvent,
    IntermediateCatchEvent,
    IntermediateThrowEvent,
    BoundaryEvent,
    Task,
    UserTask,
    ServiceTask,
    ScriptTask,
    SendTask,
    ReceiveTask,
    ManualTask,
    BusinessRuleTask,
    CallActivity,
    SubProcess,
    AdHocSubProcess,
    Transaction,
    ExclusiveGateway,
    ParallelGateway,
    InclusiveGateway,
    EventBasedGateway,
    ComplexGateway,
    Participant,
    Lane,
    TextAnnotation,
    Group,
    DataObjectReference,
    DataStoreReference,
    DataInput,
    DataOutput,
    Unknown(String),
}

impl ElementKind {
    pub fn from_tag(tag: &str) -> Self {
        use ElementKind::*;
        match tag {
            "startEvent" => StartEvent,
            "endEvent" => EndEvent,
            "intermediateCatchEvent" => IntermediateCatchEvent,
            "intermediateThrowEvent" => IntermediateThrowEvent,
            "boundaryEvent" => BoundaryEvent,
            "task" => Task,
            "userTask" => UserTask,
            "serviceTask" => ServiceTask,
            "scriptTask" => ScriptTask,
            "sendTask" => SendTask,
            "receiveTask" => ReceiveTask,
            "manualTask" => ManualTask,
            "businessRuleTask" => BusinessRuleTask,
            "callActivity" => CallActivity,
            "subProcess" => SubProcess,
            "adHocSubProcess" => AdHocSubProcess,
            "transaction" => Transaction,
            "exclusiveGateway" => ExclusiveGateway,
            "parallelGateway" => ParallelGateway,
            "inclusiveGateway" => InclusiveGateway,
            "eventBasedGateway" => EventBasedGateway,
            "complexGateway" => ComplexGateway,
            "participant" => Participant,
            "lane" => Lane,
            "textAnnotation" => TextAnnotation,
            "group" => Group,
            "dataObjectReference" => DataObjectReference,
            "dataStoreReference" => DataStoreReference,
            "dataInput" => DataInput,
            "dataOutput" => DataOutput,
            other => Unknown(other.to_string()),
        }
    }

    /// Human readable type name, e.g. "Service task".
    pub fn label(&self) -> String {
        use ElementKind::*;
        match self {
            StartEvent => "Start event".into(),
            EndEvent => "End event".into(),
            IntermediateCatchEvent => "Intermediate catch event".into(),
            IntermediateThrowEvent => "Intermediate throw event".into(),
            BoundaryEvent => "Boundary event".into(),
            Task => "Task".into(),
            UserTask => "User task".into(),
            ServiceTask => "Service task".into(),
            ScriptTask => "Script task".into(),
            SendTask => "Send task".into(),
            ReceiveTask => "Receive task".into(),
            ManualTask => "Manual task".into(),
            BusinessRuleTask => "Business rule task".into(),
            CallActivity => "Call activity".into(),
            SubProcess => "Subprocess".into(),
            AdHocSubProcess => "Ad-hoc subprocess".into(),
            Transaction => "Transaction".into(),
            ExclusiveGateway => "Exclusive gateway".into(),
            ParallelGateway => "Parallel gateway".into(),
            InclusiveGateway => "Inclusive gateway".into(),
            EventBasedGateway => "Event-based gateway".into(),
            ComplexGateway => "Complex gateway".into(),
            Participant => "Pool".into(),
            Lane => "Lane".into(),
            TextAnnotation => "Text annotation".into(),
            Group => "Group".into(),
            DataObjectReference => "Data object".into(),
            DataStoreReference => "Data store".into(),
            DataInput => "Data input".into(),
            DataOutput => "Data output".into(),
            Unknown(t) => t.clone(),
        }
    }

    pub fn is_event(&self) -> bool {
        use ElementKind::*;
        matches!(self, StartEvent | EndEvent | IntermediateCatchEvent | IntermediateThrowEvent | BoundaryEvent)
    }
    pub fn is_gateway(&self) -> bool {
        use ElementKind::*;
        matches!(self, ExclusiveGateway | ParallelGateway | InclusiveGateway | EventBasedGateway | ComplexGateway)
    }
    pub fn is_task(&self) -> bool {
        use ElementKind::*;
        matches!(self, Task | UserTask | ServiceTask | ScriptTask | SendTask | ReceiveTask | ManualTask | BusinessRuleTask)
    }
    pub fn is_activity(&self) -> bool {
        self.is_task() || self.is_container() || matches!(self, ElementKind::CallActivity)
    }
    pub fn is_container(&self) -> bool {
        use ElementKind::*;
        matches!(self, SubProcess | AdHocSubProcess | Transaction)
    }
    pub fn is_flow_node(&self) -> bool {
        self.is_event() || self.is_gateway() || self.is_activity()
    }
    pub fn is_artifact(&self) -> bool {
        use ElementKind::*;
        matches!(self, TextAnnotation | Group | DataObjectReference | DataStoreReference | DataInput | DataOutput)
    }
    /// Short category used by search filters: task/gateway/event/container/pool/lane/artifact.
    pub fn category(&self) -> &'static str {
        use ElementKind::*;
        if self.is_event() {
            "event"
        } else if self.is_gateway() {
            "gateway"
        } else if self.is_task() {
            "task"
        } else {
            match self {
                CallActivity => "call",
                SubProcess | AdHocSubProcess | Transaction => "subprocess",
                Participant => "pool",
                Lane => "lane",
                _ => "artifact",
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Expression {
    pub text: String,
    pub language: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum EventDef {
    Timer { time_date: Option<String>, time_duration: Option<String>, time_cycle: Option<String> },
    Message { message_ref: Option<String>, name: Option<String> },
    Signal { signal_ref: Option<String>, name: Option<String> },
    Error { error_ref: Option<String>, name: Option<String>, code: Option<String>, code_variable: Option<String>, message_variable: Option<String> },
    Escalation { escalation_ref: Option<String>, name: Option<String>, code: Option<String>, code_variable: Option<String> },
    Conditional { condition: Option<Expression>, variable_name: Option<String>, variable_events: Option<String> },
    Link { name: Option<String> },
    Compensate { activity_ref: Option<String>, wait_for_completion: Option<bool> },
    Terminate,
    Cancel,
}

impl EventDef {
    pub fn label(&self) -> &'static str {
        match self {
            EventDef::Timer { .. } => "Timer",
            EventDef::Message { .. } => "Message",
            EventDef::Signal { .. } => "Signal",
            EventDef::Error { .. } => "Error",
            EventDef::Escalation { .. } => "Escalation",
            EventDef::Conditional { .. } => "Conditional",
            EventDef::Link { .. } => "Link",
            EventDef::Compensate { .. } => "Compensation",
            EventDef::Terminate => "Terminate",
            EventDef::Cancel => "Cancel",
        }
    }
}

/// A value of a camunda input/output parameter, which can be nested.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum ParamValue {
    Text(String),
    Script { format: Option<String>, resource: Option<String>, source: String },
    List(Vec<ParamValue>),
    Map(Vec<(String, ParamValue)>),
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct IoParam {
    pub name: String,
    pub value: ParamValue,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Listener {
    /// "execution" or "task"
    pub kind: String,
    pub event: Option<String>,
    pub class: Option<String>,
    pub expression: Option<String>,
    pub delegate_expression: Option<String>,
    pub script: Option<(Option<String>, String)>, // (format, source)
    pub fields: Vec<FieldInjection>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FieldInjection {
    pub name: String,
    pub string_value: Option<String>,
    pub expression: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FormField {
    pub id: String,
    pub label: Option<String>,
    pub field_type: Option<String>,
    pub default_value: Option<String>,
    pub validation: Vec<(String, Option<String>)>,
    pub properties: Vec<(String, String)>,
    pub values: Vec<(String, String)>,
}

/// camunda:in / camunda:out mapping of a call activity.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct VariableMapping {
    pub direction: String, // "in" | "out"
    pub source: Option<String>,
    pub source_expression: Option<String>,
    pub target: Option<String>,
    pub variables: Option<String>, // "all"
    pub business_key: Option<String>,
    pub local: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Extensions {
    pub inputs: Vec<IoParam>,
    pub outputs: Vec<IoParam>,
    pub listeners: Vec<Listener>,
    pub properties: Vec<(String, String)>,
    pub fields: Vec<FieldInjection>,
    pub form_fields: Vec<FormField>,
    pub failed_job_retry_time_cycle: Option<String>,
    pub variable_mappings: Vec<VariableMapping>,
    /// Unknown extension elements, kept as raw XML snippets so nothing is lost.
    pub other: Vec<String>,
}

impl Extensions {
    pub fn is_empty(&self) -> bool {
        self.inputs.is_empty()
            && self.outputs.is_empty()
            && self.listeners.is_empty()
            && self.properties.is_empty()
            && self.fields.is_empty()
            && self.form_fields.is_empty()
            && self.failed_job_retry_time_cycle.is_none()
            && self.variable_mappings.is_empty()
            && self.other.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MultiInstance {
    pub sequential: bool,
    pub collection: Option<String>,
    pub element_variable: Option<String>,
    pub cardinality: Option<String>,
    pub completion_condition: Option<String>,
    pub attrs: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Element {
    pub id: String,
    pub name: Option<String>,
    pub kind: ElementKind,
    pub bounds: Rect,
    pub label_bounds: Option<Rect>,
    pub documentation: Option<String>,
    /// Every XML attribute of the element in document order. Namespaced attributes keep their
    /// prefix as written (e.g. `camunda:delegateExpression`).
    pub attrs: Vec<(String, String)>,
    pub ext: Extensions,
    pub event_defs: Vec<EventDef>,
    /// For boundary events: id of the activity they are attached to.
    pub attached_to: Option<String>,
    /// For boundary events / event sub-process starts: whether they interrupt (`cancelActivity`/`isInterrupting`).
    pub interrupting: bool,
    pub default_flow: Option<String>,
    pub multi_instance: Option<MultiInstance>,
    pub standard_loop: bool,
    pub is_for_compensation: bool,
    pub is_expanded: bool,
    pub triggered_by_event: bool,
    pub is_horizontal: bool,
    pub marker_visible: bool,
    /// Script (script task) source if inline.
    pub script: Option<String>,
    /// Owning process id (for flow nodes) or referenced process (for participants).
    pub process_id: Option<String>,
    /// Containing subprocess (for nested flow nodes) or participant (for top-level ones).
    pub parent: Option<ElementIdx>,
    pub lane: Option<ElementIdx>,
    /// Nesting depth used for draw order (pools 0, lanes 1, top-level nodes 2, …).
    pub depth: u16,
    pub incoming: Vec<FlowIdx>,
    pub outgoing: Vec<FlowIdx>,
    /// Boundary events attached to this activity.
    pub boundary_events: Vec<ElementIdx>,
    /// Byte range of the element's XML in `Model::source`.
    pub xml_range: Range<usize>,
}

impl Element {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }
    pub fn camunda(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k.strip_prefix("camunda:") == Some(name))
            .map(|(_, v)| v.as_str())
    }
    pub fn display_name(&self) -> &str {
        self.name.as_deref().filter(|n| !n.trim().is_empty()).unwrap_or(&self.id)
    }
    pub fn is_throwing(&self) -> bool {
        matches!(self.kind, ElementKind::EndEvent | ElementKind::IntermediateThrowEvent)
    }
    pub fn has_timer(&self) -> bool {
        self.event_defs.iter().any(|d| matches!(d, EventDef::Timer { .. }))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum FlowKind {
    SequenceFlow,
    MessageFlow,
    Association,
    DataInputAssociation,
    DataOutputAssociation,
}

impl FlowKind {
    pub fn label(&self) -> &'static str {
        match self {
            FlowKind::SequenceFlow => "Sequence flow",
            FlowKind::MessageFlow => "Message flow",
            FlowKind::Association => "Association",
            FlowKind::DataInputAssociation => "Data input association",
            FlowKind::DataOutputAssociation => "Data output association",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Flow {
    pub id: String,
    pub name: Option<String>,
    pub kind: FlowKind,
    pub source_id: Option<String>,
    pub target_id: Option<String>,
    pub source: Option<ElementIdx>,
    pub target: Option<ElementIdx>,
    pub waypoints: Vec<Pos>,
    pub label_bounds: Option<Rect>,
    pub condition: Option<Expression>,
    pub is_default: bool,
    pub documentation: Option<String>,
    pub attrs: Vec<(String, String)>,
    pub ext: Extensions,
    pub xml_range: Range<usize>,
    pub bounds: Rect,
}

impl Flow {
    pub fn display_name(&self) -> &str {
        self.name.as_deref().filter(|n| !n.trim().is_empty()).unwrap_or(&self.id)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Process {
    pub id: String,
    pub name: Option<String>,
    pub is_executable: bool,
    pub attrs: Vec<(String, String)>,
    pub documentation: Option<String>,
    pub ext: Extensions,
}

/// Root-level named definitions (messages, errors, signals, escalations).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NamedDef {
    pub id: String,
    pub name: Option<String>,
    pub code: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum Ref {
    Element(ElementIdx),
    Flow(FlowIdx),
}

#[derive(Clone, Debug)]
pub struct Model {
    pub source: Arc<str>,
    pub definitions_attrs: Vec<(String, String)>,
    pub processes: Vec<Process>,
    pub elements: Vec<Element>,
    pub flows: Vec<Flow>,
    pub messages: HashMap<String, NamedDef>,
    pub errors: HashMap<String, NamedDef>,
    pub signals: HashMap<String, NamedDef>,
    pub escalations: HashMap<String, NamedDef>,
    pub(crate) by_id: HashMap<String, Ref>,
    /// Overall bounding box of all shapes and edges.
    pub bounds: Rect,
    /// Element indices sorted for drawing (containers before children).
    pub draw_order: Vec<ElementIdx>,
    /// Flow-node indices in reading order (top-left to bottom-right).
    pub reading_order: Vec<ElementIdx>,
    /// Ids referenced by DI but missing in the model, or model elements with no DI shape.
    pub warnings: Vec<String>,
}

impl Model {
    pub(crate) fn build_indexes(&mut self) {
        self.by_id.clear();
        for (i, e) in self.elements.iter().enumerate() {
            self.by_id.insert(e.id.clone(), Ref::Element(i));
        }
        for (i, f) in self.flows.iter().enumerate() {
            self.by_id.insert(f.id.clone(), Ref::Flow(i));
        }
    }

    pub fn lookup(&self, id: &str) -> Option<Ref> {
        self.by_id.get(id).copied()
    }
    pub fn element_by_id(&self, id: &str) -> Option<&Element> {
        match self.by_id.get(id) {
            Some(Ref::Element(i)) => Some(&self.elements[*i]),
            _ => None,
        }
    }
    pub fn element_idx(&self, id: &str) -> Option<ElementIdx> {
        match self.by_id.get(id) {
            Some(Ref::Element(i)) => Some(*i),
            _ => None,
        }
    }
    pub fn flow_by_id(&self, id: &str) -> Option<&Flow> {
        match self.by_id.get(id) {
            Some(Ref::Flow(i)) => Some(&self.flows[*i]),
            _ => None,
        }
    }
    pub fn xml_of_element(&self, e: &Element) -> &str {
        &self.source[e.xml_range.clone()]
    }
    pub fn xml_of_flow(&self, f: &Flow) -> &str {
        &self.source[f.xml_range.clone()]
    }
    /// The executable processes in this file (what Camunda deploys).
    pub fn executable_processes(&self) -> impl Iterator<Item = &Process> {
        self.processes.iter().filter(|p| p.is_executable)
    }
    /// Flow nodes only (no pools, lanes, artifacts).
    pub fn flow_nodes(&self) -> impl Iterator<Item = (ElementIdx, &Element)> {
        self.elements.iter().enumerate().filter(|(_, e)| e.kind.is_flow_node())
    }
    /// Successors along outgoing sequence flows.
    pub fn successors(&self, idx: ElementIdx) -> Vec<(FlowIdx, ElementIdx)> {
        self.elements[idx]
            .outgoing
            .iter()
            .filter(|&&f| self.flows[f].kind == FlowKind::SequenceFlow)
            .filter_map(|&f| self.flows[f].target.map(|t| (f, t)))
            .collect()
    }
    pub fn predecessors(&self, idx: ElementIdx) -> Vec<(FlowIdx, ElementIdx)> {
        self.elements[idx]
            .incoming
            .iter()
            .filter(|&&f| self.flows[f].kind == FlowKind::SequenceFlow)
            .filter_map(|&f| self.flows[f].source.map(|s| (f, s)))
            .collect()
    }
    /// Sequence flow connecting `a` → `b`, if any.
    pub fn flow_between(&self, a: ElementIdx, b: ElementIdx) -> Option<FlowIdx> {
        self.elements[a].outgoing.iter().copied().find(|&f| self.flows[f].target == Some(b) && self.flows[f].kind == FlowKind::SequenceFlow)
    }
    /// Name of a message/signal/error referenced by id, falling back to the id.
    pub fn resolve_named(&self, map: &HashMap<String, NamedDef>, id: &str) -> String {
        map.get(id).and_then(|d| d.name.clone()).unwrap_or_else(|| id.to_string())
    }
}
