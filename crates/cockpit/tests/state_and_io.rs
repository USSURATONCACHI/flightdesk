//! Tab history, settings persistence, the async loader, historic path inference and theme probing.

use cockpit::net::{Loader, Net};
use cockpit::settings::{EngineProfile, Settings};
use cockpit::state::{Place, Tab};
use cockpit::theme::ThemeMode;
use std::time::Duration;

fn place(n: usize) -> Place {
    Place::Search(Box::new(cockpit::places::SearchPlace::new(format!("q{n}"))))
}
fn title(t: &Tab) -> String {
    t.place().title()
}

#[test]
fn tab_history_behaves_like_a_browser() {
    let mut t = Tab::new(1, place(0));
    assert!(!t.can_back() && !t.can_forward());
    t.push(place(1));
    t.push(place(2));
    assert!(t.can_back() && !t.can_forward());
    t.back();
    assert_eq!(title(&t), "🔍 q1");
    assert!(t.can_forward());
    // pushing after going back discards the forward entries
    t.push(place(3));
    assert!(!t.can_forward());
    t.back();
    t.back();
    assert_eq!(title(&t), "🔍 q0");
    t.back();
    assert_eq!(title(&t), "🔍 q0", "back at the start is a no-op");
    t.forward();
    t.forward();
    t.forward();
    assert_eq!(title(&t), "🔍 q3");
    // replace swaps the current entry in place (version switch)
    t.replace(place(9));
    assert_eq!(title(&t), "🔍 q9");
    assert!(t.can_back());
    // history is capped so long sessions do not grow without bound
    for i in 0..100 {
        t.push(place(i));
    }
    assert!(t.history.len() <= 40);
    assert_eq!(title(&t), "🔍 q99");
}

