//! Where a BPMN document comes from (a local file or a definition deployed on the engine), how
//! to load it, and how to get it onto disk for external editors.

use crate::app::App;
use bpmn::Model;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DocSource {
    File(PathBuf),
    /// A process definition id on the connected engine.
    Definition(String),
}

pub enum DocState {
    Loading,
    Ready(Arc<Model>),
    Failed(String),
}

/// A source plus its loaded model. Files are read once; definitions come from the app cache.
pub struct DocSlot {
    pub source: DocSource,
    file: Option<Result<Arc<Model>, String>>,
}

impl DocSlot {
    pub fn new(source: DocSource) -> Self {
        Self { source, file: None }
    }
    pub fn state(&mut self, app: &mut App) -> DocState {
        match &self.source {
            DocSource::File(p) => {
                let r = self.file.get_or_insert_with(|| load_file(p));
                match r {
                    Ok(m) => DocState::Ready(m.clone()),
                    Err(e) => DocState::Failed(e.clone()),
                }
            }
            DocSource::Definition(id) => match app.diagram(id) {
                Some(m) => DocState::Ready(m),
                None => match app.cache.diagram_error(id) {
                    Some(e) => DocState::Failed(e),
                    None if app.net.is_none() => DocState::Failed(crate::t!("source-offline")),
                    None => DocState::Loading,
                },
            },
        }
    }
    pub fn model(&mut self, app: &mut App) -> Option<Arc<Model>> {
        match self.state(app) {
            DocState::Ready(m) => Some(m),
            _ => None,
        }
    }
}

pub fn load_file(p: &Path) -> Result<Arc<Model>, String> {
    let text = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
    bpmn::parse(&text).map(Arc::new).map_err(|e| e.to_string())
}

impl DocSource {
    /// Short label: the file name, or `key v3` for a definition.
    pub fn label(&self, app: &App) -> String {
        match self {
            DocSource::File(p) => p.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| p.display().to_string()),
            DocSource::Definition(id) => match app.cache.def_by_id(id) {
                Some(d) => format!("{} v{}", d.key, d.version),
                None => definition_key_version(id).map(|(k, v)| format!("{k} v{v}")).unwrap_or_else(|| id.clone()),
            },
        }
    }
    /// Full path or definition id (for tooltips).
    pub fn detail(&self) -> String {
        match self {
            DocSource::File(p) => p.display().to_string(),
            DocSource::Definition(id) => id.clone(),
        }
    }
    /// The navigation target that shows this document as a diagram.
    pub fn diagram_nav(&self) -> crate::state::Nav {
        match self {
            DocSource::File(p) => crate::state::Nav::File { path: p.clone() },
            DocSource::Definition(id) => crate::state::Nav::Definition { id: id.clone() },
        }
    }
}

/// `key:3:uuid` → (`key`, 3). Definition ids of Camunda 7 have that shape unless set explicitly.
pub fn definition_key_version(id: &str) -> Option<(&str, u32)> {
    let mut it = id.rsplitn(3, ':');
    let _uuid = it.next()?;
    let v = it.next()?.parse().ok()?;
    let key = it.next()?;
    Some((key, v))
}

/// A file name that is safe on every platform and keeps `path:line:col` unambiguous (no colons).
pub fn export_file_name(def_id: &str) -> String {
    let base = match definition_key_version(def_id) {
        Some((k, v)) => {
            let uuid = def_id.rsplit(':').next().unwrap_or("");
            format!("{k}-v{v}-{}", uuid.chars().take(8).collect::<String>())
        }
        None => def_id.to_string(),
    };
    let safe: String = base.chars().map(|c| if c.is_alphanumeric() || matches!(c, '-' | '_' | '.') { c } else { '_' }).collect();
    format!("{safe}.bpmn")
}

/// A path on disk holding exactly `model.source`: the file itself, or for an engine definition a
/// copy under `<data dir>/definitions/` (written when missing or different).
pub fn path_on_disk(app: &App, source: &DocSource, model: &Model) -> Result<PathBuf, String> {
    match source {
        DocSource::File(p) => Ok(std::fs::canonicalize(p).unwrap_or_else(|_| p.clone())),
        DocSource::Definition(id) => {
            let dir = app.data_dir.as_ref().ok_or_else(|| crate::t!("source-no-data-dir"))?.join("definitions");
            std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            let path = dir.join(export_file_name(id));
            let same = std::fs::read(&path).map(|b| b == model.source.as_bytes()).unwrap_or(false);
            if !same {
                std::fs::write(&path, model.source.as_bytes()).map_err(|e| format!("{}: {e}", path.display()))?;
            }
            Ok(path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definition_ids_split_into_key_and_version() {
        assert_eq!(definition_key_version("order_process:3:9f1c2d3e-aaaa-bbbb"), Some(("order_process", 3)));
        assert_eq!(definition_key_version("ns:with:colons:12:abc"), Some(("ns:with:colons", 12)));
        assert_eq!(definition_key_version("custom-id"), None);
    }

    #[test]
    fn export_names_have_no_colons_or_separators() {
        let n = export_file_name("order_process:3:9f1c2d3e-aaaa-bbbb");
        assert_eq!(n, "order_process-v3-9f1c2d3e.bpmn");
        let n = export_file_name("weird/id:with spaces");
        assert!(!n.contains(':') && !n.contains('/') && !n.contains(' '), "{n}");
    }
}
