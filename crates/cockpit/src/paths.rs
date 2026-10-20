//! Where the app keeps its files. The library never assumes a layout: every directory is handed
//! in from the binary, which resolves command-line flags and environment variables.

use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dirs {
    /// Settings, crash log, backups.
    pub config: PathBuf,
    /// egui/eframe persisted UI state (window geometry, panel sizes).
    pub data: PathBuf,
}

impl Dirs {
    pub fn settings_file(&self) -> PathBuf {
        self.config.join("settings.toml")
    }
    pub fn crash_log(&self) -> PathBuf {
        self.config.join("crash.log")
    }
}

/// Environment variable names honoured by the binary.
pub const ENV_CONFIG_DIR: &str = "FLIGHTDECK_CONFIG_DIR";
pub const ENV_DATA_DIR: &str = "FLIGHTDECK_DATA_DIR";

/// Combine explicitly given directories with platform defaults.
///
/// Rule: all given → use them; none given → defaults; a partial set → an error, so a half
/// configured environment never silently mixes a custom config dir with a default data dir.
pub fn resolve(config: Option<PathBuf>, data: Option<PathBuf>, defaults: impl FnOnce() -> Option<Dirs>) -> Result<Dirs, String> {
    match (config, data) {
        (Some(config), Some(data)) => Ok(Dirs { config, data }),
        (None, None) => defaults().ok_or_else(|| "no home directory: set FLIGHTDECK_CONFIG_DIR and FLIGHTDECK_DATA_DIR".to_string()),
        (Some(_), None) => Err(format!("{ENV_CONFIG_DIR} (or --config-dir) was given but {ENV_DATA_DIR} (or --data-dir) was not; specify both or neither")),
        (None, Some(_)) => Err(format!("{ENV_DATA_DIR} (or --data-dir) was given but {ENV_CONFIG_DIR} (or --config-dir) was not; specify both or neither")),
    }
}

/// The platform's per-user locations (XDG on Linux, AppData on Windows, Library on macOS).
pub fn platform_defaults() -> Option<Dirs> {
    let d = directories::ProjectDirs::from("", "", "flightdeck")?;
    Some(Dirs { config: d.config_dir().to_path_buf(), data: d.data_dir().to_path_buf() })
}

/// `~` expansion for values typed by people.
pub fn expand_tilde(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix("~/") {
        if let Some(home) = directories::BaseDirs::new().map(|b| b.home_dir().to_path_buf()) {
            return home.join(rest);
        }
    }
    PathBuf::from(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn defaults() -> Option<Dirs> {
        Some(Dirs { config: "/d/config".into(), data: "/d/data".into() })
    }
    #[test]
    fn all_or_nothing() {
        assert_eq!(resolve(None, None, defaults).unwrap(), defaults().unwrap());
        assert_eq!(resolve(Some("/c".into()), Some("/x".into()), defaults).unwrap(), Dirs { config: "/c".into(), data: "/x".into() });
        let e = resolve(Some("/c".into()), None, defaults).unwrap_err();
        assert!(e.contains("FLIGHTDECK_DATA_DIR") && e.contains("both or neither"));
        let e = resolve(None, Some("/x".into()), defaults).unwrap_err();
        assert!(e.contains("FLIGHTDECK_CONFIG_DIR"));
        assert!(resolve(None, None, || None).is_err(), "no defaults available is an error, not a panic");
    }
    #[test]
    fn derived_files() {
        let d = Dirs { config: "/c".into(), data: "/x".into() };
        assert_eq!(d.settings_file(), PathBuf::from("/c/settings.toml"));
        assert_eq!(d.crash_log(), PathBuf::from("/c/crash.log"));
        assert_eq!(expand_tilde("/abs/path"), PathBuf::from("/abs/path"));
    }
}
