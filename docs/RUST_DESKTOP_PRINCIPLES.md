# Engineering principles for Rust desktop apps

Distilled from building Flightdeck (an egui/eframe Camunda cockpit). Copy this file into a new
project as `CLAUDE.md` (or include it from one). Everything here is a standing instruction, not a
suggestion. Where it says "the binary", it means the thin `main.rs`; "the library" is everything
else.

## 1. Start with a design document

- Before code, write `DESIGN.md`: the problems with the thing being replaced, the user model,
  the architecture (crates/modules and their boundaries), the interaction model (every keyboard
  and mouse binding), the data flow, and what "done" looks like. Keep it updated as decisions
  change; it is the place to record *why*.
- Separate pure domain logic (parsers, query languages, geometry, state machines) from I/O
  (network clients) from UI. Each layer is its own crate or module with its own tests.

## 2. Zero warnings, fixed properly

- `cargo build` and `cargo clippy --workspace --all-targets` must be clean. Check both after
  every change, not at the end.
- Fix warnings idiomatically: restructure the code so the warning cannot arise. Do not bandage
  with `#[allow]`, `let _ =`, underscore-prefixed names, or `#[allow(dead_code)]` on things that
  are actually dead. If a lint is wrong for a specific, justified reason, the allow goes on the
  smallest scope with a comment saying why.
- Deprecated APIs are warnings too: migrate to the replacement.

## 3. Tests that check correctness, not constants

- Write tests that would fail if the logic were wrong: parsers against real fixtures, query
  languages against edge cases, geometry/hit-testing/LOD math, state transitions, loaders,
  inference from partial data, protocol handlers. Do not write tests that assert a constant
  equals itself or that a struct has a field.
- Three tiers, all headless and runnable in CI:
  1. unit tests next to the code for pure logic;
  2. end-to-end tests against a **fake server** (e.g. a tiny in-process HTTP server returning
     canned JSON) exercising the real client, state and UI code paths;
  3. UI smoke tests with a headless harness (`egui_kittest` for egui) that drive the real app
     struct: open places, press keys, type, toggle settings, and assert on state. Animations
     can be tested offline by running frames with an explicit `RawInput.time`.
- Tests never touch the user's real configuration, data, or servers. The app struct takes its
  settings path as `Option<PathBuf>`; tests pass `None` or a temp dir. Scripts that launch the
  binary pass scratch directories (see §4).
- When a bug is reported, first reproduce it (headless if possible), then fix, then keep the
  reproduction as a test.

## 4. No default paths inside the library

- Library code never computes "the config dir" or "the data dir". Every directory is a value
  handed in from above (a `Dirs { config, data }` struct or similar).
- Only `main` resolves locations, in this order: command-line flags (`--config-dir`,
  `--data-dir`), then environment variables (`<APP>_CONFIG_DIR`, `<APP>_DATA_DIR`), then the
  platform defaults (`directories::ProjectDirs`).
- All-or-nothing rule: if there is more than one directory and only some are given, refuse to
  start with a clear error naming the missing one. All given: use them. None given: defaults.
  An empty env var counts as unset. Put this rule in a small function with unit tests.
- The crash log, settings file, persisted UI state (eframe `persistence_path`) and any caches
  all derive from that one resolved struct. Create the data directory yourself; frameworks may
  not create parents of custom paths.
- Dev/test launcher scripts always export the scratch pair so a screenshot run can never write
  a profile into the real settings file.

## 5. Settings that survive updates

- Load settings field by field: unknown fields are ignored, missing fields take defaults, a
  single malformed field does not discard the file. Never all-or-nothing deserialisation of a
  user's file. Test loading an older and a newer schema.
- Save atomically (write temp file, fsync, rename). Before overwriting a file that failed to
  parse, keep a `.broken-<timestamp>` copy. Never silently reset to defaults.
- Secrets (saved passwords) mean private permissions: directory 700, files 600, created that
  way from the start. On startup audit the permissions and, if wrong, show a warning with a
  one-click "fix" action rather than fixing silently or ignoring it.
- Do not persist values that came from command-line flags meant for a single run (`--theme`,
  `--url`); keep per-run overrides in separate fields.
- Upsert by identity, not by name: when a successful action should update a saved entry, pass
  the index/slot of the entry being acted on instead of searching by a field that may collide.

## 6. Platform integration done properly

- Detect the platform from the window system handle (`raw-window-handle`), never from
  environment variable sniffing.
- Use the platform's real mechanism for each feature (e.g. `wp_pointer_warp_v1` on Wayland,
  `CursorPosition` on X11/Windows/macOS). If a platform has no mechanism, the feature is
  simply off there; no emulation, no hidden or locked cursor, no fake pointer. Say so in the UI
  tooltip.
- Window icon: embed a raster for X11/Windows/macOS via the windowing library, and ship a
  `.desktop` entry plus hicolor icons for Wayland (compositors resolve the app id to the desktop
  entry). Keep the app id, binary name, desktop file name and icon name identical.
- Clipboard and link opening go through the OS (enable the framework features); never an
  isolated in-app clipboard.
- Theme: light, dark and "system" (default). Detect the system preference through the XDG
  portal / gsettings on Linux and the OS API elsewhere; follow changes at runtime.
- Startup controls for automation: an env var to open the window without taking focus and
  one to position it; test scripts always use them.

## 7. UX rules for canvas-style apps

- Camera: smooth, inertial, zoom-to-cursor, animated in log-zoom space, unlimited zoom range
  with level-of-detail thresholds so performance and legibility hold at every scale.
