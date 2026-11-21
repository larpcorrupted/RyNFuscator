use std::collections::HashSet;

use crate::assembler::Assembler;
use crate::lexer::{Tok, Token};
use crate::rng::Rng;

const ALPHA: &[u8] = b"abcdefghijkmnpqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ";

struct Stmt {
    head: usize,
    end: usize,
}

pub fn apply(asm: &mut Assembler, tokens: &[Token], rng: &mut Rng) {
    let mut used: HashSet<String> = tokens
        .iter()
        .filter_map(|t| match &t.tok {
            Tok::Name(n) => Some(n.clone()),
            _ => None,
        })
        .collect();

    let mut jobs: Vec<(usize, usize, Vec<Stmt>)> = Vec::new();
    for (open, close) in regions(tokens) {
        let stmts = split(tokens, open + 1, close);
        if stmts.len() < 2 || stmts.len() > 8 {
            continue;
        }
        if !stmts.iter().all(|s| ok_stmt(tokens, s)) {
            continue;
        }
        jobs.push((open, close, stmts));
    }

    // Highest index first: earlier token positions stay valid as we go.
    jobs.sort_by_key(|(open, _, _)| std::cmp::Reverse(*open));

    for (_open, _close, stmts) in jobs {
        let tab = pick(&mut used, rng);
        let ix = pick(&mut used, rng);
        let go = pick(&mut used, rng);
        let key = pick(&mut used, rng);

        // Collect the statement text before anything is overwritten.
        let bodies: Vec<String> = stmts
            .iter()
            .map(|s| asm.slice(s.head, s.end).trim().to_string())
            .collect();

        let n = bodies.len();

        // Execution order is fixed: statement i always runs before i+1.
        // Only the labels — the physical position of each statement in the
        // table — are permuted, so the emitted table bears no relation to
        // program order while behaviour is untouched.
        let mut slots: Vec<usize> = (0..n).collect();
        let mut k = slots.len();
        while k > 1 {
            k -= 1;
            let j = rng.below(k + 1);
            slots.swap(k, j);
        }
        if n > 1 && slots.iter().copied().eq(0..n) {
            slots.swap(0, 1);
        }

        let mut labels = vec![0i64; n];
        let mut next_label = rng.range(11, 60);
        for slot in 0..n {
            labels[slots[slot]] = next_label;
            next_label += rng.range(2, 9);
        }
        let order: Vec<usize> = (0..n).collect();

        let mut entries = Vec::new();
        for step in 0..n {
            let stmt_index = order[step];
            let next = if step + 1 < n {
                labels[order[step + 1]]
            } else {
                -1
            };
            entries.push(format!(
                "[{}] = function() {} end",
                labels[stmt_index], bodies[stmt_index]
            ));
            entries.push(format!("[{}] = {}", labels[stmt_index] + 1, next));
        }

        let head = format!(
            "do local {tab} = {{{body}}} local {ix} = {first} local {go} = true while {go} do local {k} = {tab}[{ix}] if {k} == nil then {go} = false else {tab}[{ix}]() {ix} = {tab}[{ix} + 1] end end end",
            tab = tab,
            body = entries.join(","),
            ix = ix,
            go = go,
            k = key,
            first = labels[order[0]]
        );

        // Statement text is now consumed by the dispatcher.
        for s in stmts.iter() {
            asm.replace_range(s.head, s.end, "");
        }
        asm.replace_range(stmts[0].head, stmts[0].head + 1, head);
    }
}

fn ok_stmt(tokens: &[Token], s: &Stmt) -> bool {
    if s.end <= s.head {
        return false;
    }
    for t in &tokens[s.head..s.end] {
        match &t.tok {
            Tok::Kw("do")
            | Tok::Kw("then")
            | Tok::Kw("function")
            | Tok::Kw("repeat")
            | Tok::Kw("until")
            | Tok::Kw("end")
            | Tok::Kw("break")
            | Tok::Kw("return")
            | Tok::Kw("local") => return false,
            _ => {}
        }
    }
    true
}

/// Split a block body into statements.
///
/// Lua has no statement terminator, so the boundary is found by looking for a
/// token that cannot continue the expression on its left: a call/assignment
/// followed by a fresh name, or anything after a `)` / `}` / literal.
fn split(tokens: &[Token], from: usize, to: usize) -> Vec<Stmt> {
    let mut out = Vec::new();
    let mut head = from;
    let mut depth = 0usize;

    for i in from..to {
        let t = &tokens[i];
        match &t.tok {
            Tok::Kw("do") | Tok::Kw("then") | Tok::Kw("function") | Tok::Kw("repeat") => depth += 1,
            Tok::Kw("end") | Tok::Kw("until") => depth = depth.saturating_sub(1),
            Tok::Op(";") if depth == 0 => {
                if i > head {
                    out.push(Stmt { head, end: i });
                }
                head = i + 1;
                continue;
            }
            _ => {}
        }

        if depth == 0 && i > head {
            let prev = &tokens[i - 1].tok;
            // An expression cannot be followed directly by a name, so a name
            // right after a value or closing bracket starts a new statement.
            let prev_is_value = matches!(
                prev,
                Tok::Number(_)
                    | Tok::Str(_)
                    | Tok::LongStr(_)
                    | Tok::Op(")")
                    | Tok::Op("}")
                    | Tok::Op("]")
            );
            let starts_name = matches!(&t.tok, Tok::Name(_));
            if prev_is_value && starts_name {
                // `f(x) g(y)` — two calls side by side, so a boundary.
                // `local x = y z` cannot occur; names never follow expressions.
                out.push(Stmt { head, end: i });
                head = i;
            }
        }
    }
    if to > head {
        out.push(Stmt { head, end: to });
    }
    out
}

fn regions(tokens: &[Token]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut stack: Vec<(usize, &'static str)> = Vec::new();
    for (i, t) in tokens.iter().enumerate() {
        match &t.tok {
            Tok::Kw("do") => stack.push((i, "do")),
            Tok::Kw("then") => stack.push((i, "then")),
            Tok::Kw("function") => stack.push((i, "function")),
            Tok::Kw("repeat") => stack.push((i, "repeat")),
            Tok::Kw("end") => {
                if let Some((open, kind)) = stack.pop() {
                    if kind == "do" {
                        out.push((open, i));
                    }
                }
            }
            Tok::Kw("until") => {
                stack.pop();
            }
            _ => {}
        }
    }
    out
}

fn pick(used: &mut HashSet<String>, rng: &mut Rng) -> String {
    loop {
        let len = 3 + rng.below(3);
        let mut s = String::new();
        for _ in 0..len {
            s.push(ALPHA[rng.below(ALPHA.len())] as char);
        }
        if !used.contains(&s) {
            used.insert(s.clone());
            return s;
        }
    }
}
