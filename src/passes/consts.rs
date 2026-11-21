// Constant pool scrambling.
//
// User-written constants are XOR-folded into a two-part encoding so a strings
// dump of the artifact never shows the original text. The runtime VM undoes it
// on load. Only constants are touched; the code and its control flow are not.

use crate::ir::{Konst, Proto};
use crate::rng::Rng;

/// Marker the runtime looks for in the constant table.
pub const MARK: &str = "\u{1}R";

pub fn scramble(p: &mut Proto, rng: &mut Rng) {
    let key = (rng.next_u64() % 250 + 1) as u8;
    for k in p.consts.iter_mut() {
        if let Konst::Str(bytes) = k {
            if bytes.len() < 3 {
                continue;
            }
            let folded: Vec<u8> = bytes
                .iter()
                .enumerate()
                .map(|(i, b)| b.wrapping_add(key).wrapping_add(i as u8))
                .collect();
            let mut encoded = Vec::with_capacity(folded.len() + 1);
            encoded.push(key);
            encoded.extend_from_slice(&folded);
            *bytes = encoded;
        }
    }
    for c in p.children.iter_mut() {
        scramble(c, rng);
    }
}

/// Emitted once ahead of the VM: rewrites every string constant in place.
pub fn decoder_source(rng: &mut Rng) -> String {
    let name = {
        const A: &[u8] = b"abcdefghijkmnpqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ";
        let mut s = String::new();
        for _ in 0..4 {
            s.push(A[rng.below(A.len())] as char);
        }
        s
    };
    format!(
        "local function {name}(P)\n  local c = P[4]\n  for i = 1, #c do\n    local v = c[i]\n    if type(v) == \"string\" and #v > 2 then\n      local k = string.byte(v, 1)\n      local o = {{}}\n      for j = 2, #v do\n        o[j - 1] = string.char((string.byte(v, j) - k - (j - 2)) % 256)\n      end\n      c[i] = table.concat(o)\n    end\n  end\n  for _, ch in ipairs(P[6]) do {name}(ch) end\n  return P\nend\n"
    )
}

/// Applies the decoder to the root proto at runtime.
pub fn apply_source(decoder: &str) -> String {
    format!("{decoder}(_P)\n")
}

/// The local name the decoder was bound to.
pub fn decoder_name(src: &str) -> String {
    src.trim_start()
        .trim_start_matches("local function ")
        .trim_start_matches("local ")
        .split(|c: char| c == '(' || c.is_whitespace())
        .next()
        .unwrap_or("_d")
        .to_string()
}
