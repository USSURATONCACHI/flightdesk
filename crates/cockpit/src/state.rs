//! Tabs, places and navigation.

use crate::places::*;

/// What a tab shows. The heavy places are boxed so the enum (and every history entry) stays small.
pub enum Place {
    Definitions(DefinitionsPlace),
    Definition(Box<DefinitionPlace>),
    Instance(Box<InstancePlace>),
    Search(Box<SearchPlace>),
    Incidents(Box<IncidentsPlace>),
    Batches(BatchesPlace),
    Settings(SettingsPlace),
    File(Box<FilePlace>),
    Xml(Box<XmlPlace>),
    Diff(Box<DiffPlace>),
}

/// A navigation request; turned into a `Place` by the app.
#[derive(Clone, Debug, PartialEq)]
pub enum Nav {
    Definitions,
    Definition { id: String },
    DefinitionKey { key: String },
    Instance { id: String },
    Search { query: String },
    Incidents,
    Batches,
    Settings,
    File { path: std::path::PathBuf },
    /// XML source of a document (or a line diff), optionally scrolled to an element.
    Xml { doc: crate::places::xml::XmlDoc, element: Option<String> },
    /// Visual diff; a missing side opens the picker for it.
    Diff { left: Option<crate::places::source::DocSource>, right: Option<crate::places::source::DocSource> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavMode {
    Here,
    NewTab,
    /// Replace the current history entry (e.g. version switch).
    Replace,
    /// Switch to a tab already showing the target (pointing it at the request's element),
    /// otherwise open a new tab.
    Reuse,
}

pub struct Tab {
    pub id: u64,
    pub history: Vec<Place>,
    pub cursor: usize,
}

impl Tab {
    pub fn new(id: u64, place: Place) -> Self {
        Self { id, history: vec![place], cursor: 0 }
    }
    pub fn place(&self) -> &Place {
        &self.history[self.cursor]
    }
    pub fn place_mut(&mut self) -> &mut Place {
        &mut self.history[self.cursor]
    }
    pub fn push(&mut self, place: Place) {
        self.history.truncate(self.cursor + 1);
        self.history.push(place);
        if self.history.len() > 40 {
            self.history.remove(0);
        }
        self.cursor = self.history.len() - 1;
    }
    pub fn replace(&mut self, place: Place) {
        self.history[self.cursor] = place;
    }
    pub fn can_back(&self) -> bool {
        self.cursor > 0
    }
    pub fn can_forward(&self) -> bool {
        self.cursor + 1 < self.history.len()
    }
    pub fn back(&mut self) {
        if self.can_back() {
            self.cursor -= 1;
        }
    }
    pub fn forward(&mut self) {
        if self.can_forward() {
            self.cursor += 1;
        }
    }
}

impl Place {
    pub fn title(&self) -> String {
        match self {
            Place::Definitions(_) => crate::t!("place-definitions"),
            Place::Definition(p) => p.title(),
            Place::Instance(p) => p.title(),
            Place::Search(p) => p.title(),
            Place::Incidents(_) => crate::t!("place-incidents"),
            Place::Batches(_) => crate::t!("place-batches"),
            Place::Settings(_) => crate::t!("settings-title"),
            Place::File(p) => p.title(),
            Place::Xml(p) => p.title(),
            Place::Diff(p) => p.title(),
        }
    }
    /// Whether this place shows what `nav` asks for (for `NavMode::Reuse`).
    pub fn shows(&self, nav: &Nav) -> bool {
        match (self, nav) {
            (Place::Definition(p), Nav::Definition { id }) => p.def_id == *id,
            (Place::Instance(p), Nav::Instance { id }) => p.id == *id,
            (Place::File(p), Nav::File { path }) => p.path == *path,
            (Place::Xml(p), Nav::Xml { doc, .. }) => p.doc == *doc,
            (Place::Diff(p), Nav::Diff { left, right }) => p.same_sources(left, right),
            (Place::Settings(_), Nav::Settings) | (Place::Incidents(_), Nav::Incidents) | (Place::Batches(_), Nav::Batches) => true,
            _ => false,
        }
    }
    /// Point a reused place at the specifics of `nav` (the element an XML view should show).
    pub fn retarget(&mut self, nav: Nav) {
        if let (Place::Xml(p), Nav::Xml { element, .. }) = (self, nav) {
            p.retarget(element);
        }
    }
    /// The diagram view of places that draw one (the active one for a diff).
    pub fn diagram_view_mut(&mut self) -> Option<&mut crate::viewport::DiagramView> {
        match self {
            Place::Definition(p) => Some(&mut p.view),
            Place::Instance(p) => Some(&mut p.view),
            Place::File(p) => Some(&mut p.view),
            Place::Diff(p) => Some(p.view_mut()),
            Place::Definitions(_) | Place::Search(_) | Place::Incidents(_) | Place::Batches(_) | Place::Settings(_) | Place::Xml(_) => None,
        }
    }
    pub fn has_diagram(&self) -> bool {
        matches!(self, Place::Definition(_) | Place::Instance(_) | Place::File(_) | Place::Diff(_))
    }
}
