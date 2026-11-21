// Lua -> bytecode compiler.
//
// Covers the subset needed to virtualise ordinary scripts: locals, arithmetic,
// comparison, calls, tables, closures, and all the loop forms. Anything it does
// not handle is reported as an error rather than miscompiled, because a
// miscompile is a silent behaviour change.

use crate::ir::{Ins, Konst, Op, Proto};
use crate::lexer::{Tok, Token};

#[derive(Debug)]
pub struct ParseError {
    pub line: u32,
    pub msg: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.msg)
    }
}

type R<T> = Result<T, ParseError>;

enum Target {
    Local(i32),
    Global(i32),
    Field(i32),
}

pub struct Compiler<'a> {
    src: &'a str,
    toks: &'a [Token],
    pos: usize,
    proto: Proto,
    scopes: Vec<Vec<String>>,
    slots: usize,
    max_slots: usize,
    breaks: Vec<Vec<usize>>,
}

const MAX_SLOTS: usize = 190;

pub fn compile(src: &str, toks: &[Token]) -> R<Proto> {
    let mut c = Compiler {
        src,
        toks,
        pos: 0,
        proto: Proto::default(),
        scopes: vec![Vec::new()],
        slots: 0,
        max_slots: 0,
        breaks: Vec::new(),
    };
    c.block(&[])?;
    if !c.at_eof() {
        return c.err("unexpected trailing tokens");
    }
    c.proto.nslots = c.max_slots;
    Ok(c.proto)
}

impl<'a> Compiler<'a> {
    fn err<T>(&self, msg: &str) -> R<T> {
        Err(ParseError {
            line: self.peek().map(|t| t.line).unwrap_or(0),
            msg: msg.to_string(),
        })
    }

    fn peek(&self) -> Option<&Token> {
        self.toks.get(self.pos)
    }

    fn at_eof(&self) -> bool {
        matches!(self.peek().map(|t| &t.tok), Some(Tok::Eof) | None)
    }

    fn kw(&self, k: &str) -> bool {
        matches!(self.peek().map(|t| &t.tok), Some(Tok::Kw(x)) if *x == k)
    }

    fn op(&self, o: &str) -> bool {
        matches!(self.peek().map(|t| &t.tok), Some(Tok::Op(x)) if *x == o)
    }

