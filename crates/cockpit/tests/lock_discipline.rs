//! egui's `Context` is a single `RwLock`. Calling any `Context` accessor (`input`, `data_mut`,
//! `memory`, `request_repaint`, …) from inside the closure of another accessor re-locks it on
//! the same thread: a panic after 10 s in debug builds, a silent hang in release builds. This
//! test scans the source for that shape so the whole class of hangs is caught at test time,
//! not by a user clicking a button.

use std::path::{Path, PathBuf};

/// Accessors that take the context lock and accept a closure.
const OPENERS: &[&str] = &["input", "input_mut", "data", "data_mut", "memory", "memory_mut", "output_mut", "fonts", "style_mut", "options_mut", "graphics_mut", "viewport", "viewport_for", "input_for", "memory_for"];
/// Calls that must not appear inside such a closure (each takes the context lock again).
const INNER: &[&str] = &[
    "input", "input_mut", "data", "data_mut", "memory", "memory_mut", "output_mut", "fonts", "style_mut", "options_mut", "set_style", "set_visuals", "set_fonts", "request_repaint", "request_repaint_after", "set_cursor_icon", "copy_text", "open_url", "animate_bool_with_time", "animate_value_with_time", "screen_rect", "pixels_per_point", "is_pointer_over_area", "wants_keyboard_input", "wants_pointer_input", "send_viewport_cmd", "load_texture", "debug_painter", "layer_painter", "set_zoom_factor",
];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().and_then(|x| x.to_str()) == Some("rs") {
            out.push(p);
        }
    }
}

/// `text[start]` is `(`; returns the index of the matching `)`, skipping string literals roughly.
fn matching_paren(text: &[u8], start: usize) -> usize {
    let (mut depth, mut i, mut in_str) = (0usize, start, false);
    while i < text.len() {
        let c = text[i];
        if in_str {
            if c == b'\\' {
                i += 1;
            } else if c == b'"' {
                in_str = false;
            }
        } else if c == b'"' {
            in_str = true;
        } else if c == b'(' {
            depth += 1;
        } else if c == b')' {
            depth -= 1;
            if depth == 0 {
                return i;
            }
        }
        i += 1;
    }
    text.len() - 1
}

/// Blank out the contents of string literals so text never counts as code.
fn strip_strings(code: &str) -> String {
    let mut out = String::with_capacity(code.len());
    let mut in_str = false;
    let mut chars = code.chars().peekable();
    while let Some(c) = chars.next() {
        if in_str {
            if c == '\\' {
                chars.next();
            } else if c == '"' {
                in_str = false;
                out.push('"');
            }
        } else {
            if c == '"' {
                in_str = true;
            }
            out.push(c);
        }
    }
    out
}

fn offenders(src: &str) -> Vec<(usize, String)> {
    let bytes = src.as_bytes();
    let mut out = vec![];
    for opener in OPENERS {
        let pat = format!(".{opener}(");
        let mut from = 0;
        while let Some(pos) = src[from..].find(&pat) {
            let call = from + pos;
            let paren = call + pat.len() - 1;
            // Only closures: `.input(|i| …)` / `.data_mut(|d| …)`.
            let after = src[paren + 1..].trim_start();
            if !after.starts_with('|') {
                from = paren + 1;
                continue;
            }
            let end = matching_paren(bytes, paren);
            let body = strip_strings(&src[paren + 1..end]);
            for inner in INNER {
                for recv in ["ctx.", "ui.", "ui.ctx().", "self.ctx.", "egui_ctx.", "ctx2."] {
                    let needle = format!("{recv}{inner}(");
                    if body.contains(&needle) {
                        let line = src[..call].matches('\n').count() + 1;
                        out.push((line, format!(".{opener}(|…| … {needle}…)")));
                    }
                }
            }
            from = paren + 1;
        }
    }
    out.sort();
    out.dedup();
    out
}

#[test]
fn no_context_accessor_is_called_inside_another_context_closure() {
    let mut files = vec![];
    rust_files(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src")), &mut files);
    let mut report = vec![];
    for f in &files {
        let src = std::fs::read_to_string(f).unwrap();
        for (line, what) in offenders(&src) {
            report.push(format!("{}:{line}: {what}", f.strip_prefix(env!("CARGO_MANIFEST_DIR")).unwrap_or(f).display()));
        }
    }
    assert!(report.is_empty(), "nested egui Context locks (hang in release builds):\n{}", report.join("\n"));
    assert!(files.len() > 20, "scanner saw only {} files", files.len());
}

#[test]
fn scanner_catches_the_shape_that_hung_the_app() {
    let bad = r#"
        if resp.clicked() {
            ui.ctx().data_mut(|d| d.insert_temp(flash_id, ui.input(|i| i.time)));
        }
    "#;
    assert_eq!(offenders(bad).len(), 1, "{:?}", offenders(bad));
    let bad_multiline = "ctx.input(|i| {\n    let t = i.time;\n    ctx.request_repaint();\n    t\n})";
    assert_eq!(offenders(bad_multiline).len(), 1);
    let good = "let now = ui.input(|i| i.time);\nui.ctx().data_mut(|d| d.insert_temp(id, now));\nlet s = ctx.input(|i| i.modifiers.alt);";
    assert!(offenders(good).is_empty(), "{:?}", offenders(good));
    // Strings mentioning the pattern are not code.
    let in_string = r#"ui.input(|i| { let _ = "ctx.input("; i.time })"#;
    assert!(offenders(in_string).is_empty());
}
