//! A small Markdown subset for documentation and descriptions: paragraphs that keep their line
//! breaks, headings, bullet and numbered lists, quotes, code blocks, horizontal rules and inline
//! **bold**, *italic*, `code`, ~~strike~~ and links. Plain text renders as itself, so text that
//! was never meant as Markdown loses nothing: underscores inside words (`ORDER_DESK_EU`) and
//! lone asterisks stay literal.

use egui::{RichText, Ui};

/// Inline formatting of one run of text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub strong: bool,
    pub em: bool,
    pub code: bool,
    pub strike: bool,
    pub link: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    Heading(u8, Vec<Span>),
    /// Consecutive text lines; each inner vec is one line (line breaks are kept).
    Para(Vec<Vec<Span>>),
    /// List items: (nesting level, marker such as "•" or "3.", lines of the item).
    List(Vec<(usize, String, Vec<Vec<Span>>)>),
    Quote(Vec<Vec<Span>>),
    Code(String),
    Rule,
}

/// Parse `text` into blocks.
pub fn parse(text: &str) -> Vec<Block> {
    let mut blocks: Vec<Block> = vec![];
    let mut lines = text.lines().peekable();
    while let Some(raw) = lines.next() {
        let line = raw.trim_end();
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        if trimmed.is_empty() {
            // A blank line ends paragraphs, quotes and lists.
            blocks.push(Block::Para(vec![]));
            continue;
        }
        if let Some(fence) = ["```", "~~~"].into_iter().find(|f| trimmed.starts_with(f)) {
            let mut code = vec![];
            for l in lines.by_ref() {
                if l.trim_start().starts_with(fence) {
                    break;
                }
                code.push(l.strip_prefix(&" ".repeat(indent)).unwrap_or(l));
            }
            blocks.push(Block::Code(code.join("\n")));
            continue;
        }
        if let Some(level) = heading_level(trimmed) {
            let body = trimmed[level as usize..].trim().trim_end_matches('#').trim_end();
            blocks.push(Block::Heading(level, inline(body)));
            continue;
        }
        if is_rule(trimmed) {
            blocks.push(Block::Rule);
            continue;
        }
        if let Some(q) = trimmed.strip_prefix('>') {
            let q = q.strip_prefix(' ').unwrap_or(q);
            match blocks.last_mut() {
                Some(Block::Quote(ls)) => ls.push(inline(q)),
                _ => blocks.push(Block::Quote(vec![inline(q)])),
            }
            continue;
        }
        if let Some((marker, body)) = list_marker(trimmed) {
            let level = indent / 2;
            let item = (level, marker, vec![inline(body)]);
            match blocks.last_mut() {
                Some(Block::List(items)) => items.push(item),
                _ => blocks.push(Block::List(vec![item])),
            }
            continue;
        }
        // Indented text right after a list item continues that item.
        if indent >= 2 {
            if let Some(Block::List(items)) = blocks.last_mut() {
                if let Some(last) = items.last_mut() {
                    last.2.push(inline(trimmed));
                    continue;
                }
            }
        }
        match blocks.last_mut() {
            Some(Block::Para(ls)) if !ls.is_empty() => ls.push(inline(trimmed)),
            _ => blocks.push(Block::Para(vec![inline(trimmed)])),
        }
    }
    // Blank-line markers were only separators.
    blocks.retain(|b| !matches!(b, Block::Para(ls) if ls.is_empty()));
    blocks
}

fn heading_level(t: &str) -> Option<u8> {
    let n = t.bytes().take_while(|&b| b == b'#').count();
    ((1..=6).contains(&n) && t[n..].starts_with(' ')).then_some(n as u8)
}

fn is_rule(t: &str) -> bool {
    let compact: String = t.chars().filter(|c| !c.is_whitespace()).collect();
    compact.len() >= 3 && ["-", "*", "_"].iter().any(|c| compact.chars().all(|x| x.to_string() == *c))
}

fn list_marker(t: &str) -> Option<(String, &str)> {
    for b in ["- ", "* ", "+ ", "• "] {
        if let Some(rest) = t.strip_prefix(b) {
            return Some(("•".into(), rest));
        }
    }
    let digits = t.bytes().take_while(|b| b.is_ascii_digit()).count();
    if (1..=9).contains(&digits) {
        let rest = &t[digits..];
        for d in [". ", ") "] {
            if let Some(body) = rest.strip_prefix(d) {
                return Some((format!("{}.", &t[..digits]), body));
            }
        }
    }
    None
}

