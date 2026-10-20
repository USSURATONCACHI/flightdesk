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
/// Colon-separated list of directories holding language packs; replaces the default search path.
pub const ENV_LOCALE_DIR: &str = "FLIGHTDECK_LOCALE_DIR";

/// Where language packs are looked for when nothing is specified, most specific first:
/// the user's config and data dirs, every XDG data dir (`/usr/share/flightdeck/locales` for the
/// package), and next to the executable (`target/debug/../../locales` for a cargo build).
pub fn default_locale_dirs(dirs: &Dirs, xdg_data_dirs: Option<&str>, exe: Option<&std::path::Path>) -> Vec<PathBuf> {
    let mut v = vec![dirs.config.join("locales"), dirs.data.join("locales")];
    let xdg = xdg_data_dirs.filter(|s| !s.trim().is_empty()).unwrap_or("/usr/local/share:/usr/share");
    for d in xdg.split(':').filter(|d| !d.is_empty()) {
        v.push(PathBuf::from(d).join("flightdeck").join("locales"));
    }
    if let Some(exe_dir) = exe.and_then(|e| e.parent()) {
        v.push(exe_dir.join("locales"));
        v.push(exe_dir.join("..").join("locales"));
        v.push(exe_dir.join("..").join("..").join("locales"));
    }
    v
}

/// Parse a `PATH`-style list into directories, dropping empty entries.
pub fn split_dir_list(s: &str) -> Vec<PathBuf> {
    s.split(':').filter(|d| !d.trim().is_empty()).map(|d| expand_tilde(d.trim())).collect()
}

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
    fn locale_search_path_is_user_then_system_then_exe() {
        let d = Dirs { config: "/home/u/.config/flightdeck".into(), data: "/home/u/.local/share/flightdeck".into() };
        let v = default_locale_dirs(&d, Some("/opt/share:/usr/share"), Some(std::path::Path::new("/repo/target/debug/flightdeck")));
        let s: Vec<String> = v.iter().map(|p| p.display().to_string()).collect();
        assert_eq!(s[0], "/home/u/.config/flightdeck/locales");
        assert_eq!(s[1], "/home/u/.local/share/flightdeck/locales");
        assert_eq!(s[2], "/opt/share/flightdeck/locales");
        assert_eq!(s[3], "/usr/share/flightdeck/locales");
        assert!(s.iter().any(|p| p == "/repo/target/debug/../../locales"), "{s:?}");
        let v = default_locale_dirs(&d, None, None);
        assert!(v.iter().any(|p| p == std::path::Path::new("/usr/share/flightdeck/locales")), "XDG default applies");
        assert_eq!(split_dir_list("/a::/b:"), vec![PathBuf::from("/a"), PathBuf::from("/b")]);
    }

    #[test]
    fn derived_files() {
        let d = Dirs { config: "/c".into(), data: "/x".into() };
        assert_eq!(d.settings_file(), PathBuf::from("/c/settings.toml"));
        assert_eq!(d.crash_log(), PathBuf::from("/c/crash.log"));
        assert_eq!(expand_tilde("/abs/path"), PathBuf::from("/abs/path"));
    }
}
