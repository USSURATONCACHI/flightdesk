//! Files and external programs: the native file picker, the system "Open with…" chooser and a
//! user-configured editor command. All of these block or wait on the desktop, so callers run
//! them on a background thread (see `App::spawn_bg`).

use std::path::{Path, PathBuf};

/// Native "open file" dialog for BPMN files (XDG portal on Linux, the common dialog on Windows).
/// Blocks until the user picks or cancels.
pub fn pick_bpmn_file(title: &str, start_dir: Option<&Path>) -> Option<PathBuf> {
    let mut d = rfd::FileDialog::new().set_title(title).add_filter("BPMN", &["bpmn", "xml"]).add_filter("*", &["*"]);
    if let Some(dir) = start_dir {
        d = d.set_directory(dir);
    }
    d.pick_file()
}

/// Whether the system can show an application chooser for a file.
pub fn open_with_supported() -> bool {
    cfg!(any(target_os = "linux", windows))
}

/// Show the system application chooser for `path` (the XDG OpenURI portal with `ask`, or
/// `SHOpenWithDialog` on Windows). Blocks until the chooser is dismissed.
pub fn open_with(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|e| e.to_string())?;
        rt.block_on(async move {
            let req = ashpd::desktop::open_uri::OpenFileRequest::default().ask(true);
            req.send_file(&file).await.map(|_| ()).map_err(|e| e.to_string())
        })
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::UI::Shell::{SHOpenWithDialog, OAIF_ALLOW_REGISTRATION, OAIF_EXEC, OPENASINFO};
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
        let info = OPENASINFO { pcszFile: wide.as_ptr(), pcszClass: std::ptr::null(), oaifInFlags: OAIF_ALLOW_REGISTRATION | OAIF_EXEC };
        // SAFETY: `info` and the string it points at outlive the call.
        let hr = unsafe { SHOpenWithDialog(std::ptr::null_mut(), &info) };
        if hr >= 0 {
            Ok(())
        } else {
            Err(format!("SHOpenWithDialog failed (0x{:08x})", hr as u32))
        }
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        let _ = path;
        Err("not supported on this platform".into())
    }
}

/// Split an editor command template into program and arguments, substituting `{file}`,
/// `{line}` and `{col}`. Quotes ("…" or '…') group words; there is no other shell syntax.
pub fn editor_command(template: &str, file: &Path, line: usize, col: usize) -> Result<Vec<String>, String> {
    let mut words: Vec<String> = vec![];
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    let mut started = false;
    for c in template.trim().chars() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => cur.push(c),
            (None, '"' | '\'') => {
                quote = Some(c);
                started = true;
            }
            (None, c) if c.is_whitespace() => {
                if started || !cur.is_empty() {
                    words.push(std::mem::take(&mut cur));
                    started = false;
                }
            }
            (None, c) => cur.push(c),
        }
    }
    if quote.is_some() {
        return Err("unbalanced quote in the editor command".into());
    }
    if started || !cur.is_empty() {
        words.push(cur);
    }
    if words.is_empty() {
        return Err("the editor command is empty".into());
    }
    let f = file.display().to_string();
    let has_file = words.iter().any(|w| w.contains("{file}"));
    let mut out: Vec<String> = words.into_iter().map(|w| w.replace("{file}", &f).replace("{line}", &line.to_string()).replace("{col}", &col.to_string())).collect();
    if !has_file {
        out.push(f);
    }
    Ok(out)
}

/// Start the configured editor on `file` at `line`:`col` without waiting for it.
pub fn run_editor(template: &str, file: &Path, line: usize, col: usize) -> Result<(), String> {
    let argv = editor_command(template, file, line, col)?;
    let mut child = std::process::Command::new(&argv[0]).args(&argv[1..]).stdin(std::process::Stdio::null()).spawn().map_err(|e| format!("{}: {e}", argv[0]))?;
    // Reap the child when it exits so it never lingers as a zombie.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

/// `path:line:col`, the form VS Code (`code -g`), JetBrains IDEs and most terminals understand.
pub fn location(path: &Path, line: usize, col: usize) -> String {
    format!("{}:{line}:{col}", path.display())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_templates_substitute_and_quote() {
        let f = Path::new("/tmp/a b.bpmn");
        assert_eq!(editor_command("code -g {file}:{line}:{col}", f, 12, 5).unwrap(), vec!["code", "-g", "/tmp/a b.bpmn:12:5"]);
        assert_eq!(editor_command("\"/opt/My Editor/ed\" --line {line}", f, 3, 1).unwrap(), vec!["/opt/My Editor/ed", "--line", "3", "/tmp/a b.bpmn"], "file appended when not placed");
        assert_eq!(editor_command("kate -l {line} -c {col} '{file}'", f, 7, 2).unwrap(), vec!["kate", "-l", "7", "-c", "2", "/tmp/a b.bpmn"]);
        assert!(editor_command("  ", f, 1, 1).is_err());
        assert!(editor_command("code \"{file}", f, 1, 1).is_err());
    }
}
