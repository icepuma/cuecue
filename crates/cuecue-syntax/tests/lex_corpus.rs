//! Every `.cue` file in the v1 corpus lexes losslessly, and without errors unless the oracle
//! reports a syntax error for its archive (ROADMAP M1.1).

use std::fs;
use std::path::{Path, PathBuf};

fn archives(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            archives(&path, out);
        } else if path.extension().is_some_and(|e| e == "txtar") {
            out.push(path);
        }
    }
}

/// (name, content) of each txtar section.
fn sections(text: &str) -> Vec<(&str, String)> {
    let mut out: Vec<(&str, String)> = Vec::new();
    for line in text.split_inclusive('\n') {
        match line
            .trim_end_matches('\n')
            .strip_prefix("-- ")
            .and_then(|l| l.strip_suffix(" --"))
        {
            Some(name) => out.push((name.trim(), String::new())),
            None => {
                if let Some((_, body)) = out.last_mut() {
                    body.push_str(line);
                }
            }
        }
    }
    out
}

#[test]
fn corpus_lexes() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/v1");
    let mut paths = Vec::new();
    archives(&dir, &mut paths);
    assert!(paths.len() > 400, "corpus not found at {}", dir.display());

    let mut failures = Vec::new();
    for path in paths {
        let text = fs::read_to_string(&path).unwrap();
        let sections = sections(&text);
        if sections.iter().any(|(name, _)| *name == "oracle/skip") {
            continue;
        }
        let syntax_error = sections.iter().any(|(name, body)| {
            *name == "oracle/error" && body.lines().any(|l| l.starts_with("syntax "))
        });
        for (name, src) in sections.iter().filter(|(n, _)| n.ends_with(".cue")) {
            let lexed = cuecue_syntax::lex(src);
            let len: u32 = lexed.tokens.iter().map(|t| t.len).sum();
            assert_eq!(
                len as usize,
                src.len(),
                "{}: {name} not lossless",
                path.display()
            );
            if !syntax_error && !lexed.errors.is_empty() {
                failures.push(format!(
                    "{} {name}: {:?}",
                    path.strip_prefix(&dir).unwrap().display(),
                    lexed.errors
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} files with lex errors:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
