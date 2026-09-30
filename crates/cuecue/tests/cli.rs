use std::process::Command;

#[test]
fn prints_version() {
    let out = Command::new(env!("CARGO_BIN_EXE_cuecue"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        format!("cuecue {}\n", env!("CARGO_PKG_VERSION"))
    );
}

fn parse_file(name: &str, src: &str, tree: bool) -> std::process::Output {
    let path = std::env::temp_dir().join(format!("cuecue-cli-{}-{name}", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cuecue"));
    cmd.arg("parse");
    if tree {
        cmd.arg("--tree");
    }
    let out = cmd.arg(&path).output().unwrap();
    std::fs::remove_file(&path).unwrap();
    out
}

#[test]
fn parse_prints_the_tree() {
    let out = parse_file("ok.cue", "a: 1\n", true);
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.starts_with("SourceFile@0..5\n"), "{stdout}");
    assert!(stdout.contains("Field@0..4"), "{stdout}");
}

#[test]
fn parse_reports_syntax_errors() {
    let out = parse_file("bad.cue", "a: {\n", false);
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.ends_with("bad.cue:2:1: expected '}', found 'EOF'\n"),
        "{stderr}"
    );
}
