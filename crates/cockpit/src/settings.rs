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

    /// Load from an explicit file; unreadable or missing files yield defaults, unknown keys are
    /// ignored and the dock width is clamped to a sane range.
    pub fn load_from(p: &std::path::Path) -> Self {
        match std::fs::read_to_string(p) {
            Ok(s) => {
                let mut st: Self = toml::from_str(&s).unwrap_or_else(|e| {
                    log::warn!("settings unreadable ({e}); using defaults");
                    Self::default()
                });
                st.dock_width = st.dock_width.clamp(280.0, 600.0);
                st
            }
            Err(_) => Self::default(),
        }
    }
    pub fn save(&self) {
        if let Some(p) = Self::path() {
            self.save_to(&p);
        }
    }

    /// Save to an explicit file. Passwords are stripped unless `remember_password` is set.
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
                if let Err(e) = std::fs::write(p, s) {
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