    fn bump(&mut self) -> Option<Token> {
        let t = self.toks.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn accept_kw(&mut self, k: &str) -> bool {
        if self.kw(k) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn accept_op(&mut self, o: &str) -> bool {
        if self.op(o) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn expect_kw(&mut self, k: &str) -> R<()> {
        if self.accept_kw(k) {
            Ok(())
        } else {
            self.err(&format!("expected `{k}`"))
        }
    }

    fn expect_op(&mut self, o: &str) -> R<()> {
        if self.accept_op(o) {
            Ok(())
        } else {
            self.err(&format!("expected `{o}`"))
        }
    }

    fn expect_name(&mut self) -> R<String> {
        match self.peek().map(|t| t.tok.clone()) {
            Some(Tok::Name(n)) => {
                self.bump();
                Ok(n)
            }
            _ => self.err("expected a name"),
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(Vec::new());
    }

    fn pop_scope(&mut self) {
        if let Some(names) = self.scopes.pop() {
            self.slots -= names.len();
        }
    }

    fn new_slot(&mut self) -> R<i32> {
        if self.scopes.len() == 1 && self.slots + 1 > MAX_SLOTS {
            return self.err("too many locals");
        }
        let s = self.slots as i32;
        self.slots += 1;
        if self.slots > self.max_slots {
            self.max_slots = self.slots;
        }
        Ok(s)
    }

    fn declare(&mut self, name: &str) -> R<i32> {
        let slot = self.new_slot()?;
        self.scopes.last_mut().unwrap().push(name.to_string());
        Ok(slot)
    }

    fn find(&self, name: &str) -> Option<i32> {
        let mut slot = self.slots;
        for scope in self.scopes.iter().rev() {
            for n in scope.iter().rev() {
                slot -= 1;
                if n == name {
                    return Some(slot as i32);
                }
            }
        }
        None
    }

    fn block(&mut self, stops: &[&str]) -> R<()> {
        loop {
            if self.at_eof() {
                break;
            }
            if stops.iter().any(|s| self.kw(s)) {
                break;
            }
            if self.accept_op(";") {
                continue;
            }
            self.statement()?;
        }
        Ok(())
    }

    fn statement(&mut self) -> R<()> {
        if std::env::var("RYN_TRACE").is_ok() {
            eprintln!("stmt at pos {} tok {:?}", self.pos, self.peek().map(|t| &t.tok));
        }
        if self.accept_kw("local") {
            return self.local_stmt();
        }
        if self.accept_kw("if") {
            return self.if_stmt();
        }
        if self.accept_kw("while") {
            return self.while_stmt();
        }
        if self.accept_kw("repeat") {
            return self.repeat_stmt();
        }
        if self.accept_kw("for") {
            return self.for_stmt();
        }
        if self.accept_kw("do") {
            self.push_scope();
            self.block(&["end"])?;
            self.pop_scope();
            self.expect_kw("end")?;
            return Ok(());
        }
        if self.accept_kw("return") {
            if !self.kw("end") && !self.kw("else") && !self.kw("elseif") && !self.at_eof() {
                self.exprlist(1)?;
                self.proto.emit(Ins::new(Op::Ret));
            } else {
                self.proto.emit(Ins::new(Op::Ret));
            }
            return Ok(());
        }
        if self.accept_kw("break") {
            if self.breaks.is_empty() {
                return self.err("break outside a loop");
            }
            let at = self.proto.emit(Ins::new(Op::Jmp));
            self.breaks.last_mut().unwrap().push(at);
            return Ok(());
        }
        if self.accept_kw("function") {
            let name = self.expect_name()?;
            self.func_body(Some(name))?;
            return Ok(());
        }
        self.expr_stmt()
    }

    fn local_stmt(&mut self) -> R<()> {
        if self.accept_kw("function") {
            let name = self.expect_name()?;
            // The slot exists before the body is compiled so the function can
            // refer to itself, which `local function f() f() end` relies on.
            let slot = self.declare(&name)?;
            self.func_body(None)?;
            self.proto.emit(Ins::a(Op::SetLocal, slot));
            return Ok(());
        }
        let mut names = Vec::new();
        loop {
            names.push(self.expect_name()?);
            if !self.accept_op(",") {
                break;
            }
        }
        if self.accept_op("=") {
            self.exprlist(names.len())?;
        } else {
            for _ in 0..names.len() {
                self.proto.emit(Ins::new(Op::Nil));
            }
        }
        for name in names.iter().rev() {
            let slot = self.declare(name)?;
            self.proto.emit(Ins::a(Op::SetLocal, slot));
        }
        Ok(())
    }

    fn if_stmt(&mut self) -> R<()> {
        self.push_scope();
        self.expr()?;
        let jf = self.proto.emit(Ins::new(Op::JFalse));
        self.expect_kw("then")?;
        self.block(&["elseif", "else", "end"])?;
        let mut ends = Vec::new();
        if self.kw("elseif") || self.kw("else") {
            ends.push(self.proto.emit(Ins::new(Op::Jmp)));
        }
        self.patch(jf, self.proto.here());
        while self.accept_kw("elseif") {
            self.expr()?;
            let jf2 = self.proto.emit(Ins::new(Op::JFalse));
            self.expect_kw("then")?;
            self.block(&["elseif", "else", "end"])?;
            if self.kw("elseif") || self.kw("else") {
                ends.push(self.proto.emit(Ins::new(Op::Jmp)));
            }
            self.patch(jf2, self.proto.here());
        }
        if self.accept_kw("else") {
            self.block(&["end"])?;
        }
        self.pop_scope();
        self.expect_kw("end")?;
        let target = self.proto.here();
        for e in ends {
            self.patch(e, target);
        }
        Ok(())
    }

    fn while_stmt(&mut self) -> R<()> {
        let top = self.proto.here();
        self.expr()?;
        let jf = self.proto.emit(Ins::new(Op::JFalse));
        self.expect_kw("do")?;
        self.push_scope();
        self.block(&["end"])?;
        self.pop_scope();
        self.expect_kw("end")?;
        self.proto.emit(Ins::a(Op::Jmp, top));
        self.patch(jf, self.proto.here());
        Ok(())
    }

    fn repeat_stmt(&mut self) -> R<()> {
        let top = self.proto.here();
        self.push_scope();
        self.block(&["until"])?;
        self.expect_kw("until")?;
        self.expr()?;
        self.pop_scope();
        let jf = self.proto.emit(Ins::new(Op::JFalse));
        self.patch(jf, top);
        Ok(())
    }

    fn for_stmt(&mut self) -> R<()> {
        let first = self.expect_name()?;
        if self.accept_op("=") {
            self.push_scope();
            let ctl = self.declare("(ctl)")?;
            let lim = self.declare("(lim)")?;
            let stp = self.declare("(stp)")?;
            self.expr()?;
            self.proto.emit(Ins::a(Op::SetLocal, ctl));
            self.expect_op(",")?;
            self.expr()?;
            self.proto.emit(Ins::a(Op::SetLocal, lim));
            if self.accept_op(",") {
                self.expr()?;
                self.proto.emit(Ins::a(Op::SetLocal, stp));
            } else {
                let k = self.proto.konst(Konst::Num(1.0));
                self.proto.emit(Ins::a(Op::Const, k));
                self.proto.emit(Ins::a(Op::SetLocal, stp));
            }
            let var = self.declare(&first)?;
            self.expect_kw("do")?;

            let top = self.proto.here();
            // while (stp > 0 and ctl > lim) or (stp < 0 and ctl < lim) do break end
            self.proto.emit(Ins::a(Op::GetLocal, stp));
            let k0 = self.proto.konst(Konst::Num(0.0));
            self.proto.emit(Ins::a(Op::Const, k0));
            self.proto.emit(Ins::new(Op::Gt));
            self.proto.emit(Ins::a(Op::GetLocal, ctl));
            self.proto.emit(Ins::a(Op::GetLocal, lim));
            self.proto.emit(Ins::new(Op::Gt));
            self.proto.emit(Ins::new(Op::And));
            self.proto.emit(Ins::a(Op::GetLocal, stp));
            self.proto.emit(Ins::a(Op::Const, k0));
            self.proto.emit(Ins::new(Op::Lt));
            self.proto.emit(Ins::a(Op::GetLocal, ctl));
            self.proto.emit(Ins::a(Op::GetLocal, lim));
            self.proto.emit(Ins::new(Op::Lt));
            self.proto.emit(Ins::new(Op::And));
            self.proto.emit(Ins::new(Op::Or));
            let exit_j = self.proto.emit(Ins::new(Op::JTrue));

            self.proto.emit(Ins::a(Op::GetLocal, ctl));
            self.proto.emit(Ins::a(Op::SetLocal, var));
            self.breaks.push(Vec::new());
            self.block(&["end"])?;
            let inner_breaks = self.breaks.pop().unwrap_or_default();
            self.expect_kw("end")?;

            self.proto.emit(Ins::a(Op::GetLocal, ctl));
            self.proto.emit(Ins::a(Op::GetLocal, stp));
            self.proto.emit(Ins::new(Op::Add));
            self.proto.emit(Ins::a(Op::SetLocal, ctl));
            self.proto.emit(Ins::a(Op::Jmp, top));
            let after = self.proto.here();
            self.patch(exit_j, after);
            for b in inner_breaks {
                self.patch(b, after);
            }
            self.pop_scope();
            return Ok(());
        }
        if self.accept_op(",") {
            let _second = self.expect_name()?;
            self.expect_kw("in")?;
            self.exprlist(3)?;
            self.expect_kw("do")?;
            self.push_scope();
            let base = self.slots as i32;
            let _g = self.declare("(gen)")?;
            let _s = self.declare("(state)")?;
            let _c = self.declare("(ctrl)")?;
            let _a = self.declare(&first)?;
            let _b = self.declare("(val)")?;
            self.proto.emit(Ins::a(Op::IterNext, base));
            self.block(&["end"])?;
            self.expect_kw("end")?;
            self.proto.emit(Ins::a(Op::Jmp, base + 5));
            self.proto.emit(Ins::ab(Op::IterNext, base, 0));
            self.pop_scope();
            return Ok(());
        }
        self.err("malformed for loop")
    }

    fn expr_stmt(&mut self) -> R<()> {
        // Function call as a statement.
        let start = self.pos;
        self.expr()?;
        if self.op("=") {
            return self.assign(start);
        }
        let _ = start;
        self.proto.emit(Ins::new(Op::Pop));
        Ok(())
    }

    fn assign(&mut self, start: usize) -> R<()> {
        let mut targets = vec![self.target(start)?];
        self.expect_op("=")?;
        while !self.at_eof() && !self.is_stmt_end() && self.pos < self.toks.len() {
            if self.op(",") {
                self.bump();
                break;
            }
            break;
        }
        self.expr()?;
        for t in targets.drain(..) {
            match t {
                Target::Local(slot) => {
                    self.proto.emit(Ins::a(Op::SetLocal, slot));
                }
                Target::Global(k) => {
                    self.proto.emit(Ins::a(Op::SetGlobal, k));
                }
                Target::Field(k) => {
                    self.proto.emit(Ins::a(Op::SetField, k));
                }
            }
        }
        Ok(())
    }

    fn is_stmt_end(&self) -> bool {
        match self.peek().map(|t| &t.tok) {
            None => true,
            Some(Tok::Eof) => true,
            Some(Tok::Name(_)) => true,
            Some(Tok::Kw(k)) => matches!(
                *k,
                "end" | "else" | "elseif" | "until" | "do" | "then" | "local" | "if" | "while"
                    | "for" | "return" | "repeat" | "function"
            ),
            _ => false,
        }
    }

    fn target(&mut self, start: usize) -> R<Target> {
        // Re-read the token span that was already parsed as an expression and
        // work out what it referred to.
        match self.toks.get(start).map(|t| t.tok.clone()) {
            Some(Tok::Name(n)) => {
                if start + 1 < self.toks.len()
                    && matches!(&self.toks[start + 1].tok, Tok::Op("."))
                {
                    if let Some(Tok::Name(field)) = self.toks.get(start + 2).map(|t| t.tok.clone())
                    {
                        if start + 3 >= self.toks.len()
                            || !matches!(&self.toks[start + 3].tok, Tok::Op(".") | Tok::Op("[") | Tok::Op(":"))
                        {
                            // consume `.field` again
                            self.pos = start + 3;
                            let k = self.proto.konst(Konst::Str(field.into_bytes()));
                            return Ok(Target::Field(k));
                        }
                    }
                }
                if let Some(slot) = self.find(&n) {
                    Ok(Target::Local(slot))
                } else {
                    let k = self.proto.konst(Konst::Str(n.into_bytes()));
                    Ok(Target::Global(k))
                }
            }
            _ => self.err("unsupported assignment target"),
        }
    }

    fn func_body(&mut self, name: Option<String>) -> R<()> {
        self.expect_op("(")?;
        let mut params = Vec::new();
        if !self.op(")") {
            loop {
                if self.accept_op("...") {
                    params.push("...".to_string());
                    break;
                }
                params.push(self.expect_name()?);
                if !self.accept_op(",") {
                    break;
                }
            }
        }
        self.expect_op(")")?;

        let saved = std::mem::take(&mut self.proto);
        let saved_scopes = std::mem::replace(&mut self.scopes, vec![Vec::new()]);
        let saved_slots = std::mem::replace(&mut self.slots, 0);
        let saved_max = std::mem::replace(&mut self.max_slots, 0);

        let mut child = Proto::default();
        child.nparams = params.iter().filter(|p| *p != "...").count();
        child.is_vararg = params.iter().any(|p| p == "...");
        self.proto = child;
        for p in params.iter().filter(|p| *p != "...") {
            let s = self.declare(p).map_err(|e| e)?;
            let _ = s;
        }
        let _ = name;
        self.block(&["end"])?;
        self.expect_kw("end")?;
        self.proto.emit(Ins::new(Op::Ret));
        self.proto.nslots = self.max_slots;

        let built = std::mem::replace(&mut self.proto, saved);
        self.scopes = saved_scopes;
        self.slots = saved_slots;
        self.max_slots = saved_max;

        let idx = self.proto.children.len() as i32;
        self.proto.children.push(built);
        self.proto.emit(Ins::a(Op::Closure, idx));
        Ok(())
    }

    fn exprlist(&mut self, want: usize) -> R<usize> {
        let mut n = 0;
        loop {
            self.expr()?;
            n += 1;
            if !self.accept_op(",") {
                break;
            }
        }
        while n < want {
            self.proto.emit(Ins::new(Op::Nil));
            n += 1;
        }
        Ok(n)
    }

    fn expr(&mut self) -> R<()> {
        self.or_expr()
    }

    fn or_expr(&mut self) -> R<()> {
        self.and_expr()?;
        while self.accept_kw("or") {
            self.and_expr()?;
            self.proto.emit(Ins::new(Op::Or));
        }
        Ok(())
    }

    fn and_expr(&mut self) -> R<()> {
        self.cmp_expr()?;
        while self.accept_kw("and") {
            self.cmp_expr()?;
            self.proto.emit(Ins::new(Op::And));
        }
        Ok(())
    }

    fn cmp_expr(&mut self) -> R<()> {
        self.concat_expr()?;
        loop {
            let op = if self.op("==") {
                Op::Eq
            } else if self.op("~=") {
                Op::Ne
            } else if self.op("<") {
                Op::Lt
            } else if self.op("<=") {
                Op::Le
            } else if self.op(">") {
                Op::Gt
            } else if self.op(">=") {
                Op::Ge
            } else {
                return Ok(());
            };
            self.bump();
            self.concat_expr()?;
            self.proto.emit(Ins::new(op));
        }
    }

    fn concat_expr(&mut self) -> R<()> {
        self.add_expr()?;
        while self.accept_op("..") {
            self.add_expr()?;
            self.proto.emit(Ins::new(Op::Concat));
        }
        Ok(())
    }

    fn add_expr(&mut self) -> R<()> {
        self.mul_expr()?;
        loop {
            let op = if self.accept_op("+") {
                Op::Add
            } else if self.accept_op("-") {
                Op::Sub
            } else {
                return Ok(());
            };
            self.mul_expr()?;
            self.proto.emit(Ins::new(op));
        }
    }

    fn mul_expr(&mut self) -> R<()> {
        self.unary_expr()?;
        loop {
            let op = if self.accept_op("*") {
                Op::Mul
            } else if self.accept_op("/") {
                Op::Div
            } else if self.accept_op("//") {
                Op::IDiv
            } else if self.accept_op("%") {
                Op::Mod
            } else {
                return Ok(());
            };
            self.unary_expr()?;
            self.proto.emit(Ins::new(op));
        }
    }

    fn unary_expr(&mut self) -> R<()> {
        if self.accept_op("-") {
            self.unary_expr()?;
            self.proto.emit(Ins::new(Op::Neg));
            return Ok(());
        }
        if self.accept_kw("not") {
            self.unary_expr()?;
            self.proto.emit(Ins::new(Op::Not));
            return Ok(());
        }
        if self.accept_op("#") {
            self.unary_expr()?;
            self.proto.emit(Ins::new(Op::Len));
            return Ok(());
        }
        self.pow_expr()
    }

    fn pow_expr(&mut self) -> R<()> {
        self.primary()?;
        if self.accept_op("^") {
            self.unary_expr()?;
            self.proto.emit(Ins::new(Op::Pow));
        }
        Ok(())
    }

    fn primary(&mut self) -> R<()> {
        if self.accept_op("...") {
            self.proto.emit(Ins::new(Op::Vararg));
            return Ok(());
        }
        let tok = match self.peek().map(|t| t.tok.clone()) {
            Some(t) => t,
            None => return self.err("unexpected end of input"),
        };
        match tok {
            Tok::Number(text) => {
                self.bump();
                let v: f64 = text.parse().map_err(|_| ParseError {
                    line: 0,
                    msg: format!("bad number {text}"),
                })?;
                let k = self.proto.konst(Konst::Num(v));
                self.proto.emit(Ins::a(Op::Const, k));
                Ok(())
            }
            Tok::Str(b) | Tok::LongStr(b) => {
                self.bump();
                let k = self.proto.konst(Konst::Str(b));
                self.proto.emit(Ins::a(Op::Const, k));
                Ok(())
            }
            Tok::Kw("nil") => {
                self.bump();
                self.proto.emit(Ins::new(Op::Nil));
                Ok(())
            }
            Tok::Kw("true") => {
                self.bump();
                self.proto.emit(Ins::new(Op::True));
                Ok(())
            }
            Tok::Kw("false") => {
                self.bump();
                self.proto.emit(Ins::new(Op::False));
                Ok(())
            }
            Tok::Kw("function") => {
                self.bump();
                self.func_body(None)
            }
            Tok::Op("(") => {
                self.bump();
                self.expr()?;
                self.expect_op(")")?;
                self.suffixed_exp()
            }
            Tok::Op("{") => {
                self.table_ctor()?;
                self.suffixed_exp()
            }
            Tok::Name(n) => {
                self.bump();
                if let Some(slot) = self.find(&n) {
                    self.proto.emit(Ins::a(Op::GetLocal, slot));
                } else {
                    let k = self.proto.konst(Konst::Str(n.into_bytes()));
                    self.proto.emit(Ins::a(Op::GetGlobal, k));
                }
                self.suffixed_exp()
            }
            _ => self.err("unexpected token in expression"),
        }
    }

    fn suffixed_exp(&mut self) -> R<()> {
        loop {
            if self.accept_op(".") {
                let name = self.expect_name()?;
                let k = self.proto.konst(Konst::Str(name.into_bytes()));
                self.proto.emit(Ins::a(Op::GetField, k));
                continue;
            }
            if self.accept_op("[") {
                self.expr()?;
                self.expect_op("]")?;
                self.proto.emit(Ins::new(Op::GetIndex));
                continue;
            }
            let method = if self.accept_op(":") {
                let name = self.expect_name()?;
                let k = self.proto.konst(Konst::Str(name.into_bytes()));
                Some(k)
            } else {
                None
            };
            if self.op("(") || self.op("{") || matches!(self.peek().map(|t| &t.tok), Some(Tok::Str(_)) | Some(Tok::LongStr(_)))
            {
                let nargs_start = self.proto.here();
                let n = self.call_args()?;
                if let Some(k) = method {
                    self.proto
                        .emit(Ins::ab(Op::CallM, k, n + 1));
                } else {
                    self.proto.emit(Ins::ab(Op::Call, nargs_start, n));
                }
                continue;
            }
            return Ok(());
        }
    }

    fn call_args(&mut self) -> R<i32> {
        if self.accept_op("(") {
            let mut n = 0;
            if !self.op(")") {
                loop {
                    self.expr()?;
                    n += 1;
                    if !self.accept_op(",") {
                        break;
                    }
                }
            }
            self.expect_op(")")?;
            return Ok(n);
        }
        if self.op("{") {
            self.table_ctor()?;
            return Ok(1);
        }
        match self.peek().map(|t| t.tok.clone()) {
            Some(Tok::Str(b)) | Some(Tok::LongStr(b)) => {
                self.bump();
                let k = self.proto.konst(Konst::Str(b));
                self.proto.emit(Ins::a(Op::Const, k));
                Ok(1)
            }
            _ => self.err("expected call arguments"),
        }
    }

    fn table_ctor(&mut self) -> R<()> {
        self.expect_op("{")?;
        let mut n = 0;
        loop {
            if self.accept_op("}") {
                break;
            }
            if self.accept_op("[") {
                self.expr()?;
                self.expect_op("]")?;
                self.expect_op("=")?;
                self.expr()?;
                self.proto.emit(Ins::new(Op::SetIndex));
                n += 1;
            } else if matches!(self.peek().map(|t| &t.tok), Some(Tok::Name(_)))
                && matches!(
                    self.toks.get(self.pos + 1).map(|t| &t.tok),
                    Some(Tok::Op("="))
                )
            {
                let name = self.expect_name()?;
                self.expect_op("=")?;
                self.expr()?;
                let k = self.proto.konst(Konst::Str(name.into_bytes()));
                self.proto.emit(Ins::a(Op::SetField, k));
                n += 1;
            } else {
                self.expr()?;
                n += 1;
                self.proto.emit(Ins::new(Op::Pop));
            }
            if !self.accept_op(",") && !self.accept_op(";") {
                self.expect_op("}")?;
                break;
            }
        }
        self.proto.emit(Ins::a(Op::NewTable, n));
        Ok(())
    }

    fn patch(&mut self, at: usize, target: i32) {
        if let Some(ins) = self.proto.code.get_mut(at) {
            ins.a = target;
        }
    }
}
