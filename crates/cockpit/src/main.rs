//! Flightdeck: a fast, keyboard-first cockpit for Camunda 7.

use cockpit::{app, mcp, paths, settings, state, theme};

use std::path::PathBuf;

#[derive(Default, Debug)]
struct Args {
    url: Option<String>,
    user: Option<String>,
    password: Option<String>,
    open: Vec<String>,
    bpmn: Option<PathBuf>,
    screenshot: Option<PathBuf>,
    screenshot_after: u64,
    exit_after_screenshot: bool,
    scale: f32,
    theme: Option<String>,
    size: Option<(f32, f32)>,
    dock: Option<usize>,
    stats: bool,
    no_focus: bool,
    config_dir: Option<PathBuf>,
    data_dir: Option<PathBuf>,
    locale_dirs: Vec<PathBuf>,
    mcp: bool,
    mcp_port: Option<u16>,
    mcp_stdio: Option<String>,
}

fn parse_args() -> Args {
    let mut a = Args { scale: 1.0, screenshot_after: 12, ..Default::default() };
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        match k.as_str() {
            "--url" => a.url = it.next(),
            "--user" => a.user = it.next(),
            "--password" => a.password = it.next(),
            "--open" => a.open.extend(it.next()),
            "--bpmn" => a.bpmn = it.next().map(PathBuf::from),
            "--screenshot" => a.screenshot = it.next().map(PathBuf::from),
            "--screenshot-after" => a.screenshot_after = it.next().and_then(|v| v.parse().ok()).unwrap_or(12),
            "--exit-after-screenshot" => a.exit_after_screenshot = true,
            "--scale" => a.scale = it.next().and_then(|v| v.parse().ok()).unwrap_or(1.0),
            "--theme" => a.theme = it.next(),
            "--dock" => a.dock = it.next().and_then(|v| v.parse().ok()),
            "--stats" => a.stats = true,
            "--no-focus" => a.no_focus = true,
            "--config-dir" => a.config_dir = it.next().map(|v| paths::expand_tilde(&v)),
            "--data-dir" => a.data_dir = it.next().map(|v| paths::expand_tilde(&v)),
            "--locale-dir" => a.locale_dirs.extend(it.next().map(|v| paths::split_dir_list(&v)).unwrap_or_default()),
            "--mcp" => a.mcp = true,
            "--mcp-port" => a.mcp_port = it.next().and_then(|v| v.parse().ok()),
            "--mcp-stdio" => a.mcp_stdio = Some(it.next().unwrap_or_else(|| "http://127.0.0.1:7788/mcp".into())),
            "--size" => {
                a.size = it.next().and_then(|v| {
                    let (w, h) = v.split_once('x')?;
                    Some((w.parse().ok()?, h.parse().ok()?))
                })
            }
            "-h" | "--help" => {
                println!("flightdeck [--config-dir DIR --data-dir DIR] [--locale-dir DIR[:DIR…]] [--mcp] [--mcp-port N] [--mcp-stdio URL] [--no-focus] [--url URL] [--user U --password P] [--open definitions|def:<key>|instance:<id>|search:<query>] [--bpmn FILE] [--theme light|dark] [--scale F] [--size WxH] [--screenshot out.png [--screenshot-after N] [--exit-after-screenshot]]");
                std::process::exit(0);
            }
            other => {
                if other.ends_with(".bpmn") {
                    a.bpmn = Some(PathBuf::from(other));
                } else {
                    eprintln!("unknown argument {other}");
                }
            }
        }
    }
    a
}

/// Plain stdout logging; redirect it yourself when you want a file. `RUST_LOG` sets the level.
fn init_logging(dirs: &paths::Dirs) {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info,wgpu=warn,naga=warn,eframe=warn,egui_wgpu=warn")).target(env_logger::Target::Stdout).init();
    log::info!("flightdeck {} · config {} · data {}", env!("CARGO_PKG_VERSION"), dirs.config.display(), dirs.data.display());
}

/// Write panics to `<config dir>/crash.log` as well as stderr, so a crash in a windowed session
/// still leaves a backtrace behind.
fn install_crash_log(path: Option<PathBuf>) {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let bt = std::backtrace::Backtrace::force_capture();
        let text = format!("[{}] {info}\n{bt}\n\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
        if let Some(p) = &path {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
                let _ = f.write_all(text.as_bytes());
            }
            eprintln!("flightdeck: panic written to {}", p.display());
        }
        default_hook(info);
    }));
}

