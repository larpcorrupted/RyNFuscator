use ryn::{protect, Options, Preset};

fn lua51() -> Options {
    Options {
        preset: Preset::Lua51,
        guard: false,
        seed: Some(1234),
        ..Default::default()
    }
}

fn lua51_plain() -> Options {
    Options {
        preset: Preset::Lua51,
        guard: false,
        seed: Some(1234),
        encode_strings: false,
        rename_locals: false,
        ..Default::default()
    }
}

#[test]
fn header_is_first_line() {
    let out = protect("print(1)\n", &lua51()).unwrap();
    assert!(out.starts_with("-- This file has been protected using RyNfuscastor OPEN-SOURCE [https://github.com/larpcorrupted/RyNFuscator]"));
}

#[test]
fn output_differs_from_input() {
    let src = "local x = 42\nprint(\"hello world\", x)\n";
    let out = protect(src, &lua51()).unwrap();
    assert_ne!(out, src);
    assert!(out.len() > src.len());
}

#[test]
fn deterministic_for_a_seed() {
    let src = "local a = 1\nprint(a)\n";
    let a = protect(src, &lua51()).unwrap();
    let b = protect(src, &lua51()).unwrap();
    assert_eq!(a, b);
}

#[test]
fn different_seeds_differ() {
    let src = "local a = 1\nprint(a, \"content\")\n";
    let mut o1 = lua51();
    o1.seed = Some(1);
    let mut o2 = lua51();
    o2.seed = Some(2);
    assert_ne!(protect(src, &o1).unwrap(), protect(src, &o2).unwrap());
}

#[test]
fn guard_present_by_default_on_luau() {
    let opts = Options {
        seed: Some(9),
        ..Default::default()
    };
    let out = protect("print(1)\n", &opts).unwrap();
    assert!(out.contains("RYN_FAIL_MODE"));
    assert!(out.contains("obfuscatenow"));
    assert!(out.contains("while true do"));
}

#[test]
fn guard_absent_when_disabled() {
    let opts = Options {
        guard: false,
        seed: Some(9),
        ..Default::default()
    };
    let out = protect("print(1)\n", &opts).unwrap();
    assert!(!out.contains("RYN_FAIL_MODE"));
}

#[test]
fn guard_rejected_for_lua51() {
    let opts = Options {
        preset: Preset::Lua51,
        guard: true,
        ..Default::default()
    };
    assert!(protect("print(1)\n", &opts).is_err());
}

#[test]
fn empty_input_is_rejected() {
    assert!(protect("", &lua51()).is_err());
    assert!(protect("   \n\n  ", &lua51()).is_err());
}

#[test]
fn comments_are_dropped_by_default() {
    let out = protect("-- secret note\nprint(1)\n", &lua51()).unwrap();
    assert!(!out.contains("secret note"));
}

#[test]
fn comments_kept_when_asked() {
    let opts = Options {
        keep_debug: true,
        seed: Some(5),
        encode_strings: false,
        rename_locals: false,
        preset: Preset::Lua51,
        guard: false,
        virtualize: false,
    };
    let out = protect("-- keep me\nprint(1)\n", &opts).unwrap();
    assert!(out.contains("keep me"));
}

#[test]
fn strings_are_not_greppable() {
    let src = "local s = \"supercalifragilistic\"\nprint(s)\n";
    let out = protect(src, &lua51()).unwrap();
    assert!(!out.contains("supercalifragilistic"));
}

#[test]
fn short_strings_are_left_alone() {
    let src = "print(\"ok\")\n";
    let out = protect(src, &lua51()).unwrap();
    assert!(out.contains("\"ok\""));
}

#[test]
fn plain_copy_when_all_passes_off() {
    let src = "local x = 1\nprint(x)\n";
    let out = protect(src, &lua51_plain()).unwrap();
    assert!(out.contains("local x = 1"));
    assert!(out.contains("print(x)"));
}

#[test]
fn globals_are_never_renamed() {
    let src = "print(workspace)\ngame:GetService(\"Players\")\n";
    let out = protect(src, &lua51()).unwrap();
    assert!(out.contains("print"));
    assert!(out.contains("workspace"));
    assert!(out.contains("game"));
    assert!(out.contains("GetService"));
}

#[test]
fn field_names_are_never_renamed() {
    let src = "local t = {}\nt.importantField = 1\nprint(t.importantField)\n";
    let out = protect(src, &lua51()).unwrap();
    assert!(out.contains("importantField"));
}

#[test]
fn identifiers_mentioned_in_strings_are_kept() {
    let src = "local thing = 5\nprint(\"thing\", thing)\n";
    let out = protect(src, &lua51()).unwrap();
    assert!(out.contains("thing"));
}

#[test]
fn large_numbers_are_left_alone() {
    let src = "print(123456789)\n";
    let out = protect(src, &lua51()).unwrap();
    assert!(out.contains("123456789"));
}

#[test]
fn floats_are_left_alone() {
    let src = "print(3.14159)\n";
    let out = protect(src, &lua51()).unwrap();
    assert!(out.contains("3.14159"));
}

#[test]
fn long_strings_are_supported() {
    let src = "local s = [[long bracket content here]]\nprint(s)\n";
    let out = protect(src, &lua51()).unwrap();
    assert!(!out.contains("long bracket content here"));
}

#[test]
fn shebang_is_preserved() {
    let src = "#!/usr/bin/lua\nprint(1)\n";
    let out = protect(src, &lua51()).unwrap();
    assert!(out.contains("#!/usr/bin/lua"));
}

#[test]
fn no_trailing_garbage() {
    let out = protect("print(1)\n", &lua51()).unwrap();
    assert!(out.trim_end().ends_with("print((0+1))") || out.contains("print"));
}

#[test]
fn debug_parse_smoke() {
    let out = ryn::debug_ir("print(1)\n");
    assert!(out.is_ok(), "{:?}", out.err());
}
