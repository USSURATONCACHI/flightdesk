//! Localisation. Language packs are Fluent files (`<tag>.ftl`, e.g. `ru-RU.ftl`) that live in
//! directories handed in by the binary; nothing is embedded, so anyone can drop in another pack.
//!
//! Lookup order for a message: the active pack, then the English pack (`en-US`), then the
//! message id itself, so a half-translated pack still produces a usable UI.

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource, FluentValue};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use unic_langid::LanguageIdentifier;

/// The pack every lookup falls back to.
pub const FALLBACK: &str = "en-US";
/// Settings value meaning "follow the operating system".
pub const SYSTEM: &str = "system";
/// Fluent message each pack uses to name itself in its own language.
const NAME_MESSAGE: &str = "language-name";

/// A language pack found on disk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pack {
    /// BCP 47 tag from the file name, e.g. `ru-RU`.
    pub tag: String,
    /// Self-name from the pack's `language-name` message, e.g. `Русский`.
    pub name: String,
    pub path: PathBuf,
}

/// Scan directories for `*.ftl` packs. Earlier directories win on duplicate tags, so a pack in
/// the user's config directory overrides the installed one.
pub fn discover(dirs: &[PathBuf]) -> Vec<Pack> {
    let mut packs: Vec<Pack> = vec![];
    for dir in dirs {
        let Ok(rd) = std::fs::read_dir(dir) else { continue };
        let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.extension().and_then(|e| e.to_str()) == Some("ftl")).collect();
        entries.sort();
        for path in entries {
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else { continue };
            if stem.parse::<LanguageIdentifier>().is_err() {
                log::warn!("ignoring {}: file name is not a language tag", path.display());
                continue;
            }
            if packs.iter().any(|p| p.tag.eq_ignore_ascii_case(stem)) {
                continue;
            }
            let name = std::fs::read_to_string(&path).ok().and_then(|src| pack_name(&src)).unwrap_or_else(|| stem.to_string());
            packs.push(Pack { tag: stem.to_string(), name, path });
        }
    }
    packs
}

/// The `language-name = …` line of a pack, without parsing the whole file.
fn pack_name(source: &str) -> Option<String> {
    source.lines().find_map(|l| l.strip_prefix(NAME_MESSAGE).and_then(|rest| rest.trim_start().strip_prefix('=')).map(|v| v.trim().to_string())).filter(|s| !s.is_empty())
}

/// Languages the operating system prefers, most preferred first (BCP 47 tags).
pub fn system_languages() -> Vec<String> {
    let mut v: Vec<String> = sys_locale::get_locales().map(|l| l.replace('_', "-")).filter(|l| !l.is_empty()).collect();
    if v.is_empty() {
        v.extend(sys_locale::get_locale());
    }
    v
}

/// Pick the pack for a settings value. `system` (or anything unknown) consults `preferred`,
/// matching the full tag first, then the language subtag; the final fallback is English, then
/// whatever pack exists.
pub fn choose<'a>(setting: &str, preferred: &[String], packs: &'a [Pack]) -> Option<&'a Pack> {
    let exact = |tag: &str| packs.iter().find(|p| p.tag.eq_ignore_ascii_case(tag));
    let by_language = |tag: &str| {
        let lang = tag.split(['-', '_']).next().unwrap_or(tag);
        packs.iter().find(|p| p.tag.split('-').next().map(|l| l.eq_ignore_ascii_case(lang)).unwrap_or(false))
    };
    if setting != SYSTEM && !setting.is_empty() {
        if let Some(p) = exact(setting).or_else(|| by_language(setting)) {
            return Some(p);
        }
    }
    if setting == SYSTEM || setting.is_empty() {
        for want in preferred {
            if let Some(p) = exact(want) {
                return Some(p);
            }
        }
        for want in preferred {
            if let Some(p) = by_language(want) {
                return Some(p);
            }
        }
    }
    exact(FALLBACK).or_else(|| packs.first())
}

struct Catalog {
    tag: String,
    /// Active pack first, English fallback second (absent when English is active or missing).
    bundles: Vec<FluentBundle<FluentResource>>,
}

