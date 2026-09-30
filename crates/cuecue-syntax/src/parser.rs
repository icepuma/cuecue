//! Recursive-descent parser for CUE edition v1 (CUE spec grammar, plus what CUE's Go parser
//! accepts beyond it). Builds a lossless `rowan` tree and keeps going after errors.

use rowan::{Checkpoint, GreenNode, GreenNodeBuilder};

use crate::SyntaxKind::{self, *};
use crate::{SyntaxNode, Token, lex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxError {
    /// Byte offset where the problem is.
    pub offset: u32,
    pub message: String,
}

pub struct Parse {
    green: GreenNode,
    /// Lexer and parser errors, in source order.
    pub errors: Vec<SyntaxError>,
}

impl Parse {
    pub fn syntax(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.green.clone())
    }

    pub fn tree(&self) -> crate::ast::SourceFile {
        use crate::ast::AstNode;
        crate::ast::SourceFile::cast(self.syntax()).expect("root is a SourceFile")
    }
}

pub fn parse(src: &str) -> Parse {
    let lexed = lex(src);
    let mut p = Parser::new(src, lexed.tokens);
    p.source_file();
    let mut errors: Vec<SyntaxError> = lexed
        .errors
        .into_iter()
        .map(|e| SyntaxError {
            offset: e.offset,
            message: e.message,
        })
        .chain(p.errors)
        .collect();
    errors.sort_by_key(|e| e.offset);
    Parse {
        green: p.builder.finish(),
        errors,
    }
}

struct Parser<'a> {
    src: &'a str,
    tokens: Vec<Token>,
    starts: Vec<usize>,
    /// Indices of non-trivia tokens.
    significant: Vec<usize>,
    /// `implicit[i]`: a newline inserts a comma before `significant[i]` (or before EOF when
    /// `i == significant.len()`).
    implicit: Vec<bool>,
    /// Next significant token, as an index into `significant`.
    cur: usize,
    /// Next token to emit, as an index into `tokens`.
    emitted: usize,
    /// The implicit comma before `significant[cur]` has been consumed.
    implicit_taken: bool,
    /// Current nesting of expressions and fields, capped at `MAX_DEPTH`.
    depth: usize,
    builder: GreenNodeBuilder<'static>,
    errors: Vec<SyntaxError>,
}

/// Deeper input is reported instead of recursing further, so it can't overflow the stack.
/// Real configurations stay far below it.
const MAX_DEPTH: usize = 200;

fn binary_precedence(kind: SyntaxKind) -> u8 {
    match kind {
        Star | Slash => 7,
        Plus | Minus => 6,
        EqEq | BangEq | Lt | LtEq | Gt | GtEq | EqTilde | BangTilde => 5,
        AmpAmp => 4,
        PipePipe => 3,
        Amp => 2,
        Pipe => 1,
        _ => 0,
    }
}

fn is_unary_op(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        Plus | Minus | Bang | Star | BangEq | Lt | LtEq | Gt | GtEq | EqTilde | BangTilde
    )
}

/// Tokens that can name a field in a label.
fn is_label_name(kind: SyntaxKind) -> bool {
    kind == Ident || kind == StringLit || kind.is_keyword()
}

impl<'a> Parser<'a> {
    fn new(src: &'a str, tokens: Vec<Token>) -> Self {
        let mut starts = Vec::with_capacity(tokens.len());
        let mut offset = 0;
        for t in &tokens {
            starts.push(offset);
            offset += t.len as usize;
        }
        let significant: Vec<usize> = (0..tokens.len())
            .filter(|&i| !tokens[i].kind.is_trivia())
            .collect();
        // CUE's scanner turns a newline after a comma-ending token into a comma, unless the
        // next token is `,` or `:`. End of file counts as a newline.
        let mut implicit = vec![false; significant.len() + 1];
        for (n, slot) in implicit.iter_mut().enumerate() {
            let Some(&prev) = n.checked_sub(1).and_then(|p| significant.get(p)) else {
                continue;
            };
            if !tokens[prev].kind.ends_line_with_comma() {
                continue;
            }
            *slot = match significant.get(n) {
                None => true,
                Some(&next) => {
                    !matches!(tokens[next].kind, Comma | Colon)
                        && tokens[prev + 1..next].iter().any(|t| t.kind == Newline)
                }
            };
        }
        Parser {
            src,
            tokens,
            starts,
            significant,
            implicit,
            cur: 0,
            emitted: 0,
            implicit_taken: false,
            depth: 0,
            builder: GreenNodeBuilder::new(),
            errors: Vec::new(),
        }
    }

