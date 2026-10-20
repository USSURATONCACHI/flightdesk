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
    /// Ctrl+F also looks for running instances (by business key / id) and highlights their activities.
    pub find_instances: bool,
    pub find_business_keys: bool,
    pub find_ids: bool,
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
            find_instances: true,
            find_business_keys: true,
            find_ids: true,
        }
    }
}

impl Settings {
    /// The per-user settings file (`<config dir>/flightdeck/settings.toml`).
    pub fn path() -> Option<std::path::PathBuf> {
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
            if let Err(e) = perms::create_private_dir(dir) {
                log::warn!("cannot create settings directory: {e}");
            }
        }
        match toml::to_string_pretty(&copy) {
            Ok(s) => {
                let tmp = p.with_extension("toml.tmp");
                if let Err(e) = perms::write_private(&tmp, s.as_bytes()).and_then(|_| std::fs::rename(&tmp, p)) {
                    log::warn!("cannot save settings: {e}");
                }
            }
            Err(e) => log::warn!("cannot serialise settings: {e}"),
        }
    }

    /// The last-used connection, or a default profile when none is saved.
    pub fn engine(&self) -> EngineProfile {
        self.engines.get(self.last_engine.min(self.engines.len().saturating_sub(1))).cloned().unwrap_or_default()
    }
}

/// File-permission hygiene for the configuration directory: the settings file can hold engine
/// passwords, so the directory is `0700` and every file in it `0600`. On non-Unix platforms the
/// per-user config directory is already private and these are no-ops.
pub mod perms {
    use std::path::{Path, PathBuf};

    /// A path whose mode lets other users read it.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Issue {
        pub path: PathBuf,
        pub mode: u32,
        pub is_dir: bool,
    }

    impl Issue {
        pub fn describe(&self) -> String {
            format!("{} is {:o}, expected {}", self.path.display(), self.mode & 0o777, if self.is_dir { "700" } else { "600" })
        }
    }

    pub fn create_private_dir(dir: &Path) -> std::io::Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            if !dir.exists() {
                return std::fs::DirBuilder::new().recursive(true).mode(0o700).create(dir);
            }
            Ok(())
        }
        #[cfg(not(unix))]
        {
            std::fs::create_dir_all(dir)
        }
    }

    /// Create or truncate `p` with owner-only permissions and write `data`.
    pub fn write_private(p: &Path, data: &[u8]) -> std::io::Result<()> {
        use std::io::Write;
        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let mut f = opts.open(p)?;
        f.write_all(data)?;
        #[cfg(unix)]
        {
            // The mode above only applies on creation; an existing file keeps its old mode.
            use std::os::unix::fs::PermissionsExt;
            f.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    }

    /// Everything under the config directory that is readable by group or others.
    pub fn audit(dir: &Path) -> Vec<Issue> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut out = vec![];
            let Ok(meta) = std::fs::metadata(dir) else { return out };
            let mode = meta.permissions().mode();
            if mode & 0o077 != 0 {
                out.push(Issue { path: dir.to_path_buf(), mode, is_dir: true });
            }
            if let Ok(rd) = std::fs::read_dir(dir) {
                let mut entries: Vec<_> = rd.flatten().collect();
                entries.sort_by_key(|e| e.file_name());
                for e in entries {
                    let Ok(m) = e.metadata() else { continue };
                    let mode = m.permissions().mode();
                    if mode & 0o077 != 0 {
                        out.push(Issue { path: e.path(), mode, is_dir: m.is_dir() });
                    }
                }
            }
            out
        }
        #[cfg(not(unix))]
        {
            let _ = dir;
            Vec::new()
        }
    }

    /// Tighten everything `audit` reported. Returns the issues that could not be fixed.
    pub fn fix(issues: &[Issue]) -> Vec<(Issue, String)> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut failed = vec![];
            for i in issues {
                let mode = if i.is_dir { 0o700 } else { 0o600 };
                if let Err(e) = std::fs::set_permissions(&i.path, std::fs::Permissions::from_mode(mode)) {
                    failed.push((i.clone(), e.to_string()));
                }
            }
            failed
        }
        #[cfg(not(unix))]
        {
            let _ = issues;
            Vec::new()
        }
    }
}