/// Inline formatting of one line.
pub fn inline(s: &str) -> Vec<Span> {
    let chars: Vec<char> = s.chars().collect();
    let mut out: Vec<Span> = vec![];
    let mut cur = Span::default();
    let mut style = Span::default();
    let mut i = 0;
    let flush = |cur: &mut Span, out: &mut Vec<Span>, style: &Span| {
        if !cur.text.is_empty() {
            out.push(std::mem::take(cur));
        }
        *cur = Span { text: String::new(), ..style.clone() };
    };
    while i < chars.len() {
        let c = chars[i];
        // Escapes.
        if c == '\\' && i + 1 < chars.len() && chars[i + 1].is_ascii_punctuation() {
            cur.text.push(chars[i + 1]);
            i += 2;
            continue;
        }
        // Inline code: no formatting inside.
        if c == '`' {
            if let Some(end) = (i + 1..chars.len()).find(|&j| chars[j] == '`') {
                flush(&mut cur, &mut out, &style);
                out.push(Span { text: chars[i + 1..end].iter().collect(), code: true, ..Default::default() });
                i = end + 1;
                continue;
            }
        }
        // [text](url)
        if c == '[' {
            if let Some((text, url, next)) = link_at(&chars, i) {
                flush(&mut cur, &mut out, &style);
                out.push(Span { text, link: Some(url), ..style.clone() });
                i = next;
                continue;
            }
        }
        // Bare URLs.
        if (c == 'h') && (starts_with(&chars, i, "https://") || starts_with(&chars, i, "http://")) && (i == 0 || !chars[i - 1].is_alphanumeric()) {
            let mut end = i;
            while end < chars.len() && !chars[end].is_whitespace() && !matches!(chars[end], '<' | '>' | '"' | '»' | '«') {
                end += 1;
            }
            while end > i && matches!(chars[end - 1], '.' | ',' | ';' | ':' | '!' | '?' | ')' | '\'') {
                end -= 1;
            }
            let url: String = chars[i..end].iter().collect();
            flush(&mut cur, &mut out, &style);
            out.push(Span { text: url.clone(), link: Some(url), ..style.clone() });
            i = end;
            continue;
        }
        // Emphasis delimiters: ** __ ~~ (two chars) or * _ (one char).
        if matches!(c, '*' | '_' | '~') {
            let double = i + 1 < chars.len() && chars[i + 1] == c;
            let len = if double { 2 } else { 1 };
            if c == '~' && !double {
                cur.text.push(c);
                i += 1;
                continue;
            }
            let open = match (c, double) {
                ('~', _) => style.strike,
                (_, true) => style.strong,
                _ => style.em,
            };
            let ok = if open { can_close(&chars, i, len, c) } else { can_open(&chars, i, len, c) && has_closer(&chars, i + len, len, c) };
            if ok {
                flush(&mut cur, &mut out, &style);
                match (c, double) {
                    ('~', _) => style.strike = !open,
                    (_, true) => style.strong = !open,
                    _ => style.em = !open,
                }
                cur = Span { text: String::new(), ..style.clone() };
                i += len;
                continue;
            }
            for _ in 0..len {
                cur.text.push(c);
            }
            i += len;
            continue;
        }
        cur.text.push(c);
        i += 1;
    }
    flush(&mut cur, &mut out, &style);
    out
}

fn starts_with(chars: &[char], i: usize, pat: &str) -> bool {
    let p: Vec<char> = pat.chars().collect();
    chars.len() >= i + p.len() && chars[i..i + p.len()] == p[..]
}

fn link_at(chars: &[char], i: usize) -> Option<(String, String, usize)> {
    let close = (i + 1..chars.len()).find(|&j| chars[j] == ']')?;
    if chars.get(close + 1) != Some(&'(') {
        return None;
    }
    let end = (close + 2..chars.len()).find(|&j| chars[j] == ')')?;
    let text: String = chars[i + 1..close].iter().collect();
    let url: String = chars[close + 2..end].iter().collect::<String>().trim().to_string();
    if text.is_empty() || url.is_empty() || url.contains(' ') {
        return None;
    }
    Some((text, url, end + 1))
}