    // ---- token access ----

    /// The next token as the grammar sees it: an implicit comma, a real token, or `None` at EOF.
    fn current(&self) -> Option<SyntaxKind> {
        if self.implicit[self.cur] && !self.implicit_taken {
            return Some(Comma);
        }
        self.nth(0)
    }

    fn at(&self, kind: SyntaxKind) -> bool {
        self.current() == Some(kind)
    }

    /// The `n`th real significant token from here, ignoring implicit commas.
    fn nth(&self, n: usize) -> Option<SyntaxKind> {
        self.significant
            .get(self.cur + n)
            .map(|&i| self.tokens[i].kind)
    }

    fn offset(&self) -> u32 {
        self.significant
            .get(self.cur)
            .map_or(self.src.len(), |&i| self.starts[i]) as u32
    }

    fn describe(&self) -> String {
        match self.current() {
            None => "'EOF'".to_owned(),
            Some(Comma) if self.implicit[self.cur] && !self.implicit_taken => "newline".to_owned(),
            Some(_) => {
                let i = self.significant[self.cur];
                let text = &self.src[self.starts[i]..self.starts[i] + self.tokens[i].len as usize];
                format!("'{text}'")
            }
        }
    }

    fn error(&mut self, message: impl Into<String>) {
        let offset = self.offset();
        // One error per position is enough; recovery can revisit the same spot.
        if self.errors.last().is_some_and(|e| e.offset == offset) {
            return;
        }
        self.errors.push(SyntaxError {
            offset,
            message: message.into(),
        });
    }

    // ---- tree building ----

    fn emit_trivia(&mut self) {
        let end = self
            .significant
            .get(self.cur)
            .copied()
            .unwrap_or(self.tokens.len());
        while self.emitted < end {
            self.emit(self.emitted);
        }
    }

    fn emit(&mut self, i: usize) {
        let t = self.tokens[i];
        let text = &self.src[self.starts[i]..self.starts[i] + t.len as usize];
        self.builder.token(rowan::SyntaxKind(t.kind as u16), text);
        self.emitted = i + 1;
    }

    fn start(&mut self, kind: SyntaxKind) {
        self.emit_trivia();
        self.builder.start_node(rowan::SyntaxKind(kind as u16));
    }

    fn finish(&mut self) {
        self.builder.finish_node();
    }

    fn checkpoint(&mut self) -> Checkpoint {
        self.emit_trivia();
        self.builder.checkpoint()
    }

    fn start_at(&mut self, checkpoint: Checkpoint, kind: SyntaxKind) {
        self.builder
            .start_node_at(checkpoint, rowan::SyntaxKind(kind as u16));
    }

    /// Consumes the current token (or implicit comma).
    fn bump(&mut self) {
        if self.implicit[self.cur] && !self.implicit_taken {
            self.implicit_taken = true;
            return;
        }
        if let Some(&i) = self.significant.get(self.cur) {
            self.emit_trivia();
            self.emit(i);
            self.cur += 1;
            self.implicit_taken = false;
        }
    }

    fn eat(&mut self, kind: SyntaxKind) -> bool {
        let found = self.at(kind);
        if found {
            self.bump();
        }
        found
    }

    fn expect(&mut self, kind: SyntaxKind, what: &str) {
        if !self.eat(kind) {
            self.error(format!("expected {what}, found {}", self.describe()));
        }
    }

    /// A name being bound (`for k, v`, `let x`); keywords are allowed, as in `for x, import in`.
    fn expect_name(&mut self) {
        if self.current().is_some_and(|k| k == Ident || k.is_keyword()) {
            self.bump();
        } else {
            self.error(format!("expected identifier, found {}", self.describe()));
        }
    }

