use bpmn::*;

fn fixture() -> Model {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/order.bpmn")).unwrap();
    parse(&src).unwrap()
}

#[test]
fn parses_fixture_structure() {
    let m = fixture();
    assert_eq!(m.processes.len(), 1);
    assert!(m.processes[0].is_executable);
    let validate = m.element_by_id("Task_Validate").unwrap();
    assert_eq!(validate.kind, ElementKind::ServiceTask);
    assert_eq!(validate.camunda("delegateExpression"), Some("${validateOrderDelegate}"));
    assert_eq!(validate.camunda("asyncBefore"), Some("true"));
    assert_eq!(validate.ext.inputs.len(), 2);
    assert_eq!(validate.ext.outputs.len(), 1);
    assert!(matches!(validate.ext.inputs[1].value, ParamValue::Map(ref m) if m.len() == 2));
    assert_eq!(validate.ext.listeners.len(), 1);
    assert_eq!(validate.ext.properties, vec![("owner".to_string(), "sales-team".to_string())]);
    assert_eq!(validate.ext.failed_job_retry_time_cycle.as_deref(), Some("R3/PT10S"));
    assert_eq!(validate.bounds, Rect::new(280.0, 140.0, 100.0, 80.0));
    assert_eq!(validate.lane.map(|l| m.elements[l].id.as_str()), Some("Lane_Sales"));
    assert_eq!(validate.parent.map(|l| m.elements[l].id.as_str()), Some("Participant_Shop"));
    assert_eq!(validate.documentation.as_deref(), Some("Checks the order payload."));
    assert!(m.warnings.is_empty(), "{:?}", m.warnings);
}

#[test]
fn events_gateways_flows() {
    let m = fixture();
    let timer = m.element_by_id("Timer_Reminder").unwrap();
    assert_eq!(timer.kind, ElementKind::BoundaryEvent);
    assert!(!timer.interrupting);
    assert_eq!(timer.attached_to.as_deref(), Some("Task_Approve"));
    assert!(matches!(timer.event_defs[0], EventDef::Timer { time_duration: Some(ref d), .. } if d == "P2D"));
    let approve = m.element_by_id("Task_Approve").unwrap();
    assert_eq!(approve.boundary_events.len(), 1);
    assert_eq!(approve.ext.listeners[0].kind, "task");

    let gw = m.element_by_id("Gateway_Amount").unwrap();
    assert_eq!(gw.default_flow.as_deref(), Some("Flow_Small"));
    assert!(m.flow_by_id("Flow_Small").unwrap().is_default);
    let large = m.flow_by_id("Flow_Large").unwrap();
    assert_eq!(large.condition.as_ref().unwrap().text, "${amount > 1000}");
    assert_eq!(large.waypoints.len(), 3);
    assert_eq!(m.successors(m.element_idx("Gateway_Amount").unwrap()).len(), 2);

    let wait = m.element_by_id("Event_Wait").unwrap();
    assert!(matches!(wait.event_defs[0], EventDef::Message { name: Some(ref n), .. } if n == "paymentReceived"));
    let err = m.element_by_id("Error_Ship").unwrap();
    assert!(matches!(err.event_defs[0], EventDef::Error { code: Some(ref c), .. } if c == "OUT_OF_STOCK"));
    let term = m.element_by_id("EndEvent_Failed").unwrap();
    assert!(matches!(term.event_defs[0], EventDef::Terminate));

    let reserve = m.element_by_id("Task_Reserve").unwrap();
    let mi = reserve.multi_instance.as_ref().unwrap();
    assert!(!mi.sequential);
    assert_eq!(mi.collection.as_deref(), Some("${items}"));
    assert_eq!(reserve.camunda("topic"), Some("stock"));
}

#[test]
fn nesting_and_order() {
    let m = fixture();
    let sub = m.element_idx("Sub_Review").unwrap();
    let script = m.element_by_id("Task_Script").unwrap();
    assert_eq!(script.parent, Some(sub));
    assert_eq!(script.depth, 3);
    assert_eq!(m.elements[sub].depth, 2);
    // draw order: pool, lanes, then nodes; subprocess before its children
    let pos = |id: &str| m.draw_order.iter().position(|&i| m.elements[i].id == id).unwrap();
    assert!(pos("Participant_Shop") < pos("Lane_Sales"));
    assert!(pos("Lane_Sales") < pos("Task_Validate"));
    assert!(pos("Sub_Review") < pos("Task_Script"));
    assert!(pos("Task_Approve") < pos("Timer_Reminder"));
    // reading order starts at the start event
    assert_eq!(m.elements[m.reading_order[0]].id, "StartEvent_1");
    assert!(m.bounds.w > 1200.0 && m.bounds.h > 500.0);
    let ann = m.element_by_id("TextAnnotation_1").unwrap();
    assert_eq!(ann.name.as_deref(), Some("Orders above 1000 need approval"));
    assert!(m.xml_of_element(script).starts_with("<bpmn:scriptTask"));
}

