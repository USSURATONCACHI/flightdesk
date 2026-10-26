use bpmn::diff::*;
use bpmn::*;

fn load(name: &str) -> Model {
    let src = std::fs::read_to_string(format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap();
    parse(&src).unwrap()
}

fn pair() -> (Model, Model) {
    (load("order.bpmn"), load("order-v2.bpmn"))
}

fn item<'a>(d: &'a ModelDiff, id: &str) -> &'a ItemDiff {
    d.items.iter().find(|i| i.id == id).unwrap_or_else(|| panic!("no diff item {id}"))
}

fn prop<'a>(it: &'a ItemDiff, key: &str) -> &'a PropChange {
    it.props.iter().find(|p| p.key == key).unwrap_or_else(|| panic!("{} has no change {key}: {:?}", it.id, it.props))
}

#[test]
fn identical_models_have_no_differences() {
    let (a, _) = pair();
    let d = diff(&a, &a.clone());
    assert!(d.is_empty(), "{:?}", d.items);
    // every item is matched to itself
    assert_eq!(d.new_of_old(Ref::Element(3)), Some(Ref::Element(3)));
    assert_eq!(d.old_of_new(Ref::Flow(2)), Some(Ref::Flow(2)));
}

#[test]
fn renamed_task_and_changed_input_script() {
    let (a, b) = pair();
    let d = diff(&a, &b);
    let v = item(&d, "Task_Validate");
    assert_eq!(v.status, Status::Changed);
    assert!(!v.layout_changed);
    let n = prop(v, "name");
    assert_eq!((n.old.as_deref(), n.new.as_deref()), (Some("Validate order"), Some("Validate order data")));
    let i = prop(v, "input orderJson");
    assert_eq!(i.old.as_deref(), Some("${order}"));
    assert_eq!(i.new.as_deref(), Some("script (javascript)\nJSON.stringify(order)"));
    // unchanged parts of the same element are not reported
    assert!(v.props.iter().all(|p| p.key != "input limits" && p.key != "camunda:delegateExpression"));
}

#[test]
fn added_task_with_its_flows_and_a_retargeted_flow() {
    let (a, b) = pair();
    let d = diff(&a, &b);
    let t = item(&d, "Task_Fraud");
    assert_eq!(t.status, Status::Added);
    assert!(t.old.is_none() && t.new == b.lookup("Task_Fraud"));
    assert_eq!(t.label, "Fraud check (Service task)");
    assert_eq!(item(&d, "Flow_Fraud").status, Status::Added);
    assert!(item(&d, "Flow_Fraud").is_flow);
    let f2 = item(&d, "Flow_2");
    assert_eq!(f2.status, Status::Changed);
    assert!(f2.layout_changed);
    let t = prop(f2, "targetRef");
    assert_eq!((t.old.as_deref(), t.new.as_deref()), (Some("Gateway_Amount"), Some("Task_Fraud")));
}

#[test]
fn removed_branch_is_reported_with_old_refs() {
    let (a, b) = pair();
    let d = diff(&a, &b);
    for id in ["Timer_Reminder", "Task_Notify", "Flow_Remind", "Flow_Remind_End"] {
        let it = item(&d, id);
        assert_eq!(it.status, Status::Removed, "{id}");
        assert!(it.new.is_none());
        assert_eq!(it.old, a.lookup(id));
        assert_eq!(d.for_old(a.lookup(id).unwrap()).map(|i| i.id.as_str()), Some(id));
    }
    // removed items come after everything that exists in the new model
    let first_removed = d.items.iter().position(|i| i.status == Status::Removed).unwrap();
    assert!(d.items[first_removed..].iter().all(|i| i.status == Status::Removed));
}

#[test]
fn changed_condition_and_flow_names() {
    let (a, b) = pair();
    let d = diff(&a, &b);
    let large = item(&d, "Flow_Large");
    assert_eq!(prop(large, "condition").new.as_deref(), Some("${amount > 5000}"));
    assert_eq!(prop(large, "name").old.as_deref(), Some("> 1000"));
    assert_eq!(prop(item(&d, "Flow_Small"), "name").new.as_deref(), Some("≤ 5000"));
}

#[test]
fn added_listener_and_moved_element() {
    let (a, b) = pair();
    let d = diff(&a, &b);
    let ship = item(&d, "Task_Ship");
    let l = prop(ship, "listener execution:end");
    assert_eq!((l.old.as_deref(), l.new.as_deref()), (None, Some("delegateExpression: ${shipAudit}")));
    let r = item(&d, "Task_Reserve");
    assert_eq!(r.status, Status::Moved);
    assert!(r.props.is_empty() && r.layout_changed);
    // untouched elements are absent
    assert!(d.for_new(b.lookup("Task_Approve").unwrap()).is_none());
    assert!(d.items.iter().all(|i| i.id != "StartEvent_1"));
}

