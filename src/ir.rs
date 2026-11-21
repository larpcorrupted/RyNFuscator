// Bytecode IR. Mirrors the shape of the original: a list of ops with a packed
// operand, per-prototype constant pools, and nested protos for closures.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Op {
    Const = 0,
    Nil = 1,
    True = 2,
    False = 3,
    GetLocal = 4,
    SetLocal = 5,
    Pop = 6,
    Dup = 7,
    Add = 8,
    Sub = 9,
    Mul = 10,
    Div = 11,
    IDiv = 12,
    Mod = 13,
    Pow = 14,
    Neg = 15,
    Not = 16,
    Len = 17,
    Concat = 18,
    Eq = 19,
    Ne = 20,
    Lt = 21,
    Le = 22,
    Gt = 23,
    Ge = 24,
    Jmp = 25,
    JFalse = 26,
    JTrue = 27,
    NewTable = 28,
    SetField = 29,
    GetField = 30,
    SetIndex = 31,
    GetIndex = 32,
    Call = 33,
    CallM = 34,
    Ret = 35,
    Closure = 36,
    ForPrep = 37,
    ForLoop = 38,
    IterNext = 39,
    Vararg = 40,
    SetList = 41,
    GetGlobal = 42,
    SetGlobal = 43,
    SelfArg = 44,
    ToStr = 45,
    And = 46,
    Or = 47,
}

#[derive(Debug, Clone, Copy)]
pub struct Ins {
    pub op: Op,
    pub a: i32,
    pub b: i32,
}

impl Ins {
    pub fn new(op: Op) -> Self {
        Ins { op, a: 0, b: 0 }
    }
    pub fn a(op: Op, a: i32) -> Self {
        Ins { op, a, b: 0 }
    }
    pub fn ab(op: Op, a: i32, b: i32) -> Self {
        Ins { op, a, b }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Konst {
    Nil,
    Bool(bool),
    Num(f64),
    Str(Vec<u8>),
}

#[derive(Debug, Clone, Default)]
pub struct Proto {
    pub nparams: usize,
    pub is_vararg: bool,
    pub nslots: usize,
    pub consts: Vec<Konst>,
    pub code: Vec<Ins>,
    pub children: Vec<Proto>,
}

impl Proto {
    pub fn konst(&mut self, k: Konst) -> i32 {
        if let Some(i) = self.consts.iter().position(|c| *c == k) {
            return i as i32;
        }
        self.consts.push(k);
        (self.consts.len() - 1) as i32
    }

    pub fn emit(&mut self, ins: Ins) -> usize {
        self.code.push(ins);
        self.code.len() - 1
    }

    pub fn here(&self) -> i32 {
        self.code.len() as i32
    }
}
