//! Every `.cue` file in the v1 corpus lexes and parses losslessly, and without errors unless
//! the oracle reports a syntax error for its archive (ROADMAP M1.1, M1.2).

use std::collections::BTreeSet;
use std::fs;

use cuecue_syntax::ast::*;
use cuecue_syntax::{SyntaxKind, SyntaxNode};
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

struct Case {
    archive: String,
    /// The oracle reports a syntax error for this archive.
    syntax_error: bool,
    /// (name, source) of its `.cue` files.
    files: Vec<(String, String)>,
}

/// Every archive the oracle didn't skip.
fn cases() -> Vec<Case> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/v1");
    let mut paths = Vec::new();
    archives(&dir, &mut paths);
    assert!(paths.len() > 400, "corpus not found at {}", dir.display());
    paths.sort();
    let mut out = Vec::new();
    for path in paths {
        let text = fs::read_to_string(&path).unwrap();
        let sections = sections(&text);
        if sections.iter().any(|(name, _)| *name == "oracle/skip") {
            continue;
        }
        let syntax_error = sections.iter().any(|(name, body)| {
            *name == "oracle/error" && body.lines().any(|l| l.starts_with("syntax "))
        });
        let files = sections
            .into_iter()
            .filter(|(n, _)| n.ends_with(".cue"))
            .map(|(n, body)| (n.to_owned(), body))
            .collect();
        let archive = path.strip_prefix(&dir).unwrap().display().to_string();
        out.push(Case {
            archive,
            syntax_error,
            files,
        });
    }
    out
}

#[test]
fn corpus_parses() {
    let mut failures = Vec::new();
    for Case {
        archive,
        syntax_error,
        files,
    } in cases()
    {
        for (name, src) in &files {
            let parse = cuecue_syntax::parse(src);
            assert_eq!(
                parse.syntax().text().to_string(),
                *src,
                "{archive} {name}: not lossless"
            );
            if !syntax_error && !parse.errors.is_empty() {
                failures.push(format!("{archive} {name}: {:?}", parse.errors));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} files with parse errors:\n{}",
        failures.len(),
        failures.join("\n")
    );
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

/// Records a node reached through the typed API.
type Seen = BTreeSet<(u32, u32, SyntaxKind)>;

fn see(seen: &mut Seen, node: &SyntaxNode) {
    let r = node.text_range();
    seen.insert((r.start().into(), r.end().into(), node.kind()));
}

fn visit_decl(seen: &mut Seen, decl: Decl) {
    match decl {
        Decl::Field(f) => visit_field(seen, f),
        Decl::Embedding(e) => visit_expr(seen, e),
        Decl::Comprehension(c) => visit_comprehension(seen, c),
        Decl::Let(l) => visit_let(seen, l),
        Decl::Ellipsis(e) => visit_ellipsis(seen, e),
        Decl::Attribute(_) => {}
    }
}

fn visit_field(seen: &mut Seen, field: Field) {
    see(seen, field.syntax());
    let label = field.label().unwrap();
    see(seen, label.syntax());
    let _ = (label.alias(), label.marker());
    match label.name().unwrap() {
        LabelName::Static(_) => {}
        LabelName::Interpolation(i) => visit_expr(seen, Expr::Interpolation(i)),
        LabelName::Dynamic(e) | LabelName::Pattern(e) => visit_expr(seen, e),
    }
    match field.value().unwrap() {
        FieldValue::Field(f) => visit_field(seen, f),
        FieldValue::Expr(e) => visit_expr(seen, e),
    }
}

fn visit_let(seen: &mut Seen, l: LetClause) {
    see(seen, l.syntax());
    l.name().unwrap();
    visit_expr(seen, l.value().unwrap());
}

fn visit_ellipsis(seen: &mut Seen, e: EllipsisExpr) {
    see(seen, e.syntax());
    if let Some(t) = e.expr() {
        visit_expr(seen, t);
    }
}

fn visit_comprehension(seen: &mut Seen, c: Comprehension) {
    see(seen, c.syntax());
    for clause in c.clauses() {
        match clause {
            Clause::For(f) => {
                see(seen, f.syntax());
                f.value().unwrap();
                let _ = f.key();
                visit_expr(seen, f.source().unwrap());
            }
            Clause::If(i) => {
                see(seen, i.syntax());
                visit_expr(seen, i.condition().unwrap());
            }
            Clause::Let(l) => visit_let(seen, l),
        }
    }
    visit_expr(seen, Expr::Struct(c.body().unwrap()));
}

fn visit_expr(seen: &mut Seen, expr: Expr) {
    see(seen, expr.syntax());
    let mut sub = |e: Option<Expr>| visit_expr(seen, e.expect("sub-expression"));
    match expr {
        Expr::Literal(l) => drop(l.token().unwrap()),
        Expr::Name(n) => drop(n.token().unwrap()),
        Expr::Interpolation(i) => {
            assert!(i.pieces().count() >= 2);
            for e in i.exprs() {
                sub(Some(e));
            }
        }
        Expr::Paren(p) => sub(p.expr()),
        Expr::Unary(u) => {
            u.op().unwrap();
            sub(u.operand());
        }
        Expr::Binary(b) => {
            b.op().unwrap();
            sub(b.lhs());
            sub(b.rhs());
        }
        Expr::Selector(s) => {
            s.name().unwrap();
            sub(s.operand());
        }
        Expr::Index(i) => {
            sub(i.operand());
            sub(i.index());
        }
        Expr::Slice(s) => {
            let (lo, hi) = s.bounds();
            sub(s.operand());
            for bound in [lo, hi].into_iter().flatten() {
                sub(Some(bound));
            }
        }
        Expr::Call(c) => {
            sub(c.callee());
            for a in c.args() {
                sub(Some(a));
            }
        }
        Expr::Struct(s) => {
            for d in s.decls() {
                visit_decl(seen, d);
            }
        }
        Expr::List(l) => {
            for element in l.elements() {
                match element {
                    Element::Expr(e) => visit_expr(seen, e),
                    Element::Comprehension(c) => visit_comprehension(seen, c),
                    Element::Ellipsis(e) => visit_ellipsis(seen, e),
                }
            }
        }
        Expr::Alias(a) => {
            a.name().unwrap();
            sub(a.expr());
        }
    }
}

/// The typed API reaches every node of every corpus tree (ROADMAP M1.3).
#[test]
fn typed_ast_covers_the_corpus() {
    let mut checked = 0;
    for case in cases().into_iter().filter(|c| !c.syntax_error) {
        for (name, src) in &case.files {
            let parse = cuecue_syntax::parse(src);
            let tree = parse.tree();
            let mut seen = Seen::new();
            if let Some(p) = tree.package() {
                see(&mut seen, p.syntax());
                p.name().unwrap();
            }
            for import in tree.imports() {
                see(&mut seen, import.syntax());
                for spec in import.specs() {
                    see(&mut seen, spec.syntax());
                    spec.path().unwrap();
                }
            }
            for decl in tree.decls() {
                visit_decl(&mut seen, decl);
            }
            let mut all = Seen::new();
            for node in tree.syntax().descendants() {
                if !matches!(node.kind(), SyntaxKind::SourceFile | SyntaxKind::ArgList) {
                    see(&mut all, &node);
                }
            }
            let missed: Vec<_> = all.difference(&seen).collect();
            assert!(
                missed.is_empty(),
                "{} {name}: unreached nodes {missed:?}",
                case.archive
            );
            checked += 1;
        }
    }
    assert!(checked > 400);
}