/// Opening delimiter: followed by a non-space; `_` must not sit inside a word.
fn can_open(chars: &[char], i: usize, len: usize, c: char) -> bool {
    let next = chars.get(i + len);
    let prev = i.checked_sub(1).and_then(|p| chars.get(p));
    next.is_some_and(|n| !n.is_whitespace() && *n != c) && (c != '_' || !prev.is_some_and(|p| p.is_alphanumeric()))
}

/// Closing delimiter: preceded by a non-space; `_` must not be followed by a word character.
fn can_close(chars: &[char], i: usize, len: usize, c: char) -> bool {
    let prev = i.checked_sub(1).and_then(|p| chars.get(p));
    let next = chars.get(i + len);
    prev.is_some_and(|p| !p.is_whitespace()) && (c != '_' || !next.is_some_and(|n| n.is_alphanumeric()))
}

fn has_closer(chars: &[char], from: usize, len: usize, c: char) -> bool {
    let mut j = from;
    while j + len <= chars.len() {
        let run = chars[j..].iter().take_while(|&&x| x == c).count();
        if run >= len && (run == len || len == 2) && can_close(chars, j, len, c) {
            return true;
        }
        j += run.max(1);
    }
    false
}

// ---------------------------------------------------------------------------------------------
// Rendering

/// Render Markdown-lite `text` into `ui` at the current width.
pub fn show(ui: &mut Ui, text: &str) {
    let blocks = parse(text);
    let weak = ui.visuals().weak_text_color();
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 3.0;
        for (n, b) in blocks.iter().enumerate() {
            if n > 0 {
                ui.add_space(4.0);
            }
            match b {
                Block::Heading(level, spans) => {
                    let size = match level {
                        1 => 17.0,
                        2 => 15.5,
                        _ => 14.0,
                    };
                    line(ui, spans, Some(size), true);
                }
                Block::Para(lines) => {
                    for l in lines {
                        line(ui, l, None, false);
                    }
                }
                Block::List(items) => {
                    for (level, marker, lines) in items {
                        ui.horizontal_top(|ui| {
                            ui.add_space(4.0 + *level as f32 * 16.0);
                            let w = if marker == "•" { 14.0 } else { 22.0 };
                            ui.add_sized([w, 18.0], egui::Label::new(RichText::new(marker).color(weak).size(if marker == "•" { 15.0 } else { 13.0 })));
                            ui.vertical(|ui| {
                                for l in lines {
                                    line(ui, l, None, false);
                                }
                            });
                        });
                    }
                }
                Block::Quote(lines) => {
                    let r = ui.horizontal_top(|ui| {
                        ui.add_space(10.0);
                        ui.vertical(|ui| {
                            for l in lines {
                                line_colored(ui, l, weak);
                            }
                        });
                    });
                    let rr = r.response.rect;
                    ui.painter().vline(rr.min.x + 2.0, rr.y_range(), egui::Stroke::new(3.0, weak.gamma_multiply(0.5)));
                }
                Block::Code(code) => {
                    egui::Frame::new().fill(ui.visuals().extreme_bg_color).corner_radius(4.0).inner_margin(6.0).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.add(egui::Label::new(RichText::new(code).monospace().size(11.5)).wrap().selectable(true));
                    });
                }
                Block::Rule => {
                    ui.separator();
                }
            }
        }
    });
}

fn line(ui: &mut Ui, spans: &[Span], size: Option<f32>, strong: bool) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for s in spans {
            let mut t = RichText::new(&s.text);
            if let Some(sz) = size {
                t = t.size(sz);
            }
            if s.strong || strong {
                t = t.strong().family(crate::theme::bold_family());
            }
            if s.em {
                t = t.italics();
            }
            if s.strike {
                t = t.strikethrough();
            }
            if s.code {
                t = t.code();
            }
            match &s.link {
                Some(url) => {
                    // The theme overrides text colour globally, so links get theirs explicitly.
                    let c = ui.visuals().hyperlink_color;
                    ui.hyperlink_to(t.color(c), url).on_hover_text(url);
                }
                None => {
                    ui.add(egui::Label::new(t).wrap().selectable(true));
                }
            }
        }
    });
}

