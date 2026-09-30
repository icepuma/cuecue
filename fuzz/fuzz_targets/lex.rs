#![no_main]

// Lexing never panics or hangs, and the tokens cover the input exactly.
libfuzzer_sys::fuzz_target!(|src: &str| {
    let lexed = cuecue_syntax::lex(src);
    let len: usize = lexed.tokens.iter().map(|t| t.len as usize).sum();
    assert_eq!(len, src.len());
});
