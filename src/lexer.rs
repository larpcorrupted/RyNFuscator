//! Lua 5.1 / Luau lexer.
//!
//! This is deliberately simple. It does not build a full AST — the passes in
//! this crate only need a correct stream of tokens with byte spans, so that
//! untouched regions can be copied through verbatim.
//!
//! Anything the lexer does not understand is an error rather than a guess.
//! Guessing is how obfuscators corrupt working code.

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Name(String),
    Number(String),
    /// A string literal, decoded to its bytes (escapes already applied).
    Str(Vec<u8>),
    /// Long bracket string `[[...]]`, decoded.
    LongStr(Vec<u8>),
    Kw(&'static str),
    Op(&'static str),
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub tok: Tok,
    /// Byte offset of the first character.
    pub start: usize,
    /// Byte offset one past the last character.
    pub end: usize,
    /// 1-based line the token starts on.
    #[allow(dead_code)]
    pub line: u32,
}

impl Token {
    pub fn text<'a>(&self, src: &'a str) -> &'a str {
        &src[self.start..self.end]
    }
}

#[derive(Debug)]
pub struct LexError {
    pub line: u32,
    pub msg: String,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.msg)
    }
}

pub const KEYWORDS: &[&str] = &[
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "if", "in",
    "local", "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
];

/// Luau-only keywords. Their presence tells us the source is Luau.
pub const LUAU_KEYWORDS: &[&str] = &["continue", "export", "type", "typeof"];

/// Multi-character operators, longest first.
const OPS: &[&str] = &[
    "...", "..=", "//=", "<<=", ">>=", "==", "~=", "<=", ">=", "..", "::", "->", "+=", "-=", "*=",
    "/=", "%=", "^=", "//", "<<", ">>", "+", "-", "*", "/", "%", "^", "#", "&", "~", "|", "<", ">",
    "=", "(", ")", "{", "}", "[", "]", ";", ":", ",", ".",
];

