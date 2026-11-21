// Serialises a Proto tree into the flat table the runtime VM reads.
//
// Layout (all indices are 1-based when emitted into Lua):
//   P = { params, vararg, slots, consts, code, children }
//   code entry = { op, a, b }

use crate::ir::{Konst, Op, Proto};

pub fn consts_lua(p: &Proto) -> String {
    let mut parts = Vec::new();
    for k in &p.consts {
        parts.push(match k {
            Konst::Nil => "0".to_string(),
            Konst::Bool(b) => {
                if *b {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            Konst::Num(n) => num_lua(*n),
            Konst::Str(s) => byte_string(s),
        });
    }
    format!("{{{}}}", parts.join(","))
}

fn num_lua(n: f64) -> String {
    if n == n.trunc() && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

fn byte_string(bytes: &[u8]) -> String {
    let mut out = String::from("\"");
    for b in bytes {
        match b {
            b'"' => out.push_str("\\\""),
            b'\\' => out.push_str("\\\\"),
            b'\n' => out.push_str("\\n"),
            0x20..=0x7e => out.push(*b as char),
            _ => out.push_str(&format!("\\{:03}", b)),
        }
    }
    out.push('"');
    out
}

pub fn code_lua(p: &Proto, remap: &[u8; 48]) -> String {
    let mut parts = Vec::with_capacity(p.code.len());
    for i in &p.code {
        let op = remap[i.op as usize];
        parts.push(format!("{{{},{},{}}}", op, i.a, i.b));
    }
    format!("{{{}}}", parts.join(","))
}

pub fn proto_lua(p: &Proto, remap: &[u8; 48]) -> String {
    let children: Vec<String> = p.children.iter().map(|c| proto_lua(c, remap)).collect();
    format!(
        "{{{},{},{},{},{},{{{}}}}}",
        p.nparams,
        if p.is_vararg { "true" } else { "false" },
        p.nslots.max(4),
        consts_lua(p),
        code_lua(p, remap),
        children.join(",")
    )
}

/// Total instruction count across the tree, used for reporting.
pub fn instruction_count(p: &Proto) -> usize {
    p.code.len() + p.children.iter().map(instruction_count).sum::<usize>()
}

/// Ordered list of opcodes actually used, so the dispatcher only emits
/// handlers the program can reach.
pub fn used_ops(p: &Proto) -> Vec<u8> {
    let mut seen = [false; 64];
    walk(p, &mut seen);
    (0..64u8).filter(|i| seen[*i as usize]).collect()
}

fn walk(p: &Proto, seen: &mut [bool; 64]) {
    for i in &p.code {
        seen[i.op as usize] = true;
    }
    for c in &p.children {
        walk(c, seen);
    }
}

/// Every constant across the tree, flattened.
pub fn all_consts(p: &Proto) -> Vec<Konst> {
    let mut out = p.consts.clone();
    for c in &p.children {
        out.extend(all_consts(c));
    }
    out
}

pub const OP_NAMES: &[&str] = &[
    "Const", "Nil", "True", "False", "GetLocal", "SetLocal", "Pop", "Dup", "Add", "Sub", "Mul",
    "Div", "IDiv", "Mod", "Pow", "Neg", "Not", "Len", "Concat", "Eq", "Ne", "Lt", "Le", "Gt",
    "Ge", "Jmp", "JFalse", "JTrue", "NewTable", "SetField", "GetField", "SetIndex", "GetIndex",
    "Call", "CallM", "Ret", "Closure", "ForPrep", "ForLoop", "IterNext", "Vararg", "SetList",
    "GetGlobal", "SetGlobal", "SelfArg", "ToStr", "And", "Or",
];

pub fn op_name(op: u8) -> &'static str {
    OP_NAMES.get(op as usize).copied().unwrap_or("?")
}

/// Ensure the IR only contains ops the runtime implements.
pub fn validate(p: &Proto) -> Result<(), String> {
    for i in &p.code {
        if (i.op as usize) >= OP_NAMES.len() {
            return Err(format!("bad opcode {}", i.op as u8));
        }
    }
    for c in &p.children {
        validate(c)?;
    }
    Ok(())
}

pub fn op_of(byte: u8) -> Option<Op> {
    use Op::*;
    Some(match byte {
        0 => Const,
        1 => Nil,
        2 => True,
        3 => False,
        4 => GetLocal,
        5 => SetLocal,
        6 => Pop,
        7 => Dup,
        8 => Add,
        9 => Sub,
        10 => Mul,
        11 => Div,
        12 => IDiv,
        13 => Mod,
        14 => Pow,
        15 => Neg,
        16 => Not,
        17 => Len,
        18 => Concat,
        19 => Eq,
        20 => Ne,
        21 => Lt,
        22 => Le,
        23 => Gt,
        24 => Ge,
        25 => Jmp,
        26 => JFalse,
        27 => JTrue,
        28 => NewTable,
        29 => SetField,
        30 => GetField,
        31 => SetIndex,
        32 => GetIndex,
        33 => Call,
        34 => CallM,
        35 => Ret,
        36 => Closure,
        37 => ForPrep,
        38 => ForLoop,
        39 => IterNext,
        40 => Vararg,
        41 => SetList,
        42 => GetGlobal,
        43 => SetGlobal,
        44 => SelfArg,
        45 => ToStr,
        46 => And,
        47 => Or,
        _ => return None,
    })
}