- Text must be zoom-stable: compute line breaks in world units and scale glyph meshes
  geometrically (quantised to a few bands) so labels neither flicker nor re-wrap while zooming.
- Everything reachable by keyboard and by mouse. Modifiers that change a gesture (e.g. Alt =
  twice the speed) get a small visible affordance near the cursor while held.
- Infinite drags (pan, dials) wrap the pointer at the *view* edge, with the real cursor visible
  (Okular style), only where the platform supports it (§6).
- Live data must not flash: when polled data changes, highlights and markers ease in and out
  (~200 ms) rather than toggling. Derived views (search results, filters) refresh on the same
  beat as the data they derive from; background refreshes never show a spinner, only the first
  load of a query does.
- Colour carries meaning: show a compact legend listing only the colours currently in play and
  explain in the hover tooltip exactly why an element is marked (which rule, which record).
- Layout must fit: measure content and size containers to it; do not rely on panels that lay
  out against their maximum width. Tables get draggable column widths and copy buttons for
  identifiers (paint a real copy icon; do not rely on Unicode glyphs that look like something
  else or may be missing from the font).
- Navigation never destroys context: opening settings or a secondary place opens a tab or
  reuses an existing one; it does not replace what the user was looking at. Destructive
  actions confirm first.

## 8. Agent-friendly from day one

- Build an MCP server into the app (toggleable in the UI): tools for the domain operations
  plus `ui_*` tools that read full UI state, navigate, select, type, press keys, run commands and
  take screenshots. Queue tool calls onto the UI thread; return "retry later" while data loads.
  Ship `.mcp.json` and a skill file so a new agent session can drive the app immediately.
- Provide a screenshot script that launches an isolated instance (scratch dirs, no focus, on a
  secondary monitor, fixed size, exits after N frames) so visual checks are one command.
- Never kill processes by name (`pkill -x <app>`): the dev build and the installed binary share
  a name and the user may be running one. Launch test instances with a unique port, record the
  PID, kill that PID.
- Never point test runs at the user's real backend; use a scratch instance with seeded
  fixtures.

## 9. Packaging

- A real install has three parts: binary in `/usr/bin`, `.desktop` in
  `/usr/share/applications`, icon in `/usr/share/icons/hicolor` (scalable SVG plus rendered
  PNGs). Keep the SVG master in `assets/` with a script to re-render the embedded raster.
- Ship a distro package definition (Arch `PKGBUILD` in `packaging/arch/`) that builds from the
  working copy, derives the version from `Cargo.toml` + git, runs `cargo test` in `check()`, and
  lists dependencies explicitly. GUI stacks dlopen almost everything, so `ldd` is misleading:
  read the `.so` names out of the binary (`strings`) to find run-time deps; list GPU drivers,
  portals and `xdg-utils` as optional.
- `.gitignore` the package build artefacts (`src/`, `pkg/`, `*.pkg.tar*`, logs).
- Ship a generic Linux archive too: binary, `locales/`, desktop entry, icons, an `install`
  script (plain `install(1)` commands into `$PREFIX`, relative paths, no comments) and a
  `run-in-place` script that points the config/data/locale variables at the archive folder.
- Windows from Linux: cross-compile with the GNU target, GUI subsystem plus `AttachConsole` so
  stdout logging survives, `.ico` rendered from the SVG, MSI built with `wixl`. Offer both a
  self-contained MSI and a portable zip whose small `flightdeck-setup.msi` installs from the
  files beside it: that needs the files marked noncompressed, the install root mapped to the
  MSI's folder (`Name:.`) and the "compressed" bit cleared in the summary stream.
- Verify Windows artefacts twice: first under Wine in a throw-away prefix (fast; one prefix per
  installer, since mixing installs with shared component ids leaves files behind), then in a
  real Windows VM driven through the app's own MCP server with the port forwarded to the host.
- Keep VM disks, ISO copies and multi-gigabyte extracts off the system volume. A sustained
  write load on a btrfs/LUKS NVMe at 90% full stalled the whole desktop; use a separate disk,
  a raw no-CoW image opened with direct I/O, a bandwidth cap and idle I/O priority for QEMU.
- Include a `LICENSE` file matching `Cargo.toml`.

## 10. How to work

- Investigate before changing: when a user reports "X looks wrong", first check the data (via
  the MCP tools or the backend) to see whether the display is correct and the model is
  surprising; explain that distinction honestly.
- Fix classes, not instances. When a bug is a *shape* (a lock taken inside another lock's
  closure, a plain key check shadowing a modified one, a default path in library code), grep
  for the shape everywhere and add a test that scans the source for it, so the next occurrence
  fails in CI instead of hanging for a user. Example: egui's `Context` is one `RwLock`; any
  `ctx.input`/`data_mut`/`memory`/`request_repaint` call made inside the closure of another
  accessor deadlocks silently in release builds. Read values first, then take the lock.
- Reproduce, fix at the root cause, add the test, run build + clippy + full tests, then commit
  with a message that says what changed and why. Small, frequent commits.
- Bump the semantic version (one place: the workspace `Cargo.toml`) in the same commit as any
  change users can notice: patch for fixes, minor for new features or new tools/flags (before
  1.0 also for incompatible changes), major for incompatible changes after 1.0. Docs-, test- and
  CI-only commits leave it alone. Never `r`-suffixed or date versions; never left pinned.
- Report outcomes faithfully: say what was verified and how (test, screenshot, live check),
  and say plainly what was not verified or was left out.
- When the user corrects an approach, record the rule where future sessions will see it
  (`CLAUDE.md` / memory) so it never needs re-explaining.