fn main() -> eframe::Result<()> {
    let args = parse_args();
    // Directories: --config-dir/--data-dir, else FLIGHTDECK_CONFIG_DIR/FLIGHTDECK_DATA_DIR, else
    // the platform defaults. A partial pair is refused (see `paths::resolve`).
    let env_dir = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty()).map(|v| paths::expand_tilde(&v));
    let dirs = match paths::resolve(args.config_dir.clone().or_else(|| env_dir(paths::ENV_CONFIG_DIR)), args.data_dir.clone().or_else(|| env_dir(paths::ENV_DATA_DIR)), paths::platform_defaults) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("flightdeck: {e}");
            std::process::exit(2);
        }
    };
    init_logging(&dirs);
    install_crash_log(Some(dirs.crash_log()));
    // Language packs: --locale-dir / FLIGHTDECK_LOCALE_DIR replace the default search path.
    let locale_dirs = if !args.locale_dirs.is_empty() {
        args.locale_dirs.clone()
    } else if let Some(list) = std::env::var(paths::ENV_LOCALE_DIR).ok().filter(|v| !v.trim().is_empty()) {
        paths::split_dir_list(&list)
    } else {
        paths::default_locale_dirs(&dirs, std::env::var("XDG_DATA_DIRS").ok().as_deref(), std::env::current_exe().ok().as_deref())
    };
    if let Some(url) = &args.mcp_stdio {
        // stdio ↔ HTTP bridge for MCP clients that only speak stdio.
        mcp::stdio_bridge(url);
        return Ok(());
    }
    let settings = settings::Settings::load_from(&dirs.settings_file());
    // `--theme` applies to this run only; it is never written to the settings file.
    let theme_override = args.theme.as_deref().map(|t| match t {
        "light" => theme::ThemeMode::Light,
        "dark" => theme::ThemeMode::Dark,
        _ => theme::ThemeMode::System,
    });
    if theme_override.unwrap_or(settings.theme) == theme::ThemeMode::System {
        theme::system::probe_now();
    }
    let (w, h) = args.size.unwrap_or((1440.0, 900.0));
    let mut viewport = egui::ViewportBuilder::default().with_title("Flightdeck").with_inner_size([w, h]).with_min_inner_size([800.0, 500.0]).with_app_id("flightdeck");
    if let Some(icon) = cockpit::icon::window_icon() {
        viewport = viewport.with_icon(icon);
    }
    // FLIGHTDECK_NO_FOCUS=1: create the window without activating it (no focus stealing).
    if std::env::var("FLIGHTDECK_NO_FOCUS").map(|v| v != "0" && !v.is_empty()).unwrap_or(false) || args.no_focus {
        viewport = viewport.with_active(false);
    }
    // FLIGHTDECK_WINDOW_POS=x,y: initial window position (honoured on X11/Windows/macOS; Wayland ignores it).
    if let Ok(pos) = std::env::var("FLIGHTDECK_WINDOW_POS") {
        if let Some((x, y)) = pos.split_once(',').and_then(|(x, y)| Some((x.trim().parse::<f32>().ok()?, y.trim().parse::<f32>().ok()?))) {
            viewport = viewport.with_position([x, y]);
        }
    }
    // eframe does not create the parent of a custom persistence path.
    if let Err(e) = std::fs::create_dir_all(&dirs.data) {
        log::warn!("cannot create data dir {}: {e}", dirs.data.display());
    }
    let options = eframe::NativeOptions { viewport, persistence_path: Some(dirs.data.join("app.ron")), ..Default::default() };
    eframe::run_native(
        "Flightdeck",
        options,
        Box::new(move |cc| {
            cockpit::platform::init(cc, cc);
            let mut app = app::App::new(&cc.egui_ctx, settings, args.scale);
            app.settings_path = Some(dirs.settings_file());
            app.locale_dirs = locale_dirs.clone();
            app.apply_language();
            if theme_override.is_some() {
                app.theme_override = theme_override;
                app.frame_theme_dirty = true;
            }
            app.set_ctx(&cc.egui_ctx);
            if let Some(p) = &args.screenshot {
                app.screenshot = Some(app::ScreenshotPlan { path: p.clone(), after_frames: args.screenshot_after, exit: args.exit_after_screenshot, requested: false });
            }
            if let Some(b) = &args.bpmn {
                app.offline = true;
                app.navigate(state::Nav::File { path: b.clone() }, state::NavMode::NewTab);
            }
            if let Some(url) = &args.url {
                let normalized = camunda::normalize_base_url(url);
                let host = normalized.trim_start_matches("http://").trim_start_matches("https://").split('/').next().unwrap_or("engine").to_string();
                let mut p = settings::EngineProfile { name: host, url: normalized, user: args.user.clone().unwrap_or_default(), password: args.password.clone().unwrap_or_default(), ..Default::default() };
                if let Some(existing) = app.settings.engines.iter().find(|e| e.url == p.url) {
                    p.name = existing.name.clone();
                }
                app.connect.connecting = true;
                app.connect_to(p);
            } else if app.settings.auto_connect && args.bpmn.is_none() {
                let p = app.settings.engine();
                app.connect.connecting = true;
                app.connect_to(p);
            }
            app.startup_dock = args.dock;
            app.stats = args.stats;
            if let Some(p) = args.mcp_port {
                app.settings.mcp_port = p;
            }
            if args.mcp || app.settings.mcp_enabled {
                app.start_mcp();
            }
            for spec in &args.open {
                let nav = if spec == "definitions" {
                    state::Nav::Definitions
                } else if let Some(k) = spec.strip_prefix("def:") {
                    state::Nav::DefinitionKey { key: k.to_string() }
                } else if let Some(i) = spec.strip_prefix("instance:") {
                    state::Nav::Instance { id: i.to_string() }
                } else if let Some(q) = spec.strip_prefix("search:") {
                    state::Nav::Search { query: q.to_string() }
                } else if spec == "incidents" {
                    state::Nav::Incidents
                } else {
                    continue;
                };
                app.startup_nav.push(nav);
            }
            Ok(Box::new(app))
        }),
    )
}
