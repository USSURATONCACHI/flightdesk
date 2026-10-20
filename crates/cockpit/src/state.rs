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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavMode {
    Here,
    NewTab,
    /// Replace the current history entry (e.g. version switch).
    Replace,
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
            Place::Definitions(_) => "Definitions".into(),
            Place::Definition(p) => p.title(),
            Place::Instance(p) => p.title(),
            Place::Search(p) => p.title(),
            Place::Incidents(_) => "Incidents".into(),
            Place::Batches(_) => "Batches".into(),
            Place::Settings(_) => "Settings".into(),
            Place::File(p) => p.title(),
        }
    }
    pub fn has_diagram(&self) -> bool {
        matches!(self, Place::Definition(_) | Place::Instance(_) | Place::File(_))
    }
}
