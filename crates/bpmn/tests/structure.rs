//! Structural guarantees of the parser on hand-written diagrams: nesting, lanes, DI gaps,
//! reference resolution, reading order on branches and loops.

use bpmn::*;

const NS: &str = r#"xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL" xmlns:bpmndi="http://www.omg.org/spec/BPMN/20100524/DI" xmlns:dc="http://www.omg.org/spec/DD/20100524/DC" xmlns:di="http://www.omg.org/spec/DD/20100524/DI" xmlns:camunda="http://camunda.org/schema/1.0/bpmn""#;

fn shape(id: &str, x: f32, y: f32, w: f32, h: f32) -> String {
    format!(r#"<bpmndi:BPMNShape id="{id}_di" bpmnElement="{id}"><dc:Bounds x="{x}" y="{y}" width="{w}" height="{h}"/></bpmndi:BPMNShape>"#)
}
fn edge(id: &str, pts: &[(f32, f32)]) -> String {
    let wps: String = pts.iter().map(|(x, y)| format!(r#"<di:waypoint x="{x}" y="{y}"/>"#)).collect();
    format!(r#"<bpmndi:BPMNEdge id="{id}_di" bpmnElement="{id}">{wps}</bpmndi:BPMNEdge>"#)
}
fn doc(process: &str, di: &str) -> String {
    format!(r#"<?xml version="1.0"?><bpmn:definitions {NS} id="d" targetNamespace="t">{process}<bpmndi:BPMNDiagram id="dia"><bpmndi:BPMNPlane id="plane" bpmnElement="p">{di}</bpmndi:BPMNPlane></bpmndi:BPMNDiagram></bpmn:definitions>"#)
}

#[test]
fn loop_and_branch_reading_order_follows_the_flow() {
    // start → A → gw → (B | C) → join → A (loop back) … end
    let process = r#"<bpmn:process id="p" isExecutable="true">
        <bpmn:startEvent id="s"/><bpmn:task id="A"/><bpmn:exclusiveGateway id="gw" default="fB"/>
        <bpmn:task id="B"/><bpmn:task id="C"/><bpmn:exclusiveGateway id="join"/><bpmn:endEvent id="e"/>
        <bpmn:sequenceFlow id="f1" sourceRef="s" targetRef="A"/><bpmn:sequenceFlow id="f2" sourceRef="A" targetRef="gw"/>
        <bpmn:sequenceFlow id="fB" sourceRef="gw" targetRef="B"/><bpmn:sequenceFlow id="fC" sourceRef="gw" targetRef="C"><bpmn:conditionExpression>${x}</bpmn:conditionExpression></bpmn:sequenceFlow>
        <bpmn:sequenceFlow id="f5" sourceRef="B" targetRef="join"/><bpmn:sequenceFlow id="f6" sourceRef="C" targetRef="join"/>
        <bpmn:sequenceFlow id="back" sourceRef="join" targetRef="A"/><bpmn:sequenceFlow id="f7" sourceRef="join" targetRef="e"/>
    </bpmn:process>"#;
    // C is drawn ABOVE B on purpose: reading order must still be graph-driven but tie-break by geometry.
    let di = [shape("s", 0.0, 100.0, 36.0, 36.0), shape("A", 100.0, 80.0, 100.0, 80.0), shape("gw", 250.0, 95.0, 50.0, 50.0), shape("C", 350.0, 0.0, 100.0, 80.0), shape("B", 350.0, 160.0, 100.0, 80.0), shape("join", 500.0, 95.0, 50.0, 50.0), shape("e", 600.0, 100.0, 36.0, 36.0),
        edge("f1", &[(36.0, 118.0), (100.0, 118.0)]), edge("f2", &[(200.0, 118.0), (250.0, 118.0)]), edge("fB", &[(275.0, 145.0), (275.0, 200.0), (350.0, 200.0)]), edge("fC", &[(275.0, 95.0), (275.0, 40.0), (350.0, 40.0)]),
        edge("f5", &[(450.0, 200.0), (525.0, 200.0), (525.0, 145.0)]), edge("f6", &[(450.0, 40.0), (525.0, 40.0), (525.0, 95.0)]), edge("back", &[(525.0, 145.0), (525.0, 300.0), (150.0, 300.0), (150.0, 160.0)]), edge("f7", &[(550.0, 120.0), (600.0, 118.0)])].concat();
    let m = parse(&doc(process, &di)).unwrap();
    let order: Vec<&str> = m.reading_order.iter().map(|&i| m.elements[i].id.as_str()).collect();
    assert_eq!(order, vec!["s", "A", "gw", "C", "B", "join", "e"], "graph order with geometric tie-break (C is above B)");
    // the loop-back flow does not break anything and both gateway branches are reachable
    let gw = m.element_idx("gw").unwrap();
    assert_eq!(m.successors(gw).len(), 2);
    assert!(m.flow_by_id("fB").unwrap().is_default);
    assert!(!m.flow_by_id("fC").unwrap().is_default);
    assert_eq!(m.flow_between(m.element_idx("join").unwrap(), m.element_idx("A").unwrap()).map(|f| m.flows[f].id.as_str()), Some("back"));
}

#[test]
fn nested_subprocesses_pools_and_lanes_resolve_parents_and_depth() {
    let process = r#"<bpmn:collaboration id="col"><bpmn:participant id="pool" name="Pool" processRef="p"/>
        <bpmn:participant id="other" name="Other"/>
        <bpmn:messageFlow id="mf" sourceRef="t1" targetRef="other"/></bpmn:collaboration>
      <bpmn:process id="p" isExecutable="true">
        <bpmn:laneSet id="ls"><bpmn:lane id="l1" name="Top"><bpmn:flowNodeRef>t1</bpmn:flowNodeRef><bpmn:flowNodeRef>sub</bpmn:flowNodeRef>
          <bpmn:childLaneSet id="cls"><bpmn:lane id="l1a" name="Inner"><bpmn:flowNodeRef>t1</bpmn:flowNodeRef></bpmn:lane></bpmn:childLaneSet></bpmn:lane></bpmn:laneSet>
        <bpmn:task id="t1"/>
        <bpmn:subProcess id="sub"><bpmn:subProcess id="sub2" triggeredByEvent="true"><bpmn:startEvent id="es" isInterrupting="false"><bpmn:messageEventDefinition messageRef="msg"/></bpmn:startEvent><bpmn:task id="deep"/></bpmn:subProcess></bpmn:subProcess>
        <bpmn:boundaryEvent id="b" attachedToRef="deep"><bpmn:errorEventDefinition errorRef="err"/></bpmn:boundaryEvent>
      </bpmn:process>
      <bpmn:message id="msg" name="Ping"/><bpmn:error id="err" name="Oops" errorCode="E1"/>"#;
    let di = [shape("pool", 0.0, 0.0, 800.0, 400.0), shape("other", 0.0, 500.0, 800.0, 60.0), shape("l1", 30.0, 0.0, 770.0, 400.0), shape("l1a", 60.0, 0.0, 740.0, 200.0), shape("t1", 100.0, 50.0, 100.0, 80.0), shape("sub", 300.0, 50.0, 400.0, 300.0), shape("sub2", 320.0, 80.0, 300.0, 200.0), shape("es", 340.0, 150.0, 36.0, 36.0), shape("deep", 420.0, 130.0, 100.0, 80.0), shape("b", 500.0, 192.0, 36.0, 36.0), edge("mf", &[(150.0, 130.0), (150.0, 500.0)])].concat();
    let m = parse(&doc(process, &di)).unwrap();
    let e = |id: &str| m.element_by_id(id).unwrap();
    assert_eq!(e("t1").parent.map(|p| m.elements[p].id.as_str()), Some("pool"));
    assert_eq!(e("t1").lane.map(|l| m.elements[l].id.as_str()), Some("l1a"), "innermost lane wins");
    assert_eq!(e("sub").lane.map(|l| m.elements[l].id.as_str()), Some("l1"));
    assert_eq!(e("sub2").parent.map(|p| m.elements[p].id.as_str()), Some("sub"));
    assert_eq!(e("deep").parent.map(|p| m.elements[p].id.as_str()), Some("sub2"));
    assert_eq!((e("sub").depth, e("sub2").depth, e("deep").depth, e("b").depth), (2, 3, 4, 5), "depth grows with nesting; boundary events sit above their host");
    assert!(e("sub2").triggered_by_event);
    assert!(!e("es").interrupting, "isInterrupting=false is honoured on event-subprocess starts");
    assert!(matches!(e("es").event_defs[0], EventDef::Message { name: Some(ref n), .. } if n == "Ping"), "message refs resolve to names");
    assert!(matches!(e("b").event_defs[0], EventDef::Error { code: Some(ref c), name: Some(ref n), .. } if c == "E1" && n == "Oops"));
    assert_eq!(e("deep").boundary_events.len(), 1);
    let mf = m.flow_by_id("mf").unwrap();
    assert_eq!(mf.kind, FlowKind::MessageFlow);
    assert_eq!(mf.target.map(|t| m.elements[t].id.as_str()), Some("other"));
    // draw order: pool before lanes before containers before nodes before boundary events
    let pos = |id: &str| m.draw_order.iter().position(|&i| m.elements[i].id == id).unwrap();
    assert!(pos("pool") < pos("l1") && pos("l1") < pos("l1a") && pos("l1a") < pos("sub") && pos("sub") < pos("sub2") && pos("sub2") < pos("deep") && pos("deep") < pos("b"));
    // the model bounds cover every shape including the second pool
    assert!(m.bounds.y + m.bounds.h >= 560.0 && m.bounds.x <= 0.0);
}

#[test]
fn missing_di_is_reported_not_fatal() {
    let process = r#"<bpmn:process id="p" isExecutable="true"><bpmn:task id="drawn"/><bpmn:task id="ghost"/>
        <bpmn:sequenceFlow id="f" sourceRef="drawn" targetRef="ghost"/></bpmn:process>"#;
    let di = [shape("drawn", 0.0, 0.0, 100.0, 80.0), r#"<bpmndi:BPMNShape id="x_di" bpmnElement="does-not-exist"><dc:Bounds x="1" y="1" width="1" height="1"/></bpmndi:BPMNShape>"#.to_string()].concat();
    let m = parse(&doc(process, &di)).unwrap();
    assert!(!m.element_by_id("ghost").unwrap().bounds.is_valid());
    assert_eq!(m.warnings.len(), 2, "{:?}", m.warnings);
    assert!(m.warnings.iter().any(|w| w.contains("ghost")) && m.warnings.iter().any(|w| w.contains("does-not-exist")));
    // undrawn elements are excluded from draw and reading order but keep their graph links
    assert!(m.draw_order.iter().all(|&i| m.elements[i].id != "ghost"));
    assert!(m.reading_order.iter().all(|&i| m.elements[i].id != "ghost"));
    assert_eq!(m.successors(m.element_idx("drawn").unwrap()).len(), 1);
    assert!(!m.flow_by_id("f").unwrap().bounds.is_valid(), "edge without waypoints has no bounds");
}

#[test]
fn rejects_non_bpmn_and_broken_xml() {
    assert!(matches!(parse("<html/>"), Err(ParseError::NotBpmn(t)) if t == "html"));
    assert!(matches!(parse("<bpmn:definitions"), Err(ParseError::Xml(_))));
}

#[test]
fn extension_elements_keep_everything_including_unknown_ones() {
    let process = r#"<bpmn:process id="p" isExecutable="true"><bpmn:serviceTask id="t" camunda:class="a.B">
      <bpmn:extensionElements>
        <camunda:inputOutput><camunda:inputParameter name="list"><camunda:list><camunda:value>1</camunda:value><camunda:value><camunda:map><camunda:entry key="k">v</camunda:entry></camunda:map></camunda:value></camunda:list></camunda:inputParameter>
          <camunda:outputParameter name="s"><camunda:script scriptFormat="groovy">1+1</camunda:script></camunda:outputParameter></camunda:inputOutput>
        <camunda:executionListener event="end" delegateExpression="${x}"><camunda:field name="f"><camunda:string>sv</camunda:string></camunda:field></camunda:executionListener>
        <camunda:field name="top" expression="${e}"/>
        <camunda:connector><camunda:connectorId>http-connector</camunda:connectorId></camunda:connector>
      </bpmn:extensionElements></bpmn:serviceTask></bpmn:process>"#;
    let m = parse(&doc(process, &shape("t", 0.0, 0.0, 100.0, 80.0))).unwrap();
    let e = m.element_by_id("t").unwrap();
    match &e.ext.inputs[0].value {
        ParamValue::List(items) => {
            assert_eq!(items[0], ParamValue::Text("1".into()));
            assert!(matches!(&items[1], ParamValue::Map(m) if m[0].0 == "k" && m[0].1 == ParamValue::Text("v".into())));
        }
        other => panic!("expected list, got {other:?}"),
    }
    assert!(matches!(&e.ext.outputs[0].value, ParamValue::Script { format: Some(f), source, .. } if f == "groovy" && source == "1+1"));
    assert_eq!(e.ext.listeners[0].fields[0].string_value.as_deref(), Some("sv"));
    assert_eq!(e.ext.fields[0].expression.as_deref(), Some("${e}"));
    assert_eq!(e.ext.other.len(), 1, "unknown extension elements are preserved as raw XML");
    assert!(e.ext.other[0].contains("http-connector"));
    assert!(!e.ext.is_empty());
    assert_eq!(e.camunda("class"), Some("a.B"));
    assert_eq!(e.attr("camunda:class"), Some("a.B"), "namespaced attributes keep their prefix");
}