    /// Wraps the current token in an error node, so parsing always moves forward.
    fn bump_error(&mut self) {
        self.start(ErrorNode);
        self.bump();
        self.finish();
    }

    // ---- grammar ----

    fn source_file(&mut self) {
        // Open the root before any trivia, so leading comments and blank lines land inside it.
        self.builder
            .start_node(rowan::SyntaxKind(SourceFile as u16));
        while self.at(Attribute) {
            self.bump();
            self.separator(None, "file");
        }
        if self.at(PackageKw) && self.nth(1) == Some(Ident) {
            self.start(PackageClause);
            self.bump();
            self.bump();
            self.finish();
            self.separator(None, "file");
        }
        while self.at(ImportKw) && !matches!(self.nth(1), Some(Colon | Question | Bang)) {
            self.import_decl();
            self.separator(None, "file");
        }
        while self.current().is_some() {
            let before = (self.cur, self.implicit_taken);
            if !self.eat(Comma) {
                self.declaration();
                self.separator(None, "file");
            }
            if (self.cur, self.implicit_taken) == before {
                self.bump_error();
            }
        }
        self.emit_trivia();
        self.finish();
    }

    fn import_decl(&mut self) {
        self.start(ImportDecl);
        self.bump();
        if self.eat(LParen) {
            while !self.at(RParen) && self.current().is_some() {
                let before = (self.cur, self.implicit_taken);
                if !self.eat(Comma) {
                    self.import_spec();
                    self.separator(Some(RParen), "import list");
                }
                if (self.cur, self.implicit_taken) == before {
                    self.bump_error();
                }
            }
            self.expect(RParen, "')'");
        } else {
            self.import_spec();
        }
        self.finish();
    }

    fn import_spec(&mut self) {
        self.start(ImportSpec);
        self.eat(Ident);
        self.expect(StringLit, "import path");
        self.finish();
    }

    /// After a declaration or element: a comma, or the closing token, or EOF.
    fn separator(&mut self, close: Option<SyntaxKind>, context: &str) {
        match self.current() {
            Some(Comma) => self.bump(),
            None => {}
            Some(kind) if Some(kind) == close => {}
            Some(_) => self.error(format!("missing ',' in {context}")),
        }
    }

    fn declaration(&mut self) {
        match self.current() {
            _ if self.at_field() => self.field(),
            Some(Ellipsis) => self.ellipsis(),
            Some(Attribute) => self.bump(),
            Some(LetKw) => self.let_clause(),
            Some(ForKw | IfKw) => self.comprehension(),
            _ => self.alias_expr(),
        }
    }

    /// Looks ahead for `label :`, where a label is `[X=] name [?|!]`, `(expr)` or `[expr]`.
    fn at_field(&self) -> bool {
        if self.current() == Some(Comma) {
            return false;
        }
        let mut n = 0;
        if self.nth(0) == Some(Ident) && self.nth(1) == Some(Eq) {
            n = 2;
        }
        match self.nth(n) {
            Some(kind) if is_label_name(kind) => n += 1,
            Some(InterpStart | LParen | LBracket) => match self.skip_group(n) {
                Some(end) => n = end,
                None => return false,
            },
            _ => return false,
        }
        if matches!(self.nth(n), Some(Question | Bang)) {
            n += 1;
        }
        self.nth(n) == Some(Colon)
    }

    /// Skips a bracketed group or interpolated string starting at lookahead `n`, returning
    /// the lookahead index after it.
    fn skip_group(&self, mut n: usize) -> Option<usize> {
        let mut depth = 0usize;
        loop {
            match self.nth(n)? {
                LParen | LBracket | LBrace | InterpStart => depth += 1,
                RParen | RBracket | RBrace | InterpEnd => depth = depth.checked_sub(1)?,
                _ => {}
            }
            n += 1;
            if depth == 0 {
                return Some(n);
            }
        }
    }

    fn field(&mut self) {
        self.start(Field);
        self.label();
        self.expect(Colon, "':'");
        if self.at_field() {
            self.nested(Self::field);
        } else {
            self.alias_expr();
        }
        while self.at(Attribute) {
            self.bump();
        }
        self.finish();
    }