pub struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
    line: u32,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Lexer {
            src: src.as_bytes(),
            pos: 0,
            line: 1,
        }
    }

    /// Tokenise the whole input. Comments and whitespace are dropped but their
    /// byte ranges stay inside the gaps between tokens, so `Token::start/end`
    /// always address the original text.
    pub fn tokenize(mut self) -> Result<Vec<Token>, LexError> {
        let mut out = Vec::new();
        loop {
            self.skip_trivia()?;
            let start = self.pos;
            let line = self.line;
            if self.pos >= self.src.len() {
                out.push(Token {
                    tok: Tok::Eof,
                    start,
                    end: start,
                    line,
                });
                return Ok(out);
            }
            let tok = self.next_token()?;
            out.push(Token {
                tok,
                start,
                end: self.pos,
                line,
            });
        }
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn peek_at(&self, off: usize) -> Option<u8> {
        self.src.get(self.pos + off).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let c = self.peek();
        if c.is_some() {
            if c == Some(b'\n') {
                self.line += 1;
            }
            self.pos += 1;
        }
        c
    }

    /// Whitespace, line comments and long comments.
    fn skip_trivia(&mut self) -> Result<(), LexError> {
        loop {
            match self.peek() {
                Some(c) if c.is_ascii_whitespace() => {
                    self.bump();
                }
                Some(b'-') if self.peek_at(1) == Some(b'-') => {
                    self.bump();
                    self.bump();
                    // long comment? --[[ ... ]]
                    if let Some(level) = self.long_bracket_level() {
                        self.read_long_bracket(level)?;
                    } else {
                        while let Some(c) = self.peek() {
                            if c == b'\n' {
                                break;
                            }
                            self.bump();
                        }
                    }
                }
                Some(b'#') if self.pos == 0 => {
                    // shebang line
                    while let Some(c) = self.peek() {
                        if c == b'\n' {
                            break;
                        }
                        self.bump();
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    /// If the cursor sits on `[`/`=`*`[`, return the level.
    fn long_bracket_level(&self) -> Option<usize> {
        if self.peek() != Some(b'[') {
            return None;
        }
        let mut i = 1;
        while self.peek_at(i) == Some(b'=') {
            i += 1;
        }
        if self.peek_at(i) == Some(b'[') {
            Some(i - 1)
        } else {
            None
        }
    }

    /// Reads a long bracket body, returning decoded bytes.
    fn read_long_bracket(&mut self, level: usize) -> Result<Vec<u8>, LexError> {
        let line = self.line;
        // consume [=*[
        self.bump();
        for _ in 0..level {
            self.bump();
        }
        self.bump();
        // A newline immediately after the opening bracket is skipped by Lua.
        if self.peek() == Some(b'\r') {
            self.bump();
        }
        if self.peek() == Some(b'\n') {
            self.bump();
        }
        let mut body = Vec::new();
        loop {
            match self.peek() {
                None => {
                    return Err(LexError {
                        line,
                        msg: "unfinished long bracket".into(),
                    })
                }
                Some(b']') => {
                    let mut i = 1;
                    while self.peek_at(i) == Some(b'=') {
                        i += 1;
                    }
                    if i - 1 == level && self.peek_at(i) == Some(b']') {
                        self.bump();
                        for _ in 0..level {
                            self.bump();
                        }
                        self.bump();
                        return Ok(body);
                    }
                    body.push(self.bump().unwrap());
                }
                Some(_) => body.push(self.bump().unwrap()),
            }
        }
    }

    fn next_token(&mut self) -> Result<Tok, LexError> {
        let c = self.peek().unwrap();

        if c.is_ascii_alphabetic() || c == b'_' {
            return Ok(self.read_name());
        }
        if c.is_ascii_digit() || (c == b'.' && self.peek_at(1).map_or(false, |d| d.is_ascii_digit()))
        {
            return self.read_number();
        }
        match c {
            b'"' | b'\'' => return self.read_string(c),
            b'[' => {
                if let Some(level) = self.long_bracket_level() {
                    let body = self.read_long_bracket(level)?;
                    return Ok(Tok::LongStr(body));
                }
            }
            _ => {}
        }

        let rest = &self.src[self.pos..];
        for op in OPS {
            let b = op.as_bytes();
            if rest.len() >= b.len() && &rest[..b.len()] == b {
                for _ in 0..b.len() {
                    self.bump();
                }
                return Ok(Tok::Op(op));
            }
        }

        Err(LexError {
            line: self.line,
            msg: format!("unexpected character {:?}", c as char),
        })
    }

    fn read_name(&mut self) -> Tok {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == b'_' {
                self.bump();
            } else {
                break;
            }
        }
        let s = std::str::from_utf8(&self.src[start..self.pos]).unwrap_or("");
        for kw in KEYWORDS {
            if s == *kw {
                return Tok::Kw(kw);
            }
        }
        Tok::Name(s.to_string())
    }

    fn read_number(&mut self) -> Result<Tok, LexError> {
        let start = self.pos;
        // hex
        if self.peek() == Some(b'0')
            && matches!(self.peek_at(1), Some(b'x') | Some(b'X'))
        {
            self.bump();
            self.bump();
            while let Some(c) = self.peek() {
                if c.is_ascii_hexdigit() || c == b'.' || c == b'p' || c == b'P' || c == b'+' || c == b'-' {
                    // stop a trailing +/- that belongs to the next expression
                    if (c == b'+' || c == b'-')
                        && !matches!(self.src.get(self.pos - 1), Some(b'p') | Some(b'P'))
                    {
                        break;
                    }
                    self.bump();
                } else {
                    break;
                }
            }
            let s = std::str::from_utf8(&self.src[start..self.pos]).unwrap_or("");
            return Ok(Tok::Number(s.to_string()));
        }
        let mut seen_dot = false;
        let mut seen_exp = false;
        while let Some(c) = self.peek() {
            match c {
                b'0'..=b'9' => {
                    self.bump();
                }
                b'.' if !seen_dot && !seen_exp => {
                    // `1..2` is a concat, not a number
                    if self.peek_at(1) == Some(b'.') {
                        break;
                    }
                    seen_dot = true;
                    self.bump();
                }
                b'e' | b'E' if !seen_exp => {
                    seen_exp = true;
                    self.bump();
                    if matches!(self.peek(), Some(b'+') | Some(b'-')) {
                        self.bump();
                    }
                }
                _ => break,
            }
        }
        let s = std::str::from_utf8(&self.src[start..self.pos]).unwrap_or("");
        Ok(Tok::Number(s.to_string()))
    }

    fn read_string(&mut self, quote: u8) -> Result<Tok, LexError> {
        let line = self.line;
        self.bump();
        let mut out = Vec::new();
        loop {
            match self.peek() {
                None | Some(b'\n') => {
                    return Err(LexError {
                        line,
                        msg: "unfinished string".into(),
                    })
                }
                Some(c) if c == quote => {
                    self.bump();
                    return Ok(Tok::Str(out));
                }
                Some(b'\\') => {
                    self.bump();
                    let e = match self.bump() {
                        Some(e) => e,
                        None => {
                            return Err(LexError {
                                line,
                                msg: "unfinished escape".into(),
                            })
                        }
                    };
                    match e {
                        b'a' => out.push(7),
                        b'b' => out.push(8),
                        b'f' => out.push(12),
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'v' => out.push(11),
                        b'\\' => out.push(b'\\'),
                        b'"' => out.push(b'"'),
                        b'\'' => out.push(b'\''),
                        b'\n' => out.push(b'\n'),
                        b'x' => {
                            let mut v: u32 = 0;
                            for _ in 0..2 {
                                match self.peek() {
                                    Some(h) if (h as char).is_ascii_hexdigit() => {
                                        v = v * 16 + (h as char).to_digit(16).unwrap();
                                        self.bump();
                                    }
                                    _ => {
                                        return Err(LexError {
                                            line,
                                            msg: "bad \\x escape".into(),
                                        })
                                    }
                                }
                            }
                            out.push(v as u8);
                        }
                        b'z' => {
                            while let Some(w) = self.peek() {
                                if w.is_ascii_whitespace() {
                                    self.bump();
                                } else {
                                    break;
                                }
                            }
                        }
                        d if d.is_ascii_digit() => {
                            let mut v: u32 = (d - b'0') as u32;
                            for _ in 0..2 {
                                match self.peek() {
                                    Some(n) if n.is_ascii_digit() => {
                                        v = v * 10 + (n - b'0') as u32;
                                        self.bump();
                                    }
                                    _ => break,
                                }
                            }
                            out.push(v as u8);
                        }
                        other => out.push(other),
                    }
                }
                Some(c) => {
                    out.push(c);
                    self.bump();
                }
            }
        }
    }
}

/// Convenience wrapper.
pub fn tokenize(src: &str) -> Result<Vec<Token>, LexError> {
    Lexer::new(src).tokenize()
}
