//! Persisted settings (TOML in the OS config directory).

use crate::theme::ThemeMode;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct EngineProfile {
    pub name: String,
    pub url: String,
    pub user: String,
    pub password: String,
    pub remember_password: bool,
    pub accept_invalid_certs: bool,
    /// Base URL of the old Cockpit web app for "copy Cockpit link" (derived from `url` when empty).
    pub cockpit_url: String,
}

impl Default for EngineProfile {
    fn default() -> Self {
        Self { name: "local".into(), url: "http://localhost:8080/engine-rest".into(), user: String::new(), password: String::new(), remember_password: false, accept_invalid_certs: false, cockpit_url: String::new() }
    }
}

impl EngineProfile {
    pub fn cockpit_base(&self) -> String {
        if !self.cockpit_url.trim().is_empty() {
            return self.cockpit_url.trim_end_matches('/').to_string();
        }
        let u = self.url.trim_end_matches('/');
        match u.rfind("/engine-rest") {
            Some(i) => format!("{}/camunda/app/cockpit/default/#", &u[..i]),
            None => format!("{u}/camunda/app/cockpit/default/#"),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct SavedSearch {
    pub name: String,
    pub query: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub engines: Vec<EngineProfile>,
    pub last_engine: usize,
    pub auto_connect: bool,
    pub theme: ThemeMode,
    /// 0.9 compact, 1.0 normal, 1.15 comfortable
    pub density: f32,
    pub wheel_zooms: bool,
    pub inertia: bool,
    pub pointer_wrap: crate::viewport::WrapMode,
    pub refresh_instance_secs: u64,
    pub refresh_list_secs: u64,
    pub pause_when_unfocused: bool,
    pub confirm_destructive: bool,
    pub dock_width: f32,
    pub saved_searches: Vec<SavedSearch>,
    pub recent_users: Vec<String>,
    pub page_size: usize,
    pub mcp_enabled: bool,
    pub mcp_port: u16,
    pub mcp_token: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            engines: vec![EngineProfile::default()],
            last_engine: 0,
            auto_connect: false,
            theme: ThemeMode::System,
            density: 1.0,
            wheel_zooms: true,
            inertia: true,
            pointer_wrap: Default::default(),
            refresh_instance_secs: 5,
            refresh_list_secs: 10,
            pause_when_unfocused: true,
            confirm_destructive: true,
            dock_width: 420.0,
            saved_searches: vec![],
            recent_users: vec![],
            page_size: 100,
            mcp_enabled: false,
            mcp_port: 7788,
            mcp_token: String::new(),
        }
    }
}

impl Settings {
    fn path() -> Option<std::path::PathBuf> {
        let d = directories::ProjectDirs::from("", "", "flightdeck")?;
        Some(d.config_dir().join("settings.toml"))
    }
    pub fn load() -> Self {
        match Self::path() {
            Some(p) => Self::load_from(&p),
            None => Self::default(),
        }
    }

    /// Load from an explicit file. Missing files yield defaults. Everything the file does contain
    /// is kept field by field: a value that no longer deserialises (a renamed enum variant, a
    /// type change between versions, one malformed connection) only resets *that* value, never
    /// the whole file. A file that is not valid TOML at all is set aside as `*.broken-<time>` so
    /// nothing is overwritten silently.
    pub fn load_from(p: &std::path::Path) -> Self {
        let Ok(text) = std::fs::read_to_string(p) else { return Self::default() };
        match Self::from_toml_lenient(&text) {
            Ok((mut settings, warnings)) => {
                for w in warnings {
                    log::warn!("settings: {w}");
                }
                settings.dock_width = settings.dock_width.clamp(280.0, 600.0);
                settings
            }
            Err(e) => {
                let backup = p.with_extension(format!("toml.broken-{}", chrono::Local::now().format("%Y%m%d-%H%M%S")));
                log::warn!("settings file is not valid TOML ({e}); keeping a copy at {}", backup.display());
                let _ = std::fs::copy(p, &backup);
                Self::default()
            }
        }
    }

    /// Parse TOML keeping every value that still fits the current schema. Returns the settings
    /// and a warning per value that had to be dropped. Errors only when the text is not TOML.
    pub fn from_toml_lenient(text: &str) -> Result<(Self, Vec<String>), toml::de::Error> {
        let file: toml::Table = toml::from_str(text)?;
        let mut warnings = vec![];
        let mut base = toml::Table::try_from(Self::default()).expect("defaults serialise");
        let accepts = |t: &toml::Table| Self::deserialize(t.clone()).is_ok();
        for (key, value) in file {
            // Arrays of tables (engines, saved searches) are filtered element by element.
            if let toml::Value::Array(items) = &value {
                let mut kept = vec![];
                for (i, item) in items.iter().enumerate() {
                    let mut candidate = base.clone();
                    candidate.insert(key.clone(), toml::Value::Array(vec![item.clone()]));
                    if accepts(&candidate) {
                        kept.push(item.clone());
                    } else {
                        warnings.push(format!("ignoring {key}[{i}]: it does not match the current format"));
                    }
                }
                let mut candidate = base.clone();
                candidate.insert(key.clone(), toml::Value::Array(kept));
                if accepts(&candidate) {
                    base = candidate;
                }
                continue;
            }
            let mut candidate = base.clone();
            candidate.insert(key.clone(), value);
            if accepts(&candidate) {
                base = candidate;
            } else {
                warnings.push(format!("ignoring `{key}`: its value is not valid for this version, using the default"));
            }
        }
        let settings = Self::deserialize(base).expect("merged table was validated field by field");
        Ok((settings, warnings))
    }

    pub fn save(&self) {
        if let Some(p) = Self::path() {
            self.save_to(&p);
        }
    }

    /// Save atomically (write a temp file, then rename). Passwords are stripped unless
    /// `remember_password` is set.
    pub fn save_to(&self, p: &std::path::Path) {
        let mut copy = self.clone();
        for e in &mut copy.engines {
            if !e.remember_password {
                e.password.clear();
            }
        }
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        match toml::to_string_pretty(&copy) {
            Ok(s) => {
                let tmp = p.with_extension("toml.tmp");
                if let Err(e) = std::fs::write(&tmp, s).and_then(|_| std::fs::rename(&tmp, p)) {
                    log::warn!("cannot save settings: {e}");
                }
            }
            Err(e) => log::warn!("cannot serialise settings: {e}"),
        }
    }

    pub fn engine(&self) -> &EngineProfile {
        &self.engines[self.last_engine.min(self.engines.len().saturating_sub(1))]
    }
}
