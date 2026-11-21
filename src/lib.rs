mod assembler;
mod bytecode;
mod header;
mod ir;
mod lexer;
mod parse;
mod passes;
mod rng;
mod vm;

use std::fmt;

use rng::{seed_from_source, Rng};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    Luau,
    Lua51,
}

impl Preset {
    pub fn label(self) -> &'static str {
        match self {
            Preset::Luau => "luau",
            Preset::Lua51 => "lua51",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Options {
    pub preset: Preset,
    pub seed: Option<u64>,
    pub guard: bool,
    pub virtualize: bool,
    pub encode_strings: bool,
    pub rename_locals: bool,
    pub keep_debug: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            preset: Preset::Luau,
            seed: None,
            guard: true,
            virtualize: true,
            encode_strings: true,
            rename_locals: true,
            keep_debug: false,
        }
    }
}

#[derive(Debug)]
pub enum ProtectError {
    Lex(lexer::LexError),
    Parse(parse::ParseError),
    Empty,
    GuardMismatch,
    Unsupported(String),
}

impl fmt::Display for ProtectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProtectError::Lex(e) => write!(f, "{e}"),
            ProtectError::Parse(e) => write!(f, "{e}"),
            ProtectError::Empty => write!(f, "input has no code"),
            ProtectError::GuardMismatch => {
                write!(f, "roblox guard is only available for the luau preset")
            }
            ProtectError::Unsupported(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for ProtectError {}

impl From<lexer::LexError> for ProtectError {
    fn from(e: lexer::LexError) -> Self {
        ProtectError::Lex(e)
    }
}

impl From<parse::ParseError> for ProtectError {
    fn from(e: parse::ParseError) -> Self {
        ProtectError::Parse(e)
    }
}

pub struct Build {
    pub source: String,
    pub seed: u64,
    pub instructions: usize,
    pub handlers: usize,
    pub virtualized: bool,
}

pub fn protect(input: &str, opts: &Options) -> Result<String, ProtectError> {
    Ok(protect_detailed(input, opts)?.source)
}

pub fn protect_detailed(input: &str, opts: &Options) -> Result<Build, ProtectError> {
    if opts.preset == Preset::Lua51 && opts.guard {
        return Err(ProtectError::GuardMismatch);
    }

    let seed = opts.seed.unwrap_or_else(|| seed_from_source(input));
    let mut rng = Rng::new(seed);

    let tokens = lexer::tokenize(input)?;
    if tokens.len() <= 1 {
        return Err(ProtectError::Empty);
    }

    let mut prelude = String::new();
    prelude.push_str(header::HEADER);
    prelude.push('\n');

    if opts.guard && !passes::guard::already_guarded(input) {
        prelude.push_str(&passes::guard::build(&mut rng, true));
    }

    if opts.virtualize {
        // Compile to bytecode, then wrap the bytecode in an interpreter.
        let mut proto = parse::compile(input, &tokens)?;
        passes::consts::scramble(&mut proto, &mut rng);
        bytecode::validate(&proto).map_err(ProtectError::Unsupported)?;

        let dec_src = passes::consts::decoder_source(&mut rng);
        let dec_name = passes::consts::decoder_name(&dec_src);
        let built = vm::emit(&proto, &mut rng, &dec_name);
        prelude.push_str(&dec_src);

        prelude.push_str(&built.source);

        return Ok(Build {
            source: prelude,
            seed,
            instructions: bytecode::instruction_count(&proto),
            handlers: built.handlers,
            virtualized: true,
        });
    }

    // Fallback: token-level passes, no VM.
    let asm = assembler::Assembler::new(input, &tokens);
    let mut asm = asm;
    asm.strip_comments = !opts.keep_debug;

    let sp = passes::strings::StringsPass::new(&tokens, &mut rng);
    let mut encoded = 0;
    if opts.encode_strings {
        encoded = sp.apply(&mut asm, &tokens, &mut rng);
    }
    if opts.rename_locals {
        passes::names::apply(&mut asm, &tokens, &mut rng);
    }
    if opts.encode_strings && encoded > 0 {
        prelude.push_str(&sp.preamble());
    }
    asm.prelude = Some(prelude);

    Ok(Build {
        source: asm.build(),
        seed,
        instructions: 0,
        handlers: 0,
        virtualized: false,
    })
}

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn debug_ir(input: &str) -> Result<String, ProtectError> {
    eprintln!("stage: tokenize");
    let tokens = lexer::tokenize(input)?;
    eprintln!("stage: tokens={}", tokens.len());
    eprintln!("stage: compile");
    let proto = parse::compile(input, &tokens)?;
    eprintln!("stage: compiled");
    Ok(format!("{proto:#?}"))
}
