//! Text helpers for the XML viewer: a line index (byte offset ↔ line and column) and a small
//! line-oriented XML highlighter. The highlighter keeps its state across lines, so comments,
//! CDATA sections and attribute values that span several lines (inline scripts) are coloured
//! correctly no matter which line the viewer starts drawing at.

use std::ops::Range;

/// Byte offsets of line starts. Lines split on `\n`; a trailing `\r` is not part of the line
/// (the same convention as `bpmn::diff::text_rows`).
#[derive(Clone, Debug, Default)]
pub struct LineIndex {
    starts: Vec<usize>,
    len: usize,
}

impl LineIndex {
    pub fn new(text: &str) -> Self {
        let mut starts = vec![0];
        starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
        // A final newline does not open another (empty) line.
        if starts.len() > 1 && *starts.last().unwrap() == text.len() {
            starts.pop();
        }
        Self { starts, len: text.len() }
    }
    pub fn lines(&self) -> usize {
        self.starts.len()
    }
    /// Byte range of line `i` without its line terminator.
    pub fn range(&self, text: &str, i: usize) -> Range<usize> {
        let start = self.starts[i];
        let mut end = self.starts.get(i + 1).map(|s| s - 1).unwrap_or(self.len);
        if end > start && text.as_bytes().get(end - 1) == Some(&b'\n') {
            end -= 1;
        }
        if end > start && text.as_bytes().get(end - 1) == Some(&b'\r') {
            end -= 1;
        }
        start..end
    }
    pub fn line<'a>(&self, text: &'a str, i: usize) -> &'a str {
        &text[self.range(text, i)]
    }
    /// 0-based line of a byte offset.
    pub fn line_of(&self, offset: usize) -> usize {
        match self.starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        }
    }
    /// 1-based line and column (in characters, as editors such as VS Code count them).
    pub fn line_col(&self, text: &str, offset: usize) -> (usize, usize) {
        let l = self.line_of(offset);
        let start = self.starts[l];
        let col = text[start..offset.min(text.len()).max(start)].chars().count();
        (l + 1, col + 1)
    }
    pub fn start_of(&self, line: usize) -> usize {
        self.starts.get(line).copied().unwrap_or(self.len)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tok {
    Text,
    Punct,
    Tag,
    /// Namespace prefix of a tag or attribute (`bpmn:`), drawn quieter than the name.
    Prefix,
    Attr,
    Value,
    Comment,
    CData,
    Entity,
    Decl,
}

/// Where the highlighter is at the start of a line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum State {
    #[default]
    Text,
    /// Inside a tag, after its name (attributes may follow).
    InTag,
    /// Inside a quoted attribute value; the quote character.
    Value(u8),
    Comment,
    CData,
    /// `<?…?>` or `<!…>`.
    Decl,
}

/// Highlighter state at the start of every line.
pub fn line_states(text: &str, index: &LineIndex) -> Vec<State> {
    let mut out = Vec::with_capacity(index.lines());
    let mut st = State::Text;
    for i in 0..index.lines() {
        out.push(st);
        st = tokenize(index.line(text, i), st, &mut |_, _| {});
    }
    out
}

/// Tokens of one line that starts in `state`.
pub fn tokens(line: &str, state: State) -> Vec<(Range<usize>, Tok)> {
    let mut v: Vec<(Range<usize>, Tok)> = vec![];
    tokenize(line, state, &mut |r, t| {
        // Merge neighbours of the same kind to keep layout jobs small.
        if let Some(last) = v.last_mut() {
            if last.1 == t && last.0.end == r.start {
                last.0.end = r.end;
                return;
            }
        }
        if !r.is_empty() {
            v.push((r, t));
        }
    });
    v
}