static ACTIVE: RwLock<Option<Arc<Catalog>>> = RwLock::new(None);

/// Tests that change the process-global active pack take this lock so they do not race.
#[cfg(test)]
pub(crate) static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn parse(path: &Path) -> Result<(FluentBundle<FluentResource>, Vec<String>), String> {
    let source = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let tag: LanguageIdentifier = path.file_stem().and_then(|s| s.to_str()).unwrap_or(FALLBACK).parse().map_err(|e| format!("{}: {e:?}", path.display()))?;
    let (resource, mut warnings) = match FluentResource::try_new(source) {
        Ok(r) => (r, vec![]),
        Err((r, errs)) => (r, errs.iter().map(|e| format!("{}: {e}", path.display())).collect()),
    };
    let mut bundle = FluentBundle::new_concurrent(vec![tag]);
    // No Unicode isolation marks around placeables: they show up as odd glyphs in egui.
    bundle.set_use_isolating(false);
    if let Err(errs) = bundle.add_resource(resource) {
        warnings.extend(errs.iter().map(|e| format!("{}: {e}", path.display())));
    }
    Ok((bundle, warnings))
}

/// What `activate` did, for logging and the settings page.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// Tag of the pack now in use, `None` when no pack could be loaded (ids are shown).
    pub active: Option<String>,
    pub warnings: Vec<String>,
}

/// Load `pack` (plus the English fallback when `pack` is not English) and make it current.
pub fn activate(pack: Option<&Pack>, packs: &[Pack]) -> Report {
    let mut report = Report::default();
    let mut bundles = vec![];
    if let Some(p) = pack {
        match parse(&p.path) {
            Ok((b, w)) => {
                bundles.push(b);
                report.warnings.extend(w);
            }
            Err(e) => report.warnings.push(e),
        }
        if !p.tag.eq_ignore_ascii_case(FALLBACK) {
            if let Some(en) = packs.iter().find(|q| q.tag.eq_ignore_ascii_case(FALLBACK)) {
                match parse(&en.path) {
                    Ok((b, w)) => {
                        bundles.push(b);
                        report.warnings.extend(w);
                    }
                    Err(e) => report.warnings.push(e),
                }
            }
        }
    }
    report.active = pack.filter(|_| !bundles.is_empty()).map(|p| p.tag.clone());
    let catalog = report.active.as_ref().map(|tag| Arc::new(Catalog { tag: tag.clone(), bundles }));
    *ACTIVE.write().unwrap() = catalog;
    for w in &report.warnings {
        log::warn!("i18n: {w}");
    }
    report
}

/// Discover, choose and activate in one go; what `main` and the settings page call.
pub fn load(dirs: &[PathBuf], setting: &str) -> (Vec<Pack>, Report) {
    let packs = discover(dirs);
    let preferred = system_languages();
    let chosen = choose(setting, &preferred, &packs);
    let report = activate(chosen, &packs);
    match &report.active {
        Some(tag) => log::info!("i18n: {tag} (setting {setting}, system {})", preferred.join(",")),
        None => log::warn!("i18n: no language packs found in {:?}; showing message ids", dirs),
    }
    (packs, report)
}

/// Tag of the active pack, if any.
pub fn active_tag() -> Option<String> {
    ACTIVE.read().unwrap().as_ref().map(|c| c.tag.clone())
}

/// Translate a message id, with the fallback chain described in the module docs.
pub fn tr(id: &str) -> String {
    tr_args(id, &[])
}