    fn label(&mut self) {
        self.start(Label);
        if self.at(Ident) && self.nth(1) == Some(Eq) {
            self.bump();
            self.bump();
        }
        match self.current() {
            Some(InterpStart) => self.interpolation(),
            Some(LParen) => {
                self.bump();
                self.alias_expr();
                self.expect(RParen, "')'");
            }
            Some(LBracket) => {
                self.bump();
                self.alias_expr();
                self.expect(RBracket, "']'");
            }
            _ => self.bump(),
        }
        if matches!(self.current(), Some(Question | Bang)) {
            self.bump();
        }
        self.finish();
    }

    fn ellipsis(&mut self) {
        self.start(EllipsisExpr);
        self.bump();
        if self.at_expr_start() {
            self.expr();
        }
        self.finish();
    }

    fn let_clause(&mut self) {
        self.start(LetClause);
        self.bump();
        self.expect_name();
        self.expect(Eq, "'='");
        self.expr();
        self.finish();
    }

    fn comprehension(&mut self) {
        self.start(Comprehension);
        loop {
            match self.current() {
                Some(ForKw) => {
                    self.start(ForClause);
                    self.bump();
                    self.expect_name();
                    // An explicit comma separates key and value; an implicit one can't.
                    if self.nth(0) == Some(Comma) && self.at(Comma) {
                        self.bump();
                        self.expect_name();
                    }
                    self.expect(InKw, "'in'");
                    self.expr();
                    self.finish();
                }
                Some(IfKw) => {
                    self.start(IfClause);
                    self.bump();
                    self.expr();
                    self.finish();
                }
                Some(LetKw) => self.let_clause(),
                Some(Comma) => self.bump(),
                _ => break,
            }
        }
        if self.at(LBrace) {
            self.struct_lit();
        } else {
            self.error(format!("expected '{{', found {}", self.describe()));
        }
        self.finish();
    }

    fn struct_lit(&mut self) {
        self.start(StructLit);
        self.bump();
        while !self.at(RBrace) && self.current().is_some() {
            let before = (self.cur, self.implicit_taken);
            if !self.eat(Comma) {
                self.declaration();
                self.separator(Some(RBrace), "struct literal");
            }
            if (self.cur, self.implicit_taken) == before {
                self.bump_error();
            }
        }
        self.expect(RBrace, "'}'");
        self.finish();
    }

    fn list_lit(&mut self) {
        self.start(ListLit);
        self.bump();
        while !self.at(RBracket) && self.current().is_some() {
            let before = (self.cur, self.implicit_taken);
            if !self.eat(Comma) {
                match self.current() {
                    Some(Ellipsis) => self.ellipsis(),
                    Some(ForKw | IfKw) => self.comprehension(),
                    _ => self.alias_expr(),
                }
                self.separator(Some(RBracket), "list literal");
            }
            if (self.cur, self.implicit_taken) == before {
                self.bump_error();
            }
        }
        self.expect(RBracket, "']'");
        self.finish();
    }

    fn alias_expr(&mut self) {
        if self.at(Ident) && self.nth(1) == Some(Eq) {
            self.start(Alias);
            self.bump();
            self.bump();
            self.expr();
            self.finish();
        } else {
            self.expr();
        }
    }

    fn at_expr_start(&self) -> bool {
        match self.current() {
            None | Some(Comma | RParen | RBracket | RBrace | Colon | Attribute) => false,
            Some(_) => true,
        }
    }

    fn expr(&mut self) {
        self.nested(|p| p.binary(0));
    }

    /// Runs `f` one level deeper, or reports the input as too deeply nested.
    fn nested(&mut self, f: impl FnOnce(&mut Self)) {
        if self.depth >= MAX_DEPTH {
            self.error("expression nested too deeply");
            if self.current().is_some() && !self.at(Comma) {
                self.bump_error();
            }
            return;
        }
        self.depth += 1;
        f(self);
        self.depth -= 1;
    }

    fn binary(&mut self, min: u8) {
        let checkpoint = self.checkpoint();
        self.unary();
        loop {
            let prec = self.current().map_or(0, binary_precedence);
            if prec <= min {
                break;
            }
            self.start_at(checkpoint, BinaryExpr);
            self.bump();
            self.binary(prec);
            self.finish();
        }
    }

