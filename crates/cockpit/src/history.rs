//! Pure helpers over historic activity instances.

use camunda::HistoricActivityInstance;
use std::collections::HashSet;

/// Camunda records activity instances but not the sequence flows a token took. Infer them: for
/// every activity instance `B`, the predecessor activity `A` (connected by a sequence flow) whose
/// instance ended last, at or before `B` started (with a small tolerance for the engine's clock
/// granularity), is taken to have fed `B`. Multi-instance bodies are skipped.
pub fn infer_traversed_flows(m: &bpmn::Model, history: &[HistoricActivityInstance]) -> HashSet<usize> {
    const TOLERANCE_MS: i64 = 1500;
    let ts = |s: &Option<String>| s.as_deref().and_then(crate::util::parse_time).map(|t| t.timestamp_millis());
    let mut sorted: Vec<&HistoricActivityInstance> = history.iter().filter(|a| !a.is_multi_instance_body()).collect();
    sorted.sort_by(|a, b| a.start_time.cmp(&b.start_time));
    let mut out = HashSet::new();
    for b in &sorted {
        let Some(bi) = m.element_idx(b.base_activity_id()) else { continue };
        let Some(bs) = ts(&b.start_time) else { continue };
        let mut best: Option<(i64, usize)> = None;
        for (flow, ai_idx) in m.predecessors(bi) {
            let aid = &m.elements[ai_idx].id;
            for a in sorted.iter().filter(|a| a.base_activity_id() == aid) {
                let Some(ae) = ts(&a.end_time) else { continue };
                if ae <= bs + TOLERANCE_MS && best.map(|(t, _)| ae > t).unwrap_or(true) {
                    best = Some((ae, flow));
                }
            }
        }
        if let Some((_, f)) = best {
            out.insert(f);
        }
    }
    out
}

/// One running instance that matched a find query, attached to the activity it waits at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstanceHit {
    pub process_instance_id: String,
    pub business_key: Option<String>,
    pub activity_id: String,
}

/// Which instance fields Ctrl+F may match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FindInstanceOptions {
    pub business_keys: bool,
    pub ids: bool,
}

/// Join running activity instances with a find query: an instance matches when its id contains
/// the query (case-insensitive) or it is in `by_business_key` (instances the engine returned for a
/// business-key search). Multi-instance bodies are skipped; results are grouped per activity.
pub fn instance_hits(unfinished: &[HistoricActivityInstance], by_business_key: &[camunda::ProcessInstance], query: &str, opts: FindInstanceOptions) -> Vec<InstanceHit> {
    let q = query.trim().to_lowercase();
    if q.is_empty() || !(opts.business_keys || opts.ids) {
        return vec![];
    }
    let keys: std::collections::HashMap<&str, Option<&str>> = by_business_key.iter().map(|p| (p.id.as_str(), p.business_key.as_deref())).collect();
    let mut out = vec![];
    for a in unfinished.iter().filter(|a| !a.is_multi_instance_body()) {
        let id_match = opts.ids && a.process_instance_id.to_lowercase().contains(&q);
        let bk = if opts.business_keys { keys.get(a.process_instance_id.as_str()) } else { None };
        if id_match || bk.is_some() {
            out.push(InstanceHit { process_instance_id: a.process_instance_id.clone(), business_key: bk.copied().flatten().map(String::from), activity_id: a.base_activity_id().to_string() });
        }
    }
    out.sort_by(|x, y| (&x.activity_id, &x.process_instance_id).cmp(&(&y.activity_id, &y.process_instance_id)));
    out.dedup();
    out
}
