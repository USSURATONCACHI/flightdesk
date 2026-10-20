//! The search language: what users type must map to exactly the engine query we send.

use cockpit::query::*;

fn q(s: &str) -> Query {
    parse(s)
}

#[test]
fn tokenizer_respects_quotes_and_reports_spans() {
    let t = tokenize(r#"a:1  b:"two words" 'c d' e"#);
    let texts: Vec<&str> = t.iter().map(|(_, _, s)| s.as_str()).collect();
    assert_eq!(texts, vec!["a:1", "b:\"two words\"", "'c d'", "e"]);
    assert_eq!((t[1].0, t[1].1), (5, 18), "byte spans point at the token in the original text");
}

#[test]
fn operators_are_parsed_longest_first_and_printed_back() {
    let p = q("var.a>=10 var.b<=2 var.c!=x var.d~foo var.e:bar");
    let ops: Vec<Op> = p.terms.iter().map(|t| t.op).collect();
    assert_eq!(ops, vec![Op::Gte, Op::Lte, Op::Neq, Op::Like, Op::Eq]);
    assert!(p.terms[4].colon);
    assert_eq!(print(&p), "var.a>=10 var.b<=2 var.c!=x var.d~foo var.e:bar");
    // quoting is preserved when needed and values with spaces are re-quoted
    let p = q(r#"bk:"ORD 1" def:x"#);
    assert_eq!(print(&p), r#"bk:"ORD 1" def:x"#);
    assert_eq!(parse(&print(&p)), p);
}

#[test]
fn unknown_fields_warn_but_do_not_break_the_query() {
    let p = q("deff:x def:y");
    assert_eq!(p.warnings, vec!["unknown field “deff”"]);
    assert_eq!(build_runtime(&p, &[]).process_definition_key.as_deref(), Some("y"));
}

#[test]
fn runtime_mapping_covers_every_field() {
    let p = q(r#"def:order id:30f8285a-cc52-11f1-8f3a-c6104db61f7c bk:ORD* act:A act:B inc~"timeout" state:suspended super:S tenant:t1 var.n="1" var.m=1 var.f=true var.s~ab*"#);
    assert!(!needs_history(&p));
    let r = build_runtime(&p, &[]);
    assert_eq!(r.process_definition_key.as_deref(), Some("order"));
    assert_eq!(r.process_instance_ids.as_deref(), Some(&["30f8285a-cc52-11f1-8f3a-c6104db61f7c".to_string()][..]));
    assert_eq!(r.business_key_like.as_deref(), Some("ORD%"), "trailing * becomes a SQL wildcard without wrapping");
    assert_eq!(r.activity_id_in.as_deref(), Some(&["A".to_string(), "B".to_string()][..]));
    assert_eq!(r.incident_message_like.as_deref(), Some("%timeout%"), "like without wildcard means contains");
    assert_eq!(r.suspended, Some(true));
    assert_eq!(r.super_process_instance.as_deref(), Some("S"));
    assert_eq!(r.tenant_id_in.as_deref(), Some(&["t1".to_string()][..]));
    let vars = r.variables.unwrap();
    assert_eq!(vars[0].value, serde_json::json!("1"), "quoted numbers stay strings");
    assert_eq!(vars[1].value, serde_json::json!(1));
    assert_eq!(vars[2].value, serde_json::json!(true));
    assert_eq!((vars[3].operator.as_str(), vars[3].value.as_str()), ("like", Some("ab%")));
    assert_eq!(r.variable_names_ignore_case, Some(true));
}

#[test]
fn bare_words_mean_business_key_or_instance_id() {
    let r = build_runtime(&q("ORD-10"), &[]);
    assert_eq!(r.business_key_like.as_deref(), Some("%ORD-10%"));
    assert!(r.process_instance_ids.is_none());
    let h = build_history(&q("30f8285a-cc52-11f1-8f3a-c6104db61f7c"), &[]);
    assert_eq!(h.process_instance_id.as_deref(), Some("30f8285a-cc52-11f1-8f3a-c6104db61f7c"));
    assert!(h.process_instance_business_key_like.is_none());
    // exact business key when no wildcard and explicit field
    assert_eq!(build_runtime(&q("bk:ORD-10"), &[]).business_key.as_deref(), Some("ORD-10"));
}

#[test]
fn history_is_chosen_when_the_question_needs_it() {
    for s in ["exec:A", "state:finished", "state:any", "started>-1d", "finished<today", "by:demo"] {
        assert!(needs_history(&q(s)), "{s} needs history");
    }
    for s in ["def:x", "act:A", "inc:yes", "state:active", "var.a>1"] {
        assert!(!needs_history(&q(s)), "{s} is answerable at runtime");
    }
    let h = build_history(&q("state:completed inc:open exec:A exec:B finished>2026-01-01 started<2026-02-01T10:30"), &[]);
    assert_eq!(h.completed, Some(true));
    assert_eq!(h.incident_status.as_deref(), Some("open"));
    assert_eq!(h.executed_activity_id_in.as_deref(), Some(&["A".to_string(), "B".to_string()][..]));
    assert_eq!(h.finished, Some(true), "a finished time bound implies finished instances");
    assert!(h.finished_after.as_deref().unwrap().starts_with("2026-01-01T00:00:00"));
    assert!(h.started_before.as_deref().unwrap().starts_with("2026-02-01T10:30:00"));
    assert!(h.unfinished.is_none(), "history mode without a state means running and finished");
    assert_eq!(build_history(&q("state:running"), &[]).unfinished, Some(true));
}

#[test]
fn version_resolves_against_the_known_definition_list() {
    let defs = vec![
        camunda::ProcessDefinition { id: "k:1:a".into(), key: "k".into(), version: 1, ..Default::default() },
        camunda::ProcessDefinition { id: "k:2:b".into(), key: "k".into(), version: 2, ..Default::default() },
        camunda::ProcessDefinition { id: "other:1:c".into(), key: "other".into(), version: 1, ..Default::default() },
    ];
    assert_eq!(build_runtime(&q("def:k ver:2"), &defs).process_definition_id.as_deref(), Some("k:2:b"));
    assert_eq!(build_history(&q("def:k@1"), &defs).process_definition_id.as_deref(), Some("k:1:a"));
    // unknown version falls back to the key so the user still gets results
    let r = build_runtime(&q("def:k@9"), &defs);
    assert!(r.process_definition_id.is_none() && r.process_definition_key.as_deref() == Some("k"));
    // wildcard keys expand to the matching keys
    assert_eq!(build_runtime(&q("def:k*"), &defs).process_definition_key_in.as_deref(), Some(&["k".to_string()][..]));
}

#[test]
fn relative_and_absolute_dates() {
    let now = chrono::Local::now();
    let d = date_value("-2h").unwrap();
    let t = cockpit::util::parse_time(&d).unwrap();
    let diff = now.signed_duration_since(t.with_timezone(&chrono::Local)).num_seconds();
    assert!((7190..=7210).contains(&diff), "about two hours ago, got {diff}s");
    assert!(date_value("today").unwrap().contains("T00:00:00"));
    assert!(date_value("2026-03-04").unwrap().starts_with("2026-03-04T00:00:00"));
    assert!(date_value("2026-03-04T05:06:07").unwrap().starts_with("2026-03-04T05:06:07"));
    assert_eq!(date_value("yesterday-ish"), None);
    assert_eq!(date_value("-3x"), None);
}

#[test]
fn suggestions_replace_the_token_under_the_caret() {
    let keys = vec![("order_process".to_string(), "Order process".to_string()), ("other".to_string(), String::new())];
    let acts = vec![("Task_Approve".to_string(), "Approve large order".to_string())];
    let ctx = SuggestCtx { def_keys: &keys, activities: &acts, variable_names: &["amount".into(), "customer".into()], users: &["demo".into()] };
    // field names when the token has no operator yet
    let s = suggest("def:x st", 8, &ctx);
    assert_eq!(s[0].insert, "state:");
    assert_eq!(s[0].range, (6, 8), "only the current token is replaced");
    // definition keys match on key prefix or name substring
    let s = suggest("def:proc", 8, &ctx);
    assert_eq!(s.iter().map(|x| x.label.as_str()).collect::<Vec<_>>(), vec!["order_process"]);
    // activity suggestions by name, keeping the operator the user typed
    let s = suggest("act=appr", 8, &ctx);
    assert_eq!(s[0].insert, "act=Task_Approve ");
    // variables after the dot
    assert_eq!(suggest("var.cu", 6, &ctx)[0].insert, "var.customer=");
    // caret inside an earlier token suggests for that token, not the last one
    let s = suggest("sta def:x", 3, &ctx);
    assert_eq!(s[0].insert, "state:");
    assert_eq!(s[0].range, (0, 3));
    // nothing sensible to suggest for an unknown field value
    assert!(suggest("tenant:abc", 10, &ctx).is_empty());
}

#[test]
fn describe_is_human_readable() {
    assert_eq!(describe(&q("def:x var.a>1 ORD")), "def = x · var.a > 1 · business key ~ ORD");
}
