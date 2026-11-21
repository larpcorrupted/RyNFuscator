use crate::assembler::Assembler;
use crate::lexer::{Tok, Token};
use crate::rng::Rng;

const LIMIT: i64 = 1_000_000;

pub fn apply(asm: &mut Assembler, tokens: &[Token], rng: &mut Rng) -> usize {
    let mut count = 0;
    for i in 0..tokens.len() {
        if let Some(v) = candidate(tokens, i) {
            asm.replace(i, disguise(v, rng));
            count += 1;
        }
    }
    count
}

fn candidate(tokens: &[Token], idx: usize) -> Option<i64> {
    let text = match &tokens[idx].tok {
        Tok::Number(t) => t,
        _ => return None,
    };
    if text.contains('.') || text.contains('e') || text.contains('E') {
        return None;
    }
    if text.starts_with("0x") || text.starts_with("0X") {
        return None;
    }
    if idx > 0 && matches!(&tokens[idx - 1].tok, Tok::Op(".") | Tok::Op(":")) {
        return None;
    }
    text.parse::<i64>().ok()
}

fn disguise(n: i64, rng: &mut Rng) -> String {
    if n.abs() > LIMIT {
        return n.to_string();
    }
    let off = rng.range(2, 9);
    match rng.below(3) {
        0 => format!("({}+{})", n - off, off),
        1 => format!("({}-{})", n + off, off),
        _ => {
            if off != 0 && n % off == 0 {
                format!("({}*{})", n / off, off)
            } else {
                format!("({}-{})", n + off, off)
            }
        }
    }
}