/// Run the state machine over one line, reporting tokens; returns the state at the line end.
fn tokenize(line: &str, mut st: State, emit: &mut dyn FnMut(Range<usize>, Tok)) -> State {
    let b = line.as_bytes();
    let n = b.len();
    let mut i = 0;
    while i < n {
        match st {
            State::Comment => {
                let end = find(b, i, b"-->").map(|e| e + 3);
                emit(i..end.unwrap_or(n), Tok::Comment);
                i = end.unwrap_or(n);
                if end.is_some() {
                    st = State::Text;
                }
            }
            State::CData => {
                let end = find(b, i, b"]]>").map(|e| e + 3);
                emit(i..end.unwrap_or(n), Tok::CData);
                i = end.unwrap_or(n);
                if end.is_some() {
                    st = State::Text;
                }
            }
            State::Decl => {
                let end = b[i..].iter().position(|&c| c == b'>').map(|e| i + e + 1);
                emit(i..end.unwrap_or(n), Tok::Decl);
                i = end.unwrap_or(n);
                if end.is_some() {
                    st = State::Text;
                }
            }
            State::Value(q) => {
                let end = b[i..].iter().position(|&c| c == q).map(|e| i + e + 1);
                let stop = end.unwrap_or(n);
                emit_value(b, i, stop, emit);
                i = stop;
                if end.is_some() {
                    st = State::InTag;
                }
            }
            State::InTag => {
                let c = b[i];
                if c.is_ascii_whitespace() {
                    let s = i;
                    while i < n && b[i].is_ascii_whitespace() {
                        i += 1;
                    }
                    emit(s..i, Tok::Text);
                } else if c == b'>' {
                    emit(i..i + 1, Tok::Punct);
                    i += 1;
                    st = State::Text;
                } else if c == b'/' && b.get(i + 1) == Some(&b'>') {
                    emit(i..i + 2, Tok::Punct);
                    i += 2;
                    st = State::Text;
                } else if c == b'=' {
                    emit(i..i + 1, Tok::Punct);
                    i += 1;
                } else if c == b'"' || c == b'\'' {
                    emit(i..i + 1, Tok::Value);
                    i += 1;
                    st = State::Value(c);
                } else {
                    let s = i;
                    while i < n && !b[i].is_ascii_whitespace() && !matches!(b[i], b'=' | b'>' | b'/' | b'"' | b'\'') {
                        i += 1;
                    }
                    if i == s {
                        // A stray character such as a lone '/': keep going.
                        i += 1;
                        emit(s..i, Tok::Punct);
                    } else {
                        emit_name(b, s, i, Tok::Attr, emit);
                    }
                }
            }
            State::Text => {
                if b[i] == b'<' {
                    if b[i..].starts_with(b"<!--") {
                        st = State::Comment;
                    } else if b[i..].starts_with(b"<![CDATA[") {
                        st = State::CData;
                    } else if b[i..].starts_with(b"<?") || b[i..].starts_with(b"<!") {
                        st = State::Decl;
                    } else {
                        let s = i;
                        i += 1;
                        if b.get(i) == Some(&b'/') {
                            i += 1;
                        }
                        emit(s..i, Tok::Punct);
                        let ns = i;
                        while i < n && !b[i].is_ascii_whitespace() && !matches!(b[i], b'>' | b'/') {
                            i += 1;
                        }
                        emit_name(b, ns, i, Tok::Tag, emit);
                        st = State::InTag;
                    }
                } else {
                    let s = i;
                    while i < n && b[i] != b'<' {
                        if b[i] == b'&' {
                            if let Some(e) = b[i..].iter().take(12).position(|&c| c == b';') {
                                if i > s {
                                    emit(s..i, Tok::Text);
                                }
                                emit(i..i + e + 1, Tok::Entity);
                                i += e + 1;
                                return tokenize_rest(line, i, st, emit);
                            }
                        }
                        i += 1;
                    }
                    emit(s..i, Tok::Text);
                }
            }
        }
    }
    st
}

fn tokenize_rest(line: &str, from: usize, st: State, emit: &mut dyn FnMut(Range<usize>, Tok)) -> State {
    let rest = &line[from..];
    tokenize(rest, st, &mut |r, t| emit(r.start + from..r.end + from, t))
}

/// An attribute value run, with `&…;` entities picked out.
fn emit_value(b: &[u8], s: usize, e: usize, emit: &mut dyn FnMut(Range<usize>, Tok)) {
    let mut i = s;
    let mut run = s;
    while i < e {
        if b[i] == b'&' {
            if let Some(k) = b[i..e].iter().take(12).position(|&c| c == b';') {
                emit(run..i, Tok::Value);
                emit(i..i + k + 1, Tok::Entity);
                i += k + 1;
                run = i;
                continue;
            }
        }
        i += 1;
    }
    emit(run..e, Tok::Value);
}

fn emit_name(b: &[u8], s: usize, e: usize, kind: Tok, emit: &mut dyn FnMut(Range<usize>, Tok)) {
    match b[s..e].iter().position(|&c| c == b':') {
        Some(p) => {
            emit(s..s + p + 1, Tok::Prefix);
            emit(s + p + 1..e, kind);
        }
        None => emit(s..e, kind),
    }
}

