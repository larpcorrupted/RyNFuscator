//! Tiny indentation-aware output buffer.
//!
//! Emitted code is generated rather than copied, so it has to be laid out
//! from scratch. Keeping this separate keeps the passes readable.

#[derive(Debug, Default)]
pub struct Buf {
    out: String,
    indent: usize,
}

impl Buf {
    pub fn new() -> Self {
        Buf::default()
    }

    pub fn depth(&self) -> usize {
        self.indent
    }

    pub fn line(&mut self, s: &str) {
        for _ in 0..self.indent {
            self.out.push_str("  ");
        }
        self.out.push_str(s);
        self.out.push('\n');
    }

    /// A line with no indentation applied and no newline appended.
    pub fn raw(&mut self, s: &str) {
        self.out.push_str(s);
    }

    pub fn blank(&mut self) {
        self.out.push('\n');
    }

    pub fn open(&mut self, s: &str) {
        self.line(s);
        self.indent += 1;
    }

    pub fn close(&mut self, s: &str) {
        self.indent = self.indent.saturating_sub(1);
        self.line(s);
    }

    pub fn finish(mut self) -> String {
        while self.out.ends_with("\n\n") {
            self.out.pop();
        }
        self.out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indents_and_dedents() {
        let mut b = Buf::new();
        b.open("do");
        b.line("x = 1");
        b.close("end");
        assert_eq!(b.finish(), "do\n  x = 1\nend\n");
    }
}