#[test]
fn regenerated_id_is_matched_by_name_without_noise_on_its_flows() {
    let (a, b) = pair();
    let d = diff(&a, &b);
    let s = item(&d, "Activity_0x1abc");
    assert_eq!(s.status, Status::Changed);
    assert_eq!(s.old_id.as_deref(), Some("Task_Script"));
    assert_eq!(s.props, vec![PropChange { key: "id".into(), old: Some("Task_Script".into()), new: Some("Activity_0x1abc".into()) }]);
    assert_eq!(d.new_of_old(a.lookup("Task_Script").unwrap()), b.lookup("Activity_0x1abc"));
    // Flow_S1/Flow_S2 reference the renamed task, but that is not a change of theirs
    assert!(d.items.iter().all(|i| i.id != "Flow_S1" && i.id != "Flow_S2"), "{:?}", d.items.iter().map(|i| &i.id).collect::<Vec<_>>());
}

#[test]
fn document_level_changes() {
    let (a, b) = pair();
    let d = diff(&a, &b);
    let get = |k: &str| d.document.iter().find(|p| p.key == k).unwrap_or_else(|| panic!("no {k} in {:?}", d.document));
    assert_eq!(get("process order_process: camunda:historyTimeToLive").new.as_deref(), Some("60"));
    assert!(get("process order_process: documentation").new.as_deref().unwrap().contains("Version 2"));
    assert_eq!(get("message Message_Payment").new.as_deref(), Some("name: paymentConfirmed"));
    assert!(d.document.iter().all(|p| !p.key.starts_with("error ")));
}

#[test]
fn counts_match_items() {
    let (a, b) = pair();
    let d = diff(&a, &b);
    let c = d.counts();
    assert_eq!(c.added, 2, "Task_Fraud, Flow_Fraud");
    assert_eq!(c.removed, 4);
    assert!(c.moved >= 1);
    assert_eq!(c.added + c.removed + c.changed + c.moved, d.items.len());
    assert_eq!(c.document, d.document.len());
    // the reverse diff mirrors added and removed
    let r = diff(&b, &a).counts();
    assert_eq!((r.added, r.removed), (c.removed, c.added));
}

#[test]
fn heuristic_matching_requires_unique_names() {
    let src = std::fs::read_to_string(format!("{}/tests/fixtures/order.bpmn", env!("CARGO_MANIFEST_DIR"))).unwrap();
    // Two tasks lose their ids and get the same name: neither may be matched by name.
    let changed = src.replace("Task_Validate", "Task_X1").replace("Task_Reserve", "Task_X2").replace("name=\"Reserve stock\"", "name=\"Validate order\"");
    let (a, b) = (parse(&src).unwrap(), parse(&changed).unwrap());
    let d = diff(&a, &b);
    assert_eq!(item(&d, "Task_X1").status, Status::Added);
    assert_eq!(item(&d, "Task_Validate").status, Status::Removed);
}