    fn unary(&mut self) {
        if self.current().is_some_and(is_unary_op) {
            self.start(UnaryExpr);
            self.bump();
            self.nested(Self::unary);
            self.finish();
        } else {
            self.postfix();
        }
    }

    fn postfix(&mut self) {
        let checkpoint = self.checkpoint();
        self.operand();
        loop {
            match self.current() {
                Some(Dot) => {
                    self.start_at(checkpoint, SelectorExpr);
                    self.bump();
                    if self.current().is_some_and(is_label_name) {
                        self.bump();
                    } else {
                        self.error(format!("expected selector, found {}", self.describe()));
                    }
                    self.finish();
                }
                Some(LBracket) => {
                    // `x[i]` or `x[i:j]`: the node kind is known only after the `:`, so wrap
                    // the parsed tokens afterwards.
                    self.bump();
                    if !self.at(Colon) {
                        self.expr();
                    }
                    let slice = self.eat(Colon);
                    if slice && !self.at(RBracket) {
                        self.expr();
                    }
                    self.expect(RBracket, "']'");
                    self.start_at(checkpoint, if slice { SliceExpr } else { IndexExpr });
                    self.finish();
                }
                Some(LParen) => {
                    self.start_at(checkpoint, CallExpr);
                    self.start(ArgList);
                    self.bump();
                    while !self.at(RParen) && self.current().is_some() {
                        let before = (self.cur, self.implicit_taken);
                        if !self.eat(Comma) {
                            self.expr();
                            self.separator(Some(RParen), "argument list");
                        }
                        if (self.cur, self.implicit_taken) == before {
                            self.bump_error();
                        }
                    }
                    self.expect(RParen, "')'");
                    self.finish();
                    self.finish();
                }
                _ => break,
            }
        }
    }

    fn operand(&mut self) {
        match self.current() {
            Some(Int | Float | StringLit | Bottom | NullKw | TrueKw | FalseKw) => {
                self.start(Literal);
                self.bump();
                self.finish();
            }
            Some(kind) if kind == Ident || kind.is_keyword() => {
                self.start(Name);
                self.bump();
                self.finish();
            }
            Some(InterpStart) => self.interpolation(),
            Some(LParen) => {
                self.start(ParenExpr);
                self.bump();
                self.alias_expr();
                self.expect(RParen, "')'");
                self.finish();
            }
            Some(LBrace) => self.struct_lit(),
            Some(LBracket) => self.list_lit(),
            _ => {
                self.error(format!("expected operand, found {}", self.describe()));
                // Leave closers and separators to the enclosing construct.
                if self.at_expr_start() && !self.at(Comma) {
                    self.bump_error();
                }
            }
        }
    }