#[test]
fn search_elements() {
    let m = fixture();
    let opts = bpmn::search::SearchOptions { include_flows: true, ..Default::default() };
    let r = bpmn::search::search(&m, "validateOrder", &opts);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].field, "attribute");
    let r = bpmn::search::search(&m, "amount > 1000", &opts);
    assert!(r.iter().any(|m| matches!(m.target, Ref::Flow(_))));
    let r = bpmn::search::search(&m, "task", &bpmn::search::SearchOptions { categories: vec!["task".into()], ..Default::default() });
    assert!(r.len() >= 6);
}

/// Parse every real-world file under $FLIGHTDECK_BPMN_DIR (ignored unless set).
#[test]
fn real_world_files() {
    let Ok(dir) = std::env::var("FLIGHTDECK_BPMN_DIR") else { return };
    let mut n = 0;
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()) != Some("bpmn") {
            continue;
        }
        let src = std::fs::read_to_string(&p).unwrap();
        let t = std::time::Instant::now();
        let m = parse(&src).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
        let dt = t.elapsed();
        let nodes = m.flow_nodes().count();
        eprintln!("{}: {} elements, {} flows, {} flow nodes, {} warnings, {:?}", p.file_name().unwrap().to_string_lossy(), m.elements.len(), m.flows.len(), nodes, m.warnings.len(), dt);
        for w in m.warnings.iter().take(5) {
            eprintln!("   warn: {w}");
        }
        assert!(nodes > 0);
        n += 1;
    }
    assert!(n > 0);
}

#[test]
fn element_search_semantics() {
    let m = fixture();
    use bpmn::search::{search, SearchOptions};
    let all = SearchOptions { include_flows: true, ..Default::default() };
    // every word must match (in any field), case-insensitively
    assert_eq!(search(&m, "APPROVE large", &all).len(), 1);
    assert_eq!(search(&m, "approve nonsense", &all).len(), 0);
    // implementation attributes, listeners and I/O are searchable
    assert_eq!(search(&m, "AuditListener", &all)[0].field, "listener");
    assert_eq!(search(&m, "orderJson", &all)[0].field, "io");
    // conditions on flows are found only when flows are included
    assert!(!search(&m, "amount > 1000", &SearchOptions::default()).iter().any(|m| matches!(m.target, Ref::Flow(_))));
    // category filter restricts to that kind
    let gw = search(&m, "gateway", &SearchOptions { categories: vec!["gateway".into()], ..Default::default() });
    assert!(gw.iter().all(|r| matches!(r.target, Ref::Element(i) if m.elements[i].kind.is_gateway())) && gw.len() == 2);
    // results follow reading order: start event before anything else that matches "order"
    let r = search(&m, "order", &all);
    assert_eq!(match r[0].target { Ref::Element(i) => m.elements[i].id.as_str(), _ => "" }, "StartEvent_1");
    // empty / whitespace query matches nothing
    assert!(search(&m, "   ", &all).is_empty());
}

#[test]
fn successors_ignore_associations_and_message_flows() {
    let m = fixture();
    let gw = m.element_idx("Gateway_Amount").unwrap();
    // Gateway_Amount has an association to the text annotation which must not count as a successor
    assert_eq!(m.successors(gw).len(), 2);
    assert_eq!(m.predecessors(gw).len(), 1);
    assert!(m.elements[gw].outgoing.len() > 2, "raw outgoing still contains the association");
}

#[test]
fn lanes_pools_and_text_annotations_are_found_by_their_text() {
    let m = fixture();
    let found = |q: &str| search::search(&m, q, &search::SearchOptions::default()).into_iter().map(|x| x.target).collect::<Vec<_>>();
    let idx = |id: &str| Ref::Element(m.element_idx(id).unwrap());
    assert!(found("warehouse").contains(&idx("Lane_Warehouse")));
    assert!(found("Order fulfilment").contains(&idx("Participant_Shop")));
    assert_eq!(found("above 1000 need"), vec![idx("TextAnnotation_1")]);
}

#[test]
fn focus_bounds_of_a_swimlane_sit_at_its_label_not_its_middle() {
    let m = fixture();
    let task = m.element_by_id("Task_Validate").unwrap();
    assert_eq!(task.focus_bounds(), task.bounds, "ordinary shapes focus on themselves");
    let lane = m.element_by_id("Lane_Warehouse").unwrap();
    let b = lane.bounds;
    let f = lane.focus_bounds();
    let header = lane.swimlane_header().unwrap();
    assert_eq!(header, Rect::new(b.x, b.y, SWIMLANE_HEADER, b.h));
    assert_eq!(f.x, b.x, "starts at the label header");
    assert!(f.w < b.w, "covers only the beginning of a wide lane");
    assert!(f.contains(header.center()), "the centred label is inside");
    assert!(task.swimlane_header().is_none());
    // A very tall horizontal lane: the focus is centred on the label, not on the top edge.
    let mut tall = lane.clone();
    tall.bounds = Rect::new(0.0, 0.0, 5000.0, 4000.0);
    let f = tall.focus_bounds();
    assert!(f.h < 4000.0 && f.contains(Pos { x: 15.0, y: 2000.0 }), "{f:?}");
    // Vertical lanes put the header along the top.
    tall.is_horizontal = false;
    let f = tall.focus_bounds();
    assert_eq!(f.y, 0.0);
    assert!(f.contains(Pos { x: 2500.0, y: 15.0 }), "{f:?}");
}