fn find(hay: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    hay[from..].windows(needle.len()).position(|w| w == needle).map(|p| p + from)
}

/// Case-insensitive substring matches in a line (byte ranges).
pub fn find_in_line(line: &str, needle_lower: &str) -> Vec<Range<usize>> {
    if needle_lower.is_empty() {
        return vec![];
    }
    let lower = line.to_lowercase();
    // Lower-casing can change byte lengths (rare outside ASCII); fall back to exact search then.
    let hay = if lower.len() == line.len() { lower.as_str() } else { line };
    hay.match_indices(needle_lower).map(|(i, m)| i..i + m.len()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_index_maps_offsets_both_ways_with_crlf_and_unicode() {
        let t = "<a>\r\n  <б x=\"1\"/>\r\n</a>\n";
        let ix = LineIndex::new(t);
        assert_eq!(ix.lines(), 3);
        assert_eq!(ix.line(t, 0), "<a>");
        assert_eq!(ix.line(t, 1), "  <б x=\"1\"/>");
        assert_eq!(ix.line(t, 2), "</a>");
        let off = t.find("x=").unwrap();
        assert_eq!(ix.line_col(t, off), (2, 6), "columns count characters, not bytes");
        assert_eq!(ix.line_of(0), 0);
        assert_eq!(ix.line_of(t.len()), 2);
        assert_eq!(LineIndex::new("").lines(), 1);
        assert_eq!(LineIndex::new("x").line("x", 0), "x");
    }

    fn kinds(line: &str, st: State) -> Vec<(String, Tok)> {
        tokens(line, st).into_iter().map(|(r, t)| (line[r].to_string(), t)).collect()
    }

    #[test]
    fn tags_attributes_values_and_prefixes() {
        let k = kinds(r#"<bpmn:task id="T1" name="a &amp; b">x</bpmn:task>"#, State::Text);
        assert!(k.contains(&("bpmn:".into(), Tok::Prefix)));
        assert!(k.contains(&("task".into(), Tok::Tag)));
        assert!(k.contains(&("id".into(), Tok::Attr)));
        assert!(k.contains(&("&amp;".into(), Tok::Entity)));
        assert!(k.contains(&("x".into(), Tok::Text)));
        assert!(k.iter().any(|(s, t)| *t == Tok::Value && s.contains("T1")));
    }

    #[test]
    fn multi_line_constructs_carry_state() {
        let t = "<a v=\"one\ntwo\" b='3'>\n<!-- c1\nc2 --><x/>\n<![CDATA[ if (a < b)\n]]>\n";
        let ix = LineIndex::new(t);
        let st = line_states(t, &ix);
        assert_eq!(st[1], State::Value(b'"'));
        assert_eq!(st[2], State::Text);
        assert_eq!(st[3], State::Comment);
        assert_eq!(st[4], State::Text);
        assert_eq!(st[5], State::CData);
        let k = kinds(ix.line(t, 1), st[1]);
        assert_eq!(k[0], ("two\"".into(), Tok::Value));
        assert!(k.contains(&("b".into(), Tok::Attr)));
        let k = kinds(ix.line(t, 3), st[3]);
        assert_eq!(k[0], ("c2 -->".into(), Tok::Comment));
        assert!(k.contains(&("x".into(), Tok::Tag)));
        assert_eq!(kinds(ix.line(t, 4), st[4])[0].1, Tok::CData);
        assert!(kinds(ix.line(t, 4), st[4]).iter().all(|(_, k)| *k == Tok::CData), "'<' inside CDATA is not a tag");
    }

    #[test]
    fn tokens_cover_the_line_exactly() {
        for line in [r#"  <bpmn:sequenceFlow id="f" sourceRef="a" targetRef="b" />"#, "text &lt; more", "<?xml version=\"1.0\"?>", "</x>"] {
            let toks = tokens(line, State::Text);
            let mut pos = 0;
            for (r, _) in &toks {
                assert_eq!(r.start, pos, "{line}: {toks:?}");
                pos = r.end;
            }
            assert_eq!(pos, line.len(), "{line}");
        }
    }

    #[test]
    fn find_is_case_insensitive() {
        assert_eq!(find_in_line("<Task id=\"TASK\">", "task"), vec![1..5, 10..14]);
        assert!(find_in_line("abc", "").is_empty());
    }
}
