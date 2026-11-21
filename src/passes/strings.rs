use std::collections::HashSet;

use crate::assembler::Assembler;
use crate::lexer::{Tok, Token};
use crate::rng::Rng;

const MIN_LEN: usize = 4;
const ALPHA: &[u8] = b"abcdefghijkmnpqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ";

pub struct StringsPass {
    decoder: String,
}

impl StringsPass {
    pub fn new(tokens: &[Token], rng: &mut Rng) -> Self {
        let mut used = taken(tokens);
        let decoder = pick(&mut used, rng);
        StringsPass { decoder }
    }

    #[allow(dead_code)]
    pub fn decoder_name(&self) -> &str {
        &self.decoder
    }

    pub fn apply(&self, asm: &mut Assembler, tokens: &[Token], rng: &mut Rng) -> usize {
        let mut count = 0;
        for i in 0..tokens.len() {
            let bytes = match &tokens[i].tok {
                Tok::Str(b) | Tok::LongStr(b) => b.as_slice(),
                _ => continue,
            };
            if bytes.len() < MIN_LEN {
                continue;
            }
            let table = table_expr(bytes, rng);
            asm.replace(i, format!("{}({})", self.decoder, table));
            count += 1;
        }
        count
    }

    pub fn preamble(&self) -> String {
        let t = &self.decoder;
        format!(
            "local {t}=(function()\nlocal function q(a,n)\nlocal o,i={{}},1\nwhile i<=n do o[i]=a[i] i=i+1 end\nreturn o\nend\nreturn function(b)\nlocal k=b[#b]\nlocal o={{}}\nfor i=1,#b-1 do o[i]=string.char((b[i]-k)%256) end\nreturn table.concat(o)\nend\nend)()\n"
        )
    }
}

fn taken(tokens: &[Token]) -> HashSet<String> {
    tokens
        .iter()
        .filter_map(|t| match &t.tok {
            Tok::Name(n) => Some(n.clone()),
            _ => None,
        })
        .collect()
}

fn table_expr(bytes: &[u8], rng: &mut Rng) -> String {
    let key = (rng.next_u64() % 251 + 1) as u8;
    let mut parts = Vec::with_capacity(bytes.len() + 1);
    for b in bytes {
        parts.push(obscure(b.wrapping_add(key) as i64, rng));
    }
    parts.push(obscure(key as i64, rng));
    format!("{{{}}}", parts.join(","))
}

fn obscure(n: i64, rng: &mut Rng) -> String {
    let off = rng.range(2, 9);
    if rng.chance(50) {
        format!("({}+{})", n - off, off)
    } else {
        format!("({}-{})", n + off, off)
    }
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
