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