#[test]
fn merged_model_holds_ghosts_consistently() {
    let (a, b) = pair();
    let d = diff(&a, &b);
    let mm = merge(&a, &b, &d);
    let m = &mm.model;
    assert_eq!(m.elements.len(), b.elements.len() + 2);
    assert_eq!(m.flows.len(), b.flows.len() + 2);
    // new items keep their indices
    for (i, e) in b.elements.iter().enumerate() {
        assert_eq!(m.elements[i].id, e.id);
        assert_eq!(m.lookup(&e.id), Some(Ref::Element(i)));
    }
    // ghosts are reachable by id, by the removed map and back
    for id in ["Timer_Reminder", "Task_Notify", "Flow_Remind", "Flow_Remind_End"] {
        let g = m.lookup(id).unwrap();
        assert_eq!(mm.removed.get(&a.lookup(id).unwrap()), Some(&g));
        assert!(mm.is_ghost(g));
        assert_eq!(mm.ghosts[&g], a.lookup(id).unwrap());
    }
    // ghost flows connect to the right elements, surviving or ghost, both ways
    for f in ["Flow_Remind", "Flow_Remind_End"] {
        let Some(Ref::Flow(fi)) = m.lookup(f) else { panic!() };
        let fl = &m.flows[fi];
        let (s, t) = (fl.source.unwrap(), fl.target.unwrap());
        assert_eq!(Some(m.elements[s].id.as_str()), fl.source_id.as_deref());
        assert_eq!(Some(m.elements[t].id.as_str()), fl.target_id.as_deref());
        assert!(m.elements[s].outgoing.contains(&fi) && m.elements[t].incoming.contains(&fi));
    }
    let Some(Ref::Flow(end)) = m.lookup("Flow_Remind_End") else { panic!() };
    assert_eq!(m.elements[m.flows[end].target.unwrap()].id, "EndEvent_Done");
    assert!(m.flows[end].target.unwrap() < b.elements.len(), "connects to the surviving end event");
    // the ghost timer is attached to the surviving task, in its lane
    let timer = m.element_idx("Timer_Reminder").unwrap();
    let approve = m.element_idx("Task_Approve").unwrap();
    assert!(m.elements[approve].boundary_events.contains(&timer));
    assert_eq!(m.elements[timer].lane, m.element_idx("Lane_Sales"));
    // drawing and reading orders cover ghosts exactly once
    for g in mm.ghosts.keys() {
        if let Ref::Element(i) = g {
            assert_eq!(m.draw_order.iter().filter(|&&x| x == *i).count(), 1);
            assert_eq!(m.reading_order.iter().filter(|&&x| x == *i).count(), 1);
        }
    }
    let mut ro = m.reading_order.clone();
    ro.sort();
    ro.dedup();
    assert_eq!(ro.len(), m.reading_order.len());
    // ghost XML comes from the old source
    assert!(m.xml_of_element(&m.elements[timer]).starts_with("<bpmn:boundaryEvent id=\"Timer_Reminder\""));
    assert!(m.xml_of_flow(&m.flows[end]).contains("Flow_Remind_End"));
    assert_eq!(&m.source[..mm.new_source_len], &*b.source);
    assert!(m.bounds.is_valid());
}

#[test]
fn text_rows_line_diff() {
    let rows = text_rows("a\nb\nc\n", "a\nB\nc\nd");
    let tags: Vec<_> = rows.iter().map(|r| (r.tag, r.old_line, r.new_line)).collect();
    assert_eq!(
        tags,
        vec![
            (LineTag::Equal, Some(0), Some(0)),
            (LineTag::Delete, Some(1), None),
            (LineTag::Insert, None, Some(1)),
            (LineTag::Equal, Some(2), Some(2)),
            (LineTag::Insert, None, Some(3)),
        ]
    );
    assert!(text_rows("", "").is_empty());
    assert_eq!(text_rows("", "x\ny").iter().filter(|r| r.tag == LineTag::Insert).count(), 2);
    assert_eq!(text_rows("x\n", "").iter().map(|r| r.tag).collect::<Vec<_>>(), vec![LineTag::Delete]);
    // CRLF and LF line endings compare equal; a trailing newline adds no line
    assert!(text_rows("a\r\nb\r\n", "a\nb").iter().all(|r| r.tag == LineTag::Equal));
    assert_eq!(split_lines("a\r\nb\n"), vec!["a", "b"]);
}

#[test]
fn text_rows_cover_both_sides_in_order() {
    let (a, b) = pair();
    let rows = text_rows(&a.source, &b.source);
    let olds: Vec<usize> = rows.iter().filter_map(|r| r.old_line).collect();
    let news: Vec<usize> = rows.iter().filter_map(|r| r.new_line).collect();
    assert_eq!(olds, (0..split_lines(&a.source).len()).collect::<Vec<_>>());
    assert_eq!(news, (0..split_lines(&b.source).len()).collect::<Vec<_>>());
    for r in rows.iter().filter(|r| r.tag == LineTag::Equal) {
        assert_eq!(split_lines(&a.source)[r.old_line.unwrap()], split_lines(&b.source)[r.new_line.unwrap()]);
    }
}

#[test]
fn inline_ranges_mark_changed_words() {
    let old = r#"<task id="T" name="Validate order">"#;
    let new = r#"<task id="T" name="Validate order data">"#;
    let (a, b) = inline_ranges(old, new);
    assert!(a.is_empty());
    assert_eq!(b.iter().map(|r| &new[r.clone()]).collect::<String>(), " data");
    let (a, b) = inline_ranges("${amount > 1000}", "${amount > 5000}");
    assert_eq!(a.iter().map(|r| &"${amount > 1000}"[r.clone()]).collect::<Vec<_>>(), ["1000"]);
    assert_eq!(b.len(), 1);
    let (a, b) = inline_ranges("", "все новое");
    assert!(a.is_empty());
    assert_eq!(b, vec![0.."все новое".len()]);
}
