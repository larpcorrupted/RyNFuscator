use std::env;
use std::fs;
use std::process::ExitCode;

use ryn::{Options, Preset, ProtectError};

const VERSION: &str = env!("CARGO_PKG_VERSION");

const HELP: &str = "\
RyNFuscator — a small, readable Lua source protector

USAGE:
    ryn <input.lua> [options]

OPTIONS:
    -o, --output <path>     Output file (default: <input>.ryn.lua)
    -p, --preset <name>     luau | lua51            (default: luau)
    -s, --seed <hex|dec>    Deterministic seed; accepts 0x1f3a or 7994
        --guard <mode>      roblox | none          (default: roblox on luau)
        --no-guard          Same as --guard none
        --no-strings        Skip string encoding
        --no-names          Skip local renaming
        --no-vm             Skip the bytecode VM (token passes only)
        --keep-debug        Keep line information (it is dropped by default)
    -q, --quiet             Only print errors
    -h, --help              Show this message
    -V, --version           Show version

EXAMPLES:
    ryn main.lua
    ryn main.lua -o dist/main.lua -p lua51
    ryn main.lua --seed 0x1f3a --guard none
";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("ryn: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        print!("{HELP}");
        return Ok(());
    }
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print!("{HELP}");
        return Ok(());
    }
    if args.iter().any(|a| a == "-V" || a == "--version") {
        println!("ryn {VERSION}");
        return Ok(());
    }

    let mut input: Option<String> = None;
    let mut output: Option<String> = None;
    let mut opts = Options::default();
    let mut quiet = false;
    let mut preset_explicit = false;

    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        let mut next = |name: &str| -> Result<String, String> {
            i += 1;
            args.get(i)
                .cloned()
                .ok_or_else(|| format!("{name} needs a value"))
        };

        match a {
            "-o" | "--output" => output = Some(next("--output")?),
            "-p" | "--preset" => {
                let v = next("--preset")?;
                opts.preset = match v.as_str() {
                    "luau" => Preset::Luau,
                    "lua51" => Preset::Lua51,
                    other => return Err(format!("unknown preset `{other}` (luau, lua51)")),
                };
                preset_explicit = true;
            }
            "-s" | "--seed" => {
                let v = next("--seed")?;
                let parsed = if let Some(hex) = v.strip_prefix("0x").or_else(|| v.strip_prefix("0X")) {
                    u64::from_str_radix(hex, 16)
                } else {
                    v.parse::<u64>()
                };
                opts.seed = Some(parsed.map_err(|_| format!("bad seed `{v}`"))?);
            }
            "--guard" => {
                let v = next("--guard")?;
                opts.guard = match v.as_str() {
                    "roblox" => true,
                    "none" => false,
                    other => return Err(format!("unknown guard `{other}` (roblox, none)")),
                };
            }
            "--no-guard" => opts.guard = false,
            "--no-strings" => opts.encode_strings = false,
            "--no-names" => opts.rename_locals = false,
            "--no-vm" => opts.virtualize = false,
            "--keep-debug" => opts.keep_debug = true,
            "-q" | "--quiet" => quiet = true,
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}` (try --help)"))
            }
            other => {
                if input.is_some() {
                    return Err("only one input file".into());
                }
                input = Some(other.to_string());
            }
        }
        i += 1;
    }

    let input = input.ok_or("no input file (try --help)")?;

    if env::var("RYN_DEBUG_IR").is_ok() {
        let s = fs::read_to_string(&input).map_err(|e| e.to_string())?;
        println!("{}", ryn::debug_ir(&s).map_err(|e| e.to_string())?);
        return Ok(());
    }

    // Roblox guard only makes sense for Luau output.
    if !preset_explicit && opts.guard {
        opts.preset = Preset::Luau;
    }
    if opts.preset == Preset::Lua51 {
        opts.guard = false;
    }

    let src = fs::read_to_string(&input).map_err(|e| format!("cannot read {input}: {e}"))?;

    let out = ryn::protect(&src, &opts).map_err(display_err)?;

    let dest = output.unwrap_or_else(|| default_output(&input));
    if let Some(parent) = std::path::Path::new(&dest).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).ok();
        }
    }
    fs::write(&dest, &out).map_err(|e| format!("cannot write {dest}: {e}"))?;

    if !quiet {
        eprintln!(
            "ryn {VERSION}  {}  ->  {}  ({} -> {} bytes)",
            opts.preset.label(),
            dest,
            src.len(),
            out.len()
        );
    }
    Ok(())
}

fn display_err(e: ProtectError) -> String {
    e.to_string()
}

fn default_output(input: &str) -> String {
    let p = std::path::Path::new(input);
    let stem = p
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    match p.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => {
            dir.join(format!("{stem}.ryn.lua")).to_string_lossy().into_owned()
        }
        _ => format!("{stem}.ryn.lua"),
    }
}