/// Translate with named arguments (`{ $n }` in the pack).
pub fn tr_args(id: &str, args: &[(&str, FluentValue<'_>)]) -> String {
    let guard = ACTIVE.read().unwrap();
    let Some(catalog) = guard.as_ref() else { return id.to_string() };
    let mut fargs = FluentArgs::new();
    for (k, v) in args {
        fargs.set(*k, v.clone());
    }
    for bundle in &catalog.bundles {
        if let Some(msg) = bundle.get_message(id) {
            if let Some(pattern) = msg.value() {
                let mut errors = vec![];
                let out = bundle.format_pattern(pattern, Some(&fargs), &mut errors);
                for e in errors {
                    log::debug!("i18n: {id}: {e}");
                }
                return out.into_owned();
            }
        }
    }
    log::debug!("i18n: missing message {id}");
    id.to_string()
}

/// Translate `id`, or return `default` when no pack defines it (for labels that come from
/// the model crate as English constants).
pub fn tr_or(id: &str, default: &str) -> String {
    let out = tr(id);
    if out == id { default.to_string() } else { out }
}

/// `"Business key (history)"` → `business-key-history`: message-id form of an English label.
pub fn slug(label: &str) -> String {
    let mut out = String::with_capacity(label.len());
    let mut dash = false;
    for c in label.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    out.trim_end_matches('-').to_string()
}

/// Key/value row label (`kv-<slug>`), falling back to the English label.
pub fn kv_label(label: &str) -> String {
    tr_or(&format!("kv-{}", slug(label)), label)
}

/// Section title (`section-<slug>`), falling back to the English title.
pub fn section_title(title: &str) -> String {
    tr_or(&format!("section-{}", slug(title)), title)
}

/// Dock tab name (`dock-<slug>`); the English names stay the MCP identifiers.
pub fn dock_tab(name: &str) -> String {
    tr_or(&format!("dock-{}", slug(name)), name)
}

/// Table column header (`col-<slug>`).
pub fn col(name: &str) -> String {
    if name.is_empty() {
        return String::new();
    }
    tr_or(&format!("col-{}", slug(name)), name)
}

/// Localised BPMN element kind, e.g. `bpmn-user-task`.
pub fn bpmn_kind(kind: &bpmn::ElementKind) -> String {
    let label = kind.label();
    tr_or(&format!("bpmn-{}", slug(&label)), &label)
}

/// Localised event definition kind, e.g. `bpmn-event-timer`.
pub fn bpmn_event(def: &bpmn::EventDef) -> String {
    let label = def.label();
    tr_or(&format!("bpmn-event-{}", slug(label)), label)
}

/// Localised flow kind, e.g. `bpmn-flow-sequence-flow`.
pub fn bpmn_flow(kind: bpmn::FlowKind) -> String {
    let label = kind.label();
    tr_or(&format!("bpmn-flow-{}", slug(label)), label)
}

/// `t!("id")`, `t!("id", n = 3, name = "x")`: translation with Fluent arguments.
#[macro_export]
macro_rules! t {
    ($id:expr) => {
        $crate::i18n::tr($id)
    };
    ($id:expr, $($k:ident = $v:expr),+ $(,)?) => {
        $crate::i18n::tr_args($id, &[$((stringify!($k), ::fluent_bundle::FluentValue::from($v))),+])
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_pack(dir: &Path, tag: &str, body: &str) -> PathBuf {
        let p = dir.join(format!("{tag}.ftl"));
        std::fs::write(&p, body).unwrap();
        p
    }

    fn repo_locales() -> PathBuf {
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../locales"))
    }
    fn message_ids(path: &Path) -> std::collections::BTreeSet<String> {
        std::fs::read_to_string(path).unwrap().lines().filter_map(|l| {
            let l = l.trim_end();
            if l.starts_with('#') || l.starts_with(' ') || l.starts_with('\t') || l.is_empty() { return None; }
            let (id, _) = l.split_once('=')?;
            let id = id.trim();
            (!id.is_empty() && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')).then(|| id.to_string())
        }).collect()
    }

    #[test]
    fn shipped_packs_parse_cleanly_and_define_the_same_messages() {
        let packs = discover(&[repo_locales()]);
        let tags: Vec<_> = packs.iter().map(|p| p.tag.as_str()).collect();
        for want in ["en-US", "ru-RU", "de-DE"] {
            assert!(tags.contains(&want), "missing shipped pack {want}: {tags:?}");
        }
        let en = packs.iter().find(|p| p.tag == "en-US").unwrap();
        let en_ids = message_ids(&en.path);
        assert!(en_ids.len() > 500, "English pack looks truncated: {}", en_ids.len());
        for p in &packs {
            let (_, warnings) = parse(&p.path).unwrap_or_else(|e| panic!("{e}"));
            assert!(warnings.is_empty(), "{}: {warnings:?}", p.tag);
            let ids = message_ids(&p.path);
            let missing: Vec<_> = en_ids.difference(&ids).collect();
            let extra: Vec<_> = ids.difference(&en_ids).collect();
            assert!(missing.is_empty() && extra.is_empty(), "{}: missing {missing:?}, extra {extra:?}", p.tag);
        }
    }

    #[test]
    fn every_message_id_used_in_the_code_exists_in_english() {
        let en_ids = message_ids(&repo_locales().join("en-US.ftl"));
        let re = regex_lite("t!\\(\"([a-z0-9-]+)\"");
        let mut used = std::collections::BTreeSet::new();
        fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
            for e in std::fs::read_dir(dir).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() { walk(&p, out); } else if p.extension().and_then(|x| x.to_str()) == Some("rs") { out.push(p); }
            }
        }
        let mut files = vec![];
        walk(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src")), &mut files);
        for f in files {
            if f.ends_with("i18n.rs") { continue; }
            let src = std::fs::read_to_string(&f).unwrap();
            for id in re(&src) { used.insert(id); }
        }
        let missing: Vec<_> = used.iter().filter(|k| !en_ids.contains(*k)).collect();
        assert!(missing.is_empty(), "message ids used in code but absent from en-US.ftl: {missing:?}");
        assert!(used.len() > 300, "scan found only {} ids", used.len());
    }

    /// Message ids passed to `t!` without pulling in a regex crate: every string literal in the
    /// macro's first argument, so ids chosen by `if` / `match` inside the call count too.
    fn regex_lite(_pattern: &str) -> impl Fn(&str) -> Vec<String> {
        |src: &str| {
            let mut out = vec![];
            let mut rest = src;
            while let Some(i) = rest.find("t!(") {
                let arg = &rest[i + 3..];
                // `assert!(` ends in `t!(` too: only a whole `t!` (or `crate::t!`) counts.
                if rest[..i].chars().next_back().is_some_and(|c| c.is_alphanumeric() || c == '_') {
                    rest = arg;
                    continue;
                }
                // The first argument ends at the first comma or closing paren at depth 0.
                let (mut depth, mut end, mut in_str) = (0i32, arg.len(), false);
                for (k, c) in arg.char_indices() {
                    match c {
                        '"' => in_str = !in_str,
                        _ if in_str => {}
                        '(' | '{' | '[' => depth += 1,
                        ')' | '}' | ']' if depth == 0 => { end = k; break; }
                        ')' | '}' | ']' => depth -= 1,
                        ',' if depth == 0 => { end = k; break; }
                        _ => {}
                    }
                }
                for (n, lit) in arg[..end].split('"').enumerate() {
                    if n % 2 == 1 && !lit.is_empty() && lit.contains('-') && lit.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') { out.push(lit.to_string()); }
                }
                rest = &arg[end.min(arg.len())..];
            }
            out
        }
    }

    #[test]
    fn slugs_are_stable_message_ids() {
        assert_eq!(slug("User task"), "user-task");
        assert_eq!(slug("Event-based gateway"), "event-based-gateway");
        assert_eq!(slug("executed (history)"), "executed-history");
        assert_eq!(slug("history TTL"), "history-ttl");
        assert_eq!(slug("Input / output mapping"), "input-output-mapping");
        assert_eq!(slug("  spaced  "), "spaced");
    }

    #[test]
    fn discovery_reads_names_and_lets_earlier_dirs_override() {
        let root = std::env::temp_dir().join(format!("fd-i18n-{}", std::process::id()));
        let (user, sys) = (root.join("user"), root.join("sys"));
        std::fs::create_dir_all(&user).unwrap();
        std::fs::create_dir_all(&sys).unwrap();
        tmp_pack(&sys, "en-US", "language-name = English\nhello = Hello\n");
        tmp_pack(&sys, "ru-RU", "language-name = Русский\nhello = Привет\n");
        tmp_pack(&user, "ru-RU", "language-name = Русский (мой)\nhello = Здравствуйте\n");
        tmp_pack(&sys, "notatag!", "hello = x\n");
        std::fs::write(sys.join("README.txt"), "ignored").unwrap();
        let packs = discover(&[user.clone(), sys.clone()]);
        let tags: Vec<_> = packs.iter().map(|p| p.tag.as_str()).collect();
        assert_eq!(tags, ["ru-RU", "en-US"]);
        assert_eq!(packs[0].name, "Русский (мой)", "user copy wins");
        assert_eq!(packs[0].path, user.join("ru-RU.ftl"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn choice_prefers_exact_then_language_then_english() {
        let packs = vec![Pack { tag: "en-US".into(), name: "English".into(), path: "/x/en-US.ftl".into() }, Pack { tag: "ru-RU".into(), name: "Русский".into(), path: "/x/ru-RU.ftl".into() }, Pack { tag: "de-DE".into(), name: "Deutsch".into(), path: "/x/de-DE.ftl".into() }];
        let pick = |setting: &str, sys: &[&str]| choose(setting, &sys.iter().map(|s| s.to_string()).collect::<Vec<_>>(), &packs).map(|p| p.tag.clone()).unwrap();
        assert_eq!(pick("system", &["ru-RU", "en-US"]), "ru-RU");
        assert_eq!(pick("system", &["de-AT"]), "de-DE", "language subtag match");
        assert_eq!(pick("system", &["fr-FR", "ru"]), "ru-RU", "second preference by language");
        assert_eq!(pick("system", &["fr-FR"]), "en-US", "unknown system language falls back to English");
        assert_eq!(pick("system", &[]), "en-US");
        assert_eq!(pick("de-DE", &["ru-RU"]), "de-DE", "explicit choice beats the system");
        assert_eq!(pick("de", &["ru-RU"]), "de-DE");
        assert_eq!(pick("xx-XX", &["ru-RU"]), "en-US", "explicit but missing pack: English, not the system language");
        let only_ru = &packs[1..2];
        assert_eq!(choose("system", &["fr-FR".to_string()], only_ru).unwrap().tag, "ru-RU", "no English: whatever exists");
        assert!(choose("system", &[], &[]).is_none());
    }

    #[test]
    fn lookup_falls_back_to_english_then_to_the_id_and_handles_plurals() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let root = std::env::temp_dir().join(format!("fd-i18n-lookup-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        tmp_pack(&root, "en-US", "language-name = English\nhello = Hello\nonly-en = Only English\nrunning = { $n ->\n    [one] { $n } running instance\n   *[other] { $n } running instances\n}\n");
        tmp_pack(&root, "ru-RU", "language-name = Русский\nhello = Привет, { $name }\nrunning = { $n ->\n    [one] { $n } запущенный экземпляр\n    [few] { $n } запущенных экземпляра\n   *[many] { $n } запущенных экземпляров\n}\n");
        let packs = discover(std::slice::from_ref(&root));
        let ru = packs.iter().find(|p| p.tag == "ru-RU").unwrap();
        let report = activate(Some(ru), &packs);
        assert_eq!(report.active.as_deref(), Some("ru-RU"));
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
        assert_eq!(tr_args("hello", &[("name", "Иван".into())]), "Привет, Иван", "no isolation marks around arguments");
        assert_eq!(tr("only-en"), "Only English", "falls back to the English pack");
        assert_eq!(tr("nowhere"), "nowhere", "falls back to the id");
        assert_eq!(t!("running", n = 1), "1 запущенный экземпляр");
        assert_eq!(t!("running", n = 3), "3 запущенных экземпляра");
        assert_eq!(t!("running", n = 5), "5 запущенных экземпляров");
        assert_eq!(t!("running", n = 21), "21 запущенный экземпляр");
        let en = packs.iter().find(|p| p.tag == "en-US").unwrap();
        activate(Some(en), &packs);
        assert_eq!(t!("running", n = 1), "1 running instance");
        assert_eq!(t!("running", n = 2), "2 running instances");
        activate(None, &packs);
        assert_eq!(tr("hello"), "hello", "nothing loaded: ids are shown, never a panic");
        let _ = std::fs::remove_dir_all(&root);
    }
}
