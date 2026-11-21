use std::collections::{HashMap, HashSet};

use crate::assembler::Assembler;
use crate::lexer::{Tok, Token};
use crate::rng::Rng;

const ALPHA: &[u8] = b"abcdefghijkmnpqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ";

pub fn apply(asm: &mut Assembler, tokens: &[Token], rng: &mut Rng) {
    let mut used: HashSet<String> = tokens
        .iter()
        .filter_map(|t| match &t.tok {
            Tok::Name(n) => Some(n.clone()),
            _ => None,
        })
        .collect();

    for (idx, new) in plan(tokens, &mut used, rng) {
        asm.replace(idx, new);
    }
}

pub fn count(tokens: &[Token]) -> usize {
    let mut used = HashSet::new();
    let mut rng = Rng::new(0);
    plan(tokens, &mut used, &mut rng).len()
}

fn plan(
    tokens: &[Token],
    used: &mut HashSet<String>,
    rng: &mut Rng,
) -> Vec<(usize, String)> {
    let mut out = Vec::new();

    let mut barred: HashSet<String> = HashSet::new();
    for kw in crate::lexer::KEYWORDS.iter().chain(crate::lexer::LUAU_KEYWORDS.iter()) {
        barred.insert((*kw).to_string());
    }
    for (i, t) in tokens.iter().enumerate() {
        if let Tok::Name(n) = &t.tok {
            let is_field = i > 0 && matches!(&tokens[i - 1].tok, Tok::Op(".") | Tok::Op(":"));
            if is_field {
                barred.insert(n.clone());
            }
        }
    }

    let mut decls: HashMap<String, usize> = HashMap::new();
    for i in 0..tokens.len() {
        if !matches!(&tokens[i].tok, Tok::Kw("local")) {
            continue;
        }
        let mut j = i + 1;
        if matches!(tokens.get(j).map(|t| &t.tok), Some(Tok::Kw("function"))) {
            j += 1;
        }
        if let Some(Tok::Name(n)) = tokens.get(j).map(|t| &t.tok) {
            *decls.entry(n.clone()).or_insert(0) += 1;
        }
    }

    let mut names: Vec<String> = decls
        .iter()
        .filter(|(n, c)| **c == 1 && !barred.contains(*n))
        .map(|(n, _)| n.clone())
        .collect();
    names.sort();

    for name in names {
        let decl = match tokens.iter().position(|t| match &t.tok {
            Tok::Name(n) => n == &name,
            _ => false,
        }) {
            Some(d) => d,
            None => continue,
        };

        let prev = decl.checked_sub(1).and_then(|i| tokens.get(i));
        let prev2 = decl.checked_sub(2).and_then(|i| tokens.get(i));
        let declared = matches!(prev.map(|t| &t.tok), Some(Tok::Kw("local")))
            || matches!(
                (prev.map(|t| &t.tok), prev2.map(|t| &t.tok)),
                (Some(Tok::Kw("function")), Some(Tok::Kw("local")))
            );
        if !declared {
            continue;
        }

        let mut sites = Vec::new();
        let mut bail = false;
        for (i, t) in tokens.iter().enumerate() {
            if let Tok::Name(n) = &t.tok {
                if n != &name {
                    continue;
                }
                if i < decl {
                    bail = true;
                    break;
                }
                if i > 0 && matches!(&tokens[i - 1].tok, Tok::Op(".") | Tok::Op(":")) {
                    continue;
                }
                sites.push(i);
            }
        }
        if bail || sites.is_empty() {
            continue;
        }

        let in_string = tokens.iter().any(|t| match &t.tok {
            Tok::Str(b) | Tok::LongStr(b) => {
                std::str::from_utf8(b).map_or(false, |s| s.contains(&name))
            }
            _ => false,
        });
        if in_string {
            continue;
        }

        let fresh = pick(used, rng);
        for i in sites {
            out.push((i, fresh.clone()));
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
