#![no_main]

// Parsing never panics or hangs, and the tree reproduces the input exactly.
libfuzzer_sys::fuzz_target!(|src: &str| {
    let parse = cuecue_syntax::parse(src);
    assert_eq!(parse.syntax().text().to_string(), src);
});