    fn interpolation(&mut self) {
        self.start(Interpolation);
        self.bump();
        loop {
            self.expr();
            match self.current() {
                Some(InterpMid) => self.bump(),
                Some(InterpEnd) => {
                    self.bump();
                    break;
                }
                _ => {
                    self.error(format!(
                        "expected ')' in interpolation, found {}",
                        self.describe()
                    ));
                    break;
                }
            }
        }
        self.finish();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parses `src`, asserts losslessness and no errors, and returns the tree without trivia
    /// in a compact form.
    fn tree(src: &str) -> String {
        let parse = parse(src);
        assert_eq!(parse.errors, [], "{src}");
        let root = parse.syntax();
        assert_eq!(root.text().to_string(), src);
        let mut out = String::new();
        fn walk(node: &SyntaxNode, out: &mut String) {
            out.push_str(&format!("{:?}(", node.kind()));
            let mut first = true;
            for child in node.children_with_tokens() {
                match child {
                    rowan::NodeOrToken::Node(n) => {
                        if !first {
                            out.push(' ');
                        }
                        walk(&n, out);
                    }
                    rowan::NodeOrToken::Token(t) if !t.kind().is_trivia() => {
                        if !first {
                            out.push(' ');
                        }
                        out.push_str(t.text());
                    }
                    _ => continue,
                }
                first = false;
            }
            out.push(')');
        }
        walk(&root, &mut out);
        out
    }

    fn errors(src: &str) -> Vec<String> {
        parse(src).errors.into_iter().map(|e| e.message).collect()
    }

    #[test]
    fn fields_and_precedence() {
        assert_eq!(
            tree("a: 1 + 2 * 3 | *int"),
            "SourceFile(Field(Label(a) : BinaryExpr(BinaryExpr(Literal(1) + BinaryExpr(Literal(2) * Literal(3))) | UnaryExpr(* Name(int)))))"
        );
        assert_eq!(
            tree("a: b: c"),
            "SourceFile(Field(Label(a) : Field(Label(b) : Name(c))))"
        );
    }

    #[test]
    fn labels() {
        assert_eq!(
            tree("X=\"a b\"?: 1\n[N=string]: {name: N}\n(k)!: 2\nif: 3"),
            "SourceFile(Field(Label(X = \"a b\" ?) : Literal(1)) Field(Label([ Alias(N = Name(string)) ]) : StructLit({ Field(Label(name) : Name(N)) })) Field(Label(( Name(k) ) !) : Literal(2)) Field(Label(if) : Literal(3)))"
        );
    }

    #[test]
    fn newlines_are_commas() {
        assert_eq!(
            tree("a: x\n-1\nl: [\n1\n2\n]"),
            "SourceFile(Field(Label(a) : Name(x)) UnaryExpr(- Literal(1)) Field(Label(l) : ListLit([ Literal(1) Literal(2) ])))"
        );
        // A newline after an operator continues the expression.
        assert_eq!(
            tree("a: 1 +\n2"),
            "SourceFile(Field(Label(a) : BinaryExpr(Literal(1) + Literal(2))))"
        );
    }

    #[test]
    fn preamble_and_comprehensions() {
        assert_eq!(
            tree("x: [for k, import in y {import}]"),
            "SourceFile(Field(Label(x) : ListLit([ Comprehension(ForClause(for k , import in Name(y)) StructLit({ Name(import) })) ])))"
        );
        assert_eq!(
            tree(
                "@a(x)\npackage p\nimport (\n\t\"strings\"\n\tl \"list\"\n)\nfor k, v in x if v let y = v {(k): y}"
            ),
            "SourceFile(@a(x) PackageClause(package p) ImportDecl(import ( ImportSpec(\"strings\") ImportSpec(l \"list\") )) Comprehension(ForClause(for k , v in Name(x)) IfClause(if Name(v)) LetClause(let y = Name(v)) StructLit({ Field(Label(( Name(k) )) : Name(y)) })))"
        );
    }

    #[test]
    fn primaries() {
        assert_eq!(
            tree("a: x.y.#z[0][1:](p, \"s\\(q)t\")"),
            "SourceFile(Field(Label(a) : CallExpr(SliceExpr(IndexExpr(SelectorExpr(SelectorExpr(Name(x) . y) . #z) [ Literal(0) ]) [ Literal(1) : ]) ArgList(( Name(p) , Interpolation(\"s\\( Name(q) )t\") )))))"
        );
    }

    #[test]
    fn leading_trivia_stays_in_the_tree() {
        assert_eq!(
            tree("\t// c\n\na: 1"),
            "SourceFile(Field(Label(a) : Literal(1)))"
        );
    }

    #[test]
    fn errors_are_reported_and_recovered() {
        assert_eq!(errors("a: {"), ["expected '}', found 'EOF'"]);
        assert_eq!(errors("a: 1 +"), ["expected operand, found 'EOF'"]);
        assert_eq!(errors("a: 1 ]\nb: 2"), ["missing ',' in file"]);
        for deep in [
            "a: ".to_owned() + &"(".repeat(100_000),
            "-".repeat(100_000),
            "a:".repeat(100_000),
        ] {
            assert!(errors(&deep).contains(&"expression nested too deeply".to_owned()));
        }
        // Found by fuzzing: a bad import spec used to loop forever.
        assert!(!errors("import (\n\t\"stri\n\"n)\nx").is_empty());
        let parse = parse("a: ) b: 2");
        assert!(!parse.errors.is_empty());
        assert_eq!(parse.syntax().text().to_string(), "a: ) b: 2");
    }
}