#[test]
fn settings_round_trip_strips_unremembered_passwords_and_tolerates_junk() {
    let dir = std::env::temp_dir().join(format!("flightdeck-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("settings.toml");
    let mut s = Settings { dock_width: 5000.0, theme: ThemeMode::Dark, page_size: 42, ..Default::default() };
    s.engines = vec![
        EngineProfile { name: "a".into(), url: "http://a/engine-rest".into(), user: "u".into(), password: "secret".into(), remember_password: false, ..Default::default() },
        EngineProfile { name: "b".into(), url: "http://b/engine-rest".into(), user: "u".into(), password: "keep".into(), remember_password: true, ..Default::default() },
    ];
    s.save_to(&path);
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(!text.contains("secret"), "unremembered passwords never reach disk");
    assert!(text.contains("keep"));
    let loaded = Settings::load_from(&path);
    assert_eq!(loaded.theme, ThemeMode::Dark);
    assert_eq!(loaded.page_size, 42);
    assert_eq!(loaded.engines[0].password, "");
    assert_eq!(loaded.engines[1].password, "keep");
    assert_eq!(loaded.dock_width, 600.0, "absurd persisted widths are clamped on load");
    // an older/newer file with unknown keys and missing ones still loads
    std::fs::write(&path, "theme = \"Light\"\nfuture_option = 1\n[[engines]]\nname = \"x\"\n").unwrap();
    let loaded = Settings::load_from(&path);
    assert_eq!(loaded.theme, ThemeMode::Light);
    assert_eq!(loaded.engines[0].name, "x");
    assert_eq!(loaded.engines[0].url, "http://localhost:8080/engine-rest", "missing fields take defaults");
    // garbage falls back to defaults instead of failing to start
    std::fs::write(&path, "this is not toml = = =").unwrap();
    assert_eq!(Settings::load_from(&path), Settings::default());
    assert_eq!(Settings::load_from(&dir.join("missing.toml")), Settings::default());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cockpit_link_base_is_derived_from_the_rest_url() {
    let p = EngineProfile { url: "https://host:8443/camunda/engine-rest".into(), ..Default::default() };
    assert_eq!(p.cockpit_base(), "https://host:8443/camunda/camunda/app/cockpit/default/#");
    let p = EngineProfile { url: "http://h/engine-rest".into(), cockpit_url: "http://cockpit.example/#/".into(), ..Default::default() };
    assert_eq!(p.cockpit_base(), "http://cockpit.example/#", "an explicit Cockpit URL wins and loses its trailing slash");
}

fn net() -> Net<()> {
    let client = camunda::Client::new(camunda::Config { base_url: "http://127.0.0.1:9/engine-rest".into(), timeout: Duration::from_millis(300), ..Default::default() }).unwrap();
    Net::new(client, egui::Context::default())
}

fn wait_for<T>(l: &mut Loader<T>) {
    let start = std::time::Instant::now();
    while !l.poll() {
        assert!(start.elapsed() < Duration::from_secs(5), "loader never completed");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn loader_keeps_last_good_data_through_refreshes_and_errors() {
    let net = net();
    let mut l: Loader<Vec<u32>> = Loader::default();
    assert!(l.is_empty() && l.stale(Duration::ZERO));
    net.fetch(&mut l, |_| async { Ok(vec![1, 2]) });
    assert!(l.is_loading());
    assert!(!l.stale(Duration::ZERO), "no double fetch while one is in flight");
    wait_for(&mut l);
    assert_eq!(l.data.as_deref(), Some(&[1, 2][..]));
    assert!(l.error.is_none() && !l.is_loading());
    assert!(!l.stale(Duration::from_secs(60)) && l.stale(Duration::ZERO));
    // a failing refresh reports the error but keeps the previous data visible
    net.fetch(&mut l, |_| async { Err(camunda::ApiError::Timeout) });
    assert!(l.data.is_some(), "data stays while loading");
    wait_for(&mut l);
    assert_eq!(l.data.as_deref(), Some(&[1, 2][..]));
    assert_eq!(l.error, Some(camunda::ApiError::Timeout));
    // a later success clears the error
    net.fetch(&mut l, |_| async { Ok(vec![3]) });
    wait_for(&mut l);
    assert_eq!(l.data.as_deref(), Some(&[3][..]));
    assert!(l.error.is_none());
    // invalidate forces the next poll cycle to refetch without dropping data
    l.invalidate();
    assert!(l.stale(Duration::from_secs(3600)) && l.data.is_some());
    l.reset();
    assert!(l.is_empty());
}

#[test]
fn a_real_connection_failure_is_a_network_error_not_a_panic() {
    let net = net();
    let mut l: Loader<camunda::Version> = Loader::default();
    net.fetch(&mut l, |c| async move { c.version().await });
    wait_for(&mut l);
    assert!(matches!(l.error, Some(camunda::ApiError::Network(_)) | Some(camunda::ApiError::Timeout)), "{:?}", l.error);
    assert!(l.data.is_none());
}

#[test]
fn action_callbacks_arrive_on_the_draining_side_in_order() {
    let net: Net<Vec<&'static str>> = Net::new(camunda::Client::new(camunda::Config::default()).unwrap(), egui::Context::default());
    net.run(|_| async { Ok::<_, camunda::ApiError>(1) }, |log, r| log.push(if r.is_ok() { "first ok" } else { "first err" }));
    net.run(|_| async { Err::<u8, _>(camunda::ApiError::Timeout) }, |log, r| log.push(if r.is_ok() { "second ok" } else { "second err" }));
    let mut log = vec![];
    let start = std::time::Instant::now();
    while log.len() < 2 {
        for cb in net.drain() {
            cb(&mut log);
        }
        assert!(start.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(5));
    }
    log.sort();
    assert_eq!(log, vec!["first ok", "second err"]);
    assert_eq!(net.in_flight(), 0);
}

#[test]
fn traversed_flows_are_inferred_from_activity_timing() {
    use camunda::HistoricActivityInstance as H;
    let m = bpmn::parse(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../bpmn/tests/fixtures/order.bpmn")).unwrap()).unwrap();
    let h = |id: &str, start: &str, end: Option<&str>| H { activity_id: id.into(), start_time: Some(format!("2026-10-20T10:{start}.000+0300")), end_time: end.map(|e| format!("2026-10-20T10:{e}.000+0300")), ..Default::default() };
    // start → validate → gateway → approve (large branch) → join; the small branch was NOT taken
    let hist = vec![
        h("StartEvent_1", "00:00", Some("00:00")),
        h("Task_Validate", "00:00", Some("00:05")),
        h("Gateway_Amount", "00:05", Some("00:05")),
        h("Task_Approve", "00:05", Some("00:30")),
        h("Gateway_Join", "00:30", Some("00:30")),
        h("Task_Reserve#multiInstanceBody", "00:30", None),
        h("Task_Reserve", "00:30", None),
    ];
    let flows = cockpit::history::infer_traversed_flows(&m, &hist);
    let ids: std::collections::BTreeSet<&str> = flows.iter().map(|&f| m.flows[f].id.as_str()).collect();
    assert_eq!(ids, ["Flow_1", "Flow_2", "Flow_Large", "Flow_Approved", "Flow_3"].into_iter().collect());
    assert!(!ids.contains("Flow_Small"), "the branch that was not executed is not highlighted");
    // engine timestamps can be a few hundred ms *after* the successor's start (same transaction): still linked
    let hist = vec![h("StartEvent_1", "00:00", Some("00:01")), h("Task_Validate", "00:00", None)];
    let flows = cockpit::history::infer_traversed_flows(&m, &hist);
    assert_eq!(flows.len(), 1);
    // but a predecessor that ended long after the successor started cannot have fed it
    let hist = vec![h("StartEvent_1", "00:00", Some("00:10")), h("Task_Validate", "00:00", None)];
    assert!(cockpit::history::infer_traversed_flows(&m, &hist).is_empty());
    // unknown activity ids (from a newer diagram version) are ignored
    let hist = vec![h("Gone_Task", "00:00", Some("00:00")), h("Task_Validate", "00:01", None)];
    assert!(cockpit::history::infer_traversed_flows(&m, &hist).is_empty());
}

#[test]
fn desktop_theme_probes_are_parsed_defensively() {
    use cockpit::theme::system::{parse_gsettings_color_scheme, parse_portal_color_scheme};
    assert_eq!(parse_portal_color_scheme("(<uint32 1>,)\n"), Some(true));
    assert_eq!(parse_portal_color_scheme("(<uint32 2>,)"), Some(false));
    assert_eq!(parse_portal_color_scheme("(<uint32 0>,)"), None);
    assert_eq!(parse_portal_color_scheme("v u 1"), Some(true));
    assert_eq!(parse_portal_color_scheme("error: no such interface"), None);
    assert_eq!(parse_gsettings_color_scheme("'prefer-dark'\n"), Some(true));
    assert_eq!(parse_gsettings_color_scheme("'default'"), Some(false));
    assert_eq!(parse_gsettings_color_scheme(""), None);
}

#[test]
fn settings_survive_schema_changes_without_losing_connections() {
    let dir = std::env::temp_dir().join(format!("flightdeck-lenient-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("settings.toml");
    // A file written by another version: an enum variant that no longer exists, a field with the
    // wrong type, an unknown key, and one connection entry that is malformed.
    std::fs::write(
        &path,
        r#"
theme = "Dark"
pointer_wrap = "Lock"
page_size = "lots"
some_future_knob = { a = 1 }
refresh_list_secs = 42

[[engines]]
name = "prod"
url = "https://prod/engine-rest"
user = "ops"

[[engines]]
name = 17
url = ["not", "a", "string"]

[[engines]]
name = "staging"
url = "https://staging/engine-rest"

[[saved_searches]]
name = "late"
query = "state:finished finished<-1d"
"#,
    )
    .unwrap();
    let s = Settings::load_from(&path);
    assert_eq!(s.theme, ThemeMode::Dark, "valid values are kept");
    assert_eq!(s.refresh_list_secs, 42);
    assert_eq!(s.page_size, Settings::default().page_size, "only the bad value falls back");
    assert_eq!(s.pointer_wrap, Default::default());
    let names: Vec<&str> = s.engines.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["prod", "staging"], "the malformed connection is dropped, the others stay");
    assert_eq!(s.engines[0].user, "ops");
    assert_eq!(s.saved_searches.len(), 1);
    // saving and loading again is stable and keeps the connections
    s.save_to(&path);
    let again = Settings::load_from(&path);
    assert_eq!(again.engines, s.engines);
    assert!(!dir.join("settings.toml.tmp").exists(), "atomic save leaves no temp file behind");
    // a file that is not TOML at all is backed up, not overwritten
    std::fs::write(&path, "[[engines]\nname = \"half\"").unwrap();
    let s = Settings::load_from(&path);
    assert_eq!(s, Settings::default());
    let backups: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().contains("broken")).collect();
    assert_eq!(backups.len(), 1, "the unreadable file was preserved");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_app_without_a_settings_path_never_writes_a_settings_file() {
    use cockpit::app::App;
    use egui_kittest::Harness;
    let dir = std::env::temp_dir().join(format!("flightdeck-persist-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let target = dir.join("settings.toml");
    // 1. default app (as every headless test builds it): dirty settings are dropped, not written
    let ctx = egui::Context::default();
    let mut app = App::new(&ctx, Settings::default(), 1.0);
    app.offline = true;
    assert!(app.settings_path.is_none(), "tests must not point at the real configuration");
    app.set_theme_mode(ThemeMode::Dark);
    let mut h = Harness::builder().allow_missing_glyphs().build_ui_state(|ui, app: &mut App| app.frame(ui), app);
    h.run_steps(3);
    assert!(!target.exists());
    assert!(!h.state().settings_dirty, "the dirty flag was consumed without a file");
    // 2. with an explicit path, the same change is persisted there and only there
    let ctx = egui::Context::default();
    let mut app = App::new(&ctx, Settings::default(), 1.0);
    app.offline = true;
    app.settings_path = Some(target.clone());
    app.set_theme_mode(ThemeMode::Light);
    let mut h = Harness::builder().allow_missing_glyphs().build_ui_state(|ui, app: &mut App| app.frame(ui), app);
    h.run_steps(3);
    assert_eq!(Settings::load_from(&target).theme, ThemeMode::Light);
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(unix)]
#[test]
fn settings_files_are_private_and_loose_permissions_are_detected_and_fixed() {
    use cockpit::settings::perms;
    use std::os::unix::fs::PermissionsExt;
    let base = std::env::temp_dir().join(format!("flightdeck-perms-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let dir = base.join("flightdeck");
    let path = dir.join("settings.toml");
    // a fresh save creates a 700 directory and a 600 file
    Settings::default().save_to(&path);
    assert_eq!(std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777, 0o700);
    assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
    assert!(perms::audit(&dir).is_empty());
    // saving over a file that was loosened keeps it private (mode is re-applied, not only on create)
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    Settings::default().save_to(&path);
    assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
    // an audit catches a world-readable directory, file and stray backup, and fix repairs them all
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o664)).unwrap();
    std::fs::write(dir.join("settings.toml.broken-x"), "junk").unwrap();
    std::fs::set_permissions(dir.join("settings.toml.broken-x"), std::fs::Permissions::from_mode(0o644)).unwrap();
    let issues = perms::audit(&dir);
    let paths: Vec<_> = issues.iter().map(|i| i.path.file_name().unwrap().to_string_lossy().to_string()).collect();
    assert_eq!(paths, vec!["flightdeck", "settings.toml", "settings.toml.broken-x"]);
    assert!(issues[0].is_dir && issues[0].describe().contains("expected 700"));
    assert!(perms::fix(&issues).is_empty());
    assert!(perms::audit(&dir).is_empty());
    assert_eq!(std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777, 0o700);
    let _ = std::fs::remove_dir_all(&base);
}
