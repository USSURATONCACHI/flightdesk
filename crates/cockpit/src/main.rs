//! Flightdeck: a fast, keyboard-first cockpit for Camunda 7.
#![cfg_attr(windows, windows_subsystem = "windows")]

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
    on_top: bool,
    config_dir: Option<PathBuf>,
    data_dir: Option<PathBuf>,
    locale_dirs: Vec<PathBuf>,
    mcp: bool,
    mcp_port: Option<u16>,
    mcp_host: Option<String>,
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
            "--on-top" => a.on_top = true,
            "--config-dir" => a.config_dir = it.next().map(|v| paths::expand_tilde(&v)),
            "--data-dir" => a.data_dir = it.next().map(|v| paths::expand_tilde(&v)),
            "--locale-dir" => a.locale_dirs.extend(it.next().map(|v| paths::split_dir_list(&v)).unwrap_or_default()),
            "--mcp" => a.mcp = true,
            "--mcp-port" => a.mcp_port = it.next().and_then(|v| v.parse().ok()),
            "--mcp-host" => a.mcp_host = it.next(),
            "--mcp-stdio" => a.mcp_stdio = Some(it.next().unwrap_or_else(|| "http://127.0.0.1:7788/mcp".into())),
            "--size" => {
                a.size = it.next().and_then(|v| {
                    let (w, h) = v.split_once('x')?;
                    Some((w.parse().ok()?, h.parse().ok()?))
                })
            }
            "-h" | "--help" => {
                println!("flightdeck [--config-dir DIR --data-dir DIR] [--locale-dir DIR[:DIR…]] [--mcp] [--mcp-port N] [--mcp-host ADDR] [--mcp-stdio URL] [--no-focus] [--on-top] [--url URL] [--user U --password P] [--open definitions|def:<key>|instance:<id>|search:<query>] [--bpmn FILE] [--theme light|dark] [--scale F] [--size WxH] [--screenshot out.png [--screenshot-after N] [--exit-after-screenshot]]");
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

/// With the GUI subsystem Windows gives the process no console; when it was started from one,
/// attach to it so logs still appear on stdout as they do everywhere else.
#[cfg(windows)]
fn attach_parent_console() {
    use windows_sys::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
    // Safety: plain Win32 call with a documented constant; failure (no parent console) is fine.
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

fn main() -> eframe::Result<()> {
    #[cfg(windows)]
    attach_parent_console();
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
    // FLIGHTDECK_NO_FOCUS=1 / --no-focus: open the window without taking keyboard focus.
    // Windows/macOS: with_active(false). X11: the EWMH hint set in the creator below.
    // Wayland cannot decline focus, so such runs go through Xwayland when it is available.
    let no_focus = std::env::var("FLIGHTDECK_NO_FOCUS").map(|v| v != "0" && !v.is_empty()).unwrap_or(false) || args.no_focus;
    if no_focus {
        viewport = viewport.with_active(false);
    }
    // FLIGHTDECK_ON_TOP=1 / --on-top: keep the window above others. Unfocused windows are not
    // raised, and a covered window paints nothing, so screenshot runs want this.
    let on_top = std::env::var("FLIGHTDECK_ON_TOP").map(|v| v != "0" && !v.is_empty()).unwrap_or(false) || args.on_top;
    if on_top {
        viewport = viewport.with_window_level(egui::WindowLevel::AlwaysOnTop);
    }
    let force_x11 = no_focus && cockpit::platform::no_focus_prefers_x11(std::env::var("WAYLAND_DISPLAY").ok().as_deref(), std::env::var("DISPLAY").ok().as_deref());
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
    let mut options = eframe::NativeOptions { viewport, persistence_path: Some(dirs.data.join("app.ron")), ..Default::default() };
    // Remember frames the GPU skips because the window is covered: screenshots explain stalls with it.
    let surface_status = options.wgpu_options.on_surface_status.clone();
    options.wgpu_options.on_surface_status = std::sync::Arc::new(move |status| {
        if matches!(status, eframe::egui_wgpu::wgpu::CurrentSurfaceTexture::Occluded) {
            cockpit::platform::note_occluded_frame();
        }
        surface_status(status)
    });
    #[cfg(target_os = "linux")]
    if force_x11 {
        log::info!("no-focus start: using X11 (Xwayland), Wayland has no way to open a window unfocused");
        options.event_loop_builder = Some(Box::new(|b| {
            use winit::platform::x11::EventLoopBuilderExtX11;
            b.with_x11();
        }));
    }
    #[cfg(not(target_os = "linux"))]
    let _ = force_x11;
    eframe::run_native(
        "Flightdeck",
        options,
        Box::new(move |cc| {
            if no_focus {
                cockpit::platform::suppress_initial_focus(cc);
            }
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
            app.on_top = on_top;
            if let Some(h) = &args.mcp_host {
                app.mcp_host = h.clone();
            }
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