fn line_colored(ui: &mut Ui, spans: &[Span], color: egui::Color32) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for s in spans {
            let mut t = RichText::new(&s.text).color(color);
            if s.strong {
                t = t.family(crate::theme::bold_family());
            }
            if s.em {
                t = t.italics();
            }
            if s.code {
                t = t.code();
            }
            match &s.link {
                Some(url) => {
                    let c = ui.visuals().hyperlink_color;
                    ui.hyperlink_to(t.color(c), url).on_hover_text(url);
                }
                None => {
                    ui.add(egui::Label::new(t).wrap().selectable(true));
                }
            }
        }
    });
}

/// Rough count of display lines, used to decide whether to fold long documentation.
pub fn weight(text: &str) -> usize {
    text.lines().map(|l| 1 + l.chars().count() / 70).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(spans: &[Span]) -> String {
        spans.iter().map(|s| s.text.as_str()).collect()
    }

    #[test]
    fn plain_text_survives_unchanged_including_identifier_underscores() {
        let t = "Role codes ORDER_DESK_EU and SUPER_USER; 2 * 3 = 6; snake_case_name.";
        let b = parse(t);
        assert_eq!(b.len(), 1);
        let Block::Para(lines) = &b[0] else { panic!("{b:?}") };
        assert_eq!(plain(&lines[0]), t);
        assert!(lines[0].iter().all(|s| !s.em && !s.strong), "{lines:?}");
    }

    #[test]
    fn line_breaks_are_kept_and_blank_lines_split_paragraphs() {
        let b = parse("first line\nsecond line\n\nnext paragraph");
        assert_eq!(b.len(), 2);
        let Block::Para(lines) = &b[0] else { panic!() };
        assert_eq!(lines.len(), 2);
        assert_eq!(plain(&lines[1]), "second line");
    }

    #[test]
    fn emphasis_code_strike_and_links() {
        let s = inline("a **bold** and *it* and _em_ with `x_y*z` ~~old~~ [docs](https://example.org/a) see https://example.org/b.");
        let find = |t: &str| s.iter().find(|x| x.text == t).unwrap_or_else(|| panic!("{t} missing in {s:?}")).clone();
        assert!(find("bold").strong);
        assert!(find("it").em);
        assert!(find("em").em);
        assert!(find("x_y*z").code);
        assert!(find("old").strike);
        assert_eq!(find("docs").link.as_deref(), Some("https://example.org/a"));
        assert_eq!(find("https://example.org/b").link.as_deref(), Some("https://example.org/b"), "trailing dot is not part of the url");
    }

    #[test]
    fn unmatched_delimiters_stay_literal() {
        assert_eq!(plain(&inline("**not closed")), "**not closed");
        assert!(inline("**not closed").iter().all(|s| !s.strong));
        assert_eq!(plain(&inline("price * qty")), "price * qty");
        assert_eq!(plain(&inline("\\*escaped\\*")), "*escaped*");
    }

    #[test]
    fn block_structure() {
        let md = "# Title\nintro\n\n- one\n- two\n  continued\n  - nested\n1. first\n2) second\n\n> quoted **text**\n\n```\ncode *raw*\n```\n---\nend";
        let b = parse(md);
        assert!(matches!(&b[0], Block::Heading(1, s) if plain(s) == "Title"));
        assert!(matches!(&b[1], Block::Para(_)));
        let Block::List(items) = &b[2] else { panic!("{b:?}") };
        assert_eq!(items.len(), 5);
        assert_eq!(items[1].2.len(), 2, "continuation line joins the item");
        assert_eq!(items[2].0, 1, "nested level");
        assert_eq!(items[3].1, "1.");
        assert_eq!(items[4].1, "2.");
        assert!(matches!(&b[3], Block::Quote(l) if l[0].iter().any(|s| s.strong)));
        assert_eq!(b[4], Block::Code("code *raw*".into()));
        assert_eq!(b[5], Block::Rule);
        assert!(matches!(&b[6], Block::Para(l) if plain(&l[0]) == "end"));
    }
}
