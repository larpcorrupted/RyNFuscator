use std::collections::HashMap;

use crate::lexer::Token;

pub struct Assembler<'a> {
    src: &'a str,
    tokens: &'a [Token],
    edits: HashMap<usize, String>,
    before: HashMap<usize, Vec<String>>,
    after: HashMap<usize, Vec<String>>,
    pub prelude: Option<String>,
    pub strip_comments: bool,
}

impl<'a> Assembler<'a> {
    pub fn new(src: &'a str, tokens: &'a [Token]) -> Self {
        Assembler {
            src,
            tokens,
            edits: HashMap::new(),
            before: HashMap::new(),
            after: HashMap::new(),
            prelude: None,
            strip_comments: false,
        }
    }

    pub fn replace(&mut self, idx: usize, text: impl Into<String>) {
        self.edits.insert(idx, text.into());
    }

    #[allow(dead_code)]
    pub fn insert_before(&mut self, idx: usize, text: impl Into<String>) {
        self.before.entry(idx).or_default().push(text.into());
    }

    #[allow(dead_code)]
    pub fn insert_after(&mut self, idx: usize, text: impl Into<String>) {
        self.after.entry(idx).or_default().push(text.into());
    }

    /// Reconstruct the original text for a token range, applying edits.
    pub fn slice(&self, from: usize, to: usize) -> String {
        let mut out = String::new();
        if from >= to || to > self.tokens.len() {
            return out;
        }
        let mut cursor = self.tokens[from].start;
        for i in from..to {
            let t = &self.tokens[i];
            if t.start > cursor {
                out.push_str(&self.src[cursor..t.start]);
            }
            match self.edits.get(&i) {
                Some(text) => out.push_str(text),
                None => {
                    if !matches!(t.tok, crate::lexer::Tok::Eof) {
                        out.push_str(t.text(self.src));
                    }
                }
            }
            cursor = t.end;
        }
        out
    }

    /// Replace a whole token range with generated text.
    pub fn replace_range(&mut self, from: usize, to: usize, text: impl Into<String>) {
        self.replace(from, text);
        for i in from + 1..to {
            self.replace(i, "");
        }
    }

    pub fn build(&self) -> String {
        let mut out = String::with_capacity(self.src.len() + 512);

        if let Some(p) = &self.prelude {
            out.push_str(p);
            if !p.ends_with('\n') {
                out.push('\n');
            }
        }

        let mut cursor = 0usize;
        for (i, t) in self.tokens.iter().enumerate() {
            let gap = &self.src[cursor..t.start];
            if self.strip_comments {
                out.push_str(&drop_comments(gap));
            } else {
                out.push_str(gap);
            }

            if let Some(list) = self.before.get(&i) {
                for s in list {
                    out.push_str(s);
                }
            }

            match t.tok {
                crate::lexer::Tok::Eof => {}
                _ => match self.edits.get(&i) {
                    Some(text) => out.push_str(text),
                    None => out.push_str(t.text(self.src)),
                },
            }

            if let Some(list) = self.after.get(&i) {
                for s in list {
                    out.push_str(s);
                }
            }

            cursor = t.end;
        }

        out.push_str(&self.src[cursor..]);
        out
    }
}

fn drop_comments(gap: &str) -> String {
    let mut out = String::with_capacity(gap.len());
    let b = gap.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'-' && i + 1 < b.len() && b[i + 1] == b'-' {
            if i + 2 < b.len() && b[i + 2] == b'[' {
                let mut lvl = 0;
                let mut j = i + 3;
                while j < b.len() && b[j] == b'=' {
                    lvl += 1;
                    j += 1;
                }
                if j < b.len() && b[j] == b'[' {
                    let close = format!("]{}]", "=".repeat(lvl));
                    j += 1;
                    if let Some(pos) = gap[j..].find(&close) {
                        let end = j + pos + close.len();
                        for c in gap[i..end].chars() {
                            if c == '\n' {
                                out.push('\n');
                            }
                        }
                        i = end;
                        continue;
                    }
                    i = b.len();
                    continue;
                }
            }
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        out.push(b[i] as char);
        i += 1;
    }
    out
}
