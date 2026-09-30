//! Typed views over the syntax tree, one per grammar production (ROADMAP M1.3). They never
//! own data; each wraps a `SyntaxNode` and finds its parts on demand, returning `None` where
//! the parser recovered from an error.

use crate::SyntaxKind::{self, *};
use crate::{SyntaxNode, SyntaxToken};

pub trait AstNode: Sized {
    fn cast(node: SyntaxNode) -> Option<Self>;
    fn syntax(&self) -> &SyntaxNode;
}

macro_rules! nodes {
    ($($name:ident),* $(,)?) => {$(
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name(SyntaxNode);

        impl AstNode for $name {
            fn cast(node: SyntaxNode) -> Option<Self> {
                (node.kind() == SyntaxKind::$name).then(|| Self(node))
            }

            fn syntax(&self) -> &SyntaxNode {
                &self.0
            }
        }
    )*};
}

nodes!(
    SourceFile,
    PackageClause,
    ImportDecl,
    ImportSpec,
    Field,
    Label,
    Alias,
    EllipsisExpr,
    Comprehension,
    ForClause,
    IfClause,
    LetClause,
    StructLit,
    ListLit,
    Name,
    Literal,
    Interpolation,
    ParenExpr,
    UnaryExpr,
    BinaryExpr,
    SelectorExpr,
    IndexExpr,
    SliceExpr,
    CallExpr,
);

fn child<N: AstNode>(node: &SyntaxNode) -> Option<N> {
    node.children().find_map(N::cast)
}

fn children<N: AstNode>(node: &SyntaxNode) -> impl Iterator<Item = N> {
    node.children().filter_map(N::cast)
}

fn tokens(node: &SyntaxNode) -> impl Iterator<Item = SyntaxToken> {
    node.children_with_tokens()
        .filter_map(|e| e.into_token())
        .filter(|t| !t.kind().is_trivia())
}

fn token(node: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxToken> {
    tokens(node).find(|t| t.kind() == kind)
}

fn is_name(kind: SyntaxKind) -> bool {
    kind == Ident || kind.is_keyword()
}

/// Any expression.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    Literal(Literal),
    Name(Name),
    Interpolation(Interpolation),
    Paren(ParenExpr),
    Unary(UnaryExpr),
    Binary(BinaryExpr),
    Selector(SelectorExpr),
    Index(IndexExpr),
    Slice(SliceExpr),
    Call(CallExpr),
    Struct(StructLit),
    List(ListLit),
    /// `X=expr`, where an alias is allowed.
    Alias(Alias),
}

impl AstNode for Expr {
    fn cast(node: SyntaxNode) -> Option<Self> {
        Some(match node.kind() {
            SyntaxKind::Literal => Expr::Literal(Literal(node)),
            SyntaxKind::Name => Expr::Name(Name(node)),
            SyntaxKind::Interpolation => Expr::Interpolation(Interpolation(node)),
            SyntaxKind::ParenExpr => Expr::Paren(ParenExpr(node)),
            SyntaxKind::UnaryExpr => Expr::Unary(UnaryExpr(node)),
            SyntaxKind::BinaryExpr => Expr::Binary(BinaryExpr(node)),
            SyntaxKind::SelectorExpr => Expr::Selector(SelectorExpr(node)),
            SyntaxKind::IndexExpr => Expr::Index(IndexExpr(node)),
            SyntaxKind::SliceExpr => Expr::Slice(SliceExpr(node)),
            SyntaxKind::CallExpr => Expr::Call(CallExpr(node)),
            SyntaxKind::StructLit => Expr::Struct(StructLit(node)),
            SyntaxKind::ListLit => Expr::List(ListLit(node)),
            SyntaxKind::Alias => Expr::Alias(Alias(node)),
            _ => return None,
        })
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            Expr::Literal(n) => n.syntax(),
            Expr::Name(n) => n.syntax(),
            Expr::Interpolation(n) => n.syntax(),
            Expr::Paren(n) => n.syntax(),
            Expr::Unary(n) => n.syntax(),
            Expr::Binary(n) => n.syntax(),
            Expr::Selector(n) => n.syntax(),
            Expr::Index(n) => n.syntax(),
            Expr::Slice(n) => n.syntax(),
            Expr::Call(n) => n.syntax(),
            Expr::Struct(n) => n.syntax(),
            Expr::List(n) => n.syntax(),
            Expr::Alias(n) => n.syntax(),
        }
    }
}

/// A declaration in a file or struct literal.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Decl {
    Field(Field),
    Embedding(Expr),
    Comprehension(Comprehension),
    Let(LetClause),
    Ellipsis(EllipsisExpr),
    Attribute(SyntaxToken),
}

fn decls(node: &SyntaxNode) -> impl Iterator<Item = Decl> {
    node.children_with_tokens()
        .filter_map(|element| match element {
            rowan::NodeOrToken::Token(t) => (t.kind() == Attribute).then_some(Decl::Attribute(t)),
            rowan::NodeOrToken::Node(n) => match n.kind() {
                SyntaxKind::Field => Some(Decl::Field(Field(n))),
                SyntaxKind::Comprehension => Some(Decl::Comprehension(Comprehension(n))),
                SyntaxKind::LetClause => Some(Decl::Let(LetClause(n))),
                SyntaxKind::EllipsisExpr => Some(Decl::Ellipsis(EllipsisExpr(n))),
                _ => Expr::cast(n).map(Decl::Embedding),
            },
        })
}

impl SourceFile {
    pub fn package(&self) -> Option<PackageClause> {
        child(&self.0)
    }

    pub fn imports(&self) -> impl Iterator<Item = ImportDecl> {
        children(&self.0)
    }

    /// Everything after the preamble, plus attributes before the package clause.
    pub fn decls(&self) -> impl Iterator<Item = Decl> {
        decls(&self.0)
    }
}

impl PackageClause {
    pub fn name(&self) -> Option<SyntaxToken> {
        token(&self.0, Ident)
    }
}

impl ImportDecl {
    pub fn specs(&self) -> impl Iterator<Item = ImportSpec> {
        children(&self.0)
    }
}

impl ImportSpec {
    /// The explicit package name, as in `import l "list"`.
    pub fn name(&self) -> Option<SyntaxToken> {
        token(&self.0, Ident)
    }

    pub fn path(&self) -> Option<SyntaxToken> {
        token(&self.0, StringLit)
    }
}

/// What a field's colon is followed by.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FieldValue {
    /// The next field of a shorthand chain `a: b: c`.
    Field(Field),
    Expr(Expr),
}

impl Field {
    pub fn label(&self) -> Option<Label> {
        child(&self.0)
    }

    pub fn value(&self) -> Option<FieldValue> {
        self.0.children().find_map(|n| match n.kind() {
            SyntaxKind::Field => Some(FieldValue::Field(Field(n))),
            _ => Expr::cast(n).map(FieldValue::Expr),
        })
    }

    pub fn attributes(&self) -> impl Iterator<Item = SyntaxToken> {
        tokens(&self.0).filter(|t| t.kind() == Attribute)
    }
}

/// The name part of a label.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LabelName {
    /// An identifier, keyword or plain string.
    Static(SyntaxToken),
    /// An interpolated string.
    Interpolation(Interpolation),
    /// `(expr)`
    Dynamic(Expr),
    /// `[expr]`
    Pattern(Expr),
}

impl Label {
    /// `X` in `X=name: v`.
    pub fn alias(&self) -> Option<SyntaxToken> {
        token(&self.0, Eq)?;
        tokens(&self.0).next().filter(|t| t.kind() == Ident)
    }

    pub fn name(&self) -> Option<LabelName> {
        if let Some(interpolation) = child(&self.0) {
            return Some(LabelName::Interpolation(interpolation));
        }
        let expr = || child::<Expr>(&self.0);
        if token(&self.0, LParen).is_some() {
            return expr().map(LabelName::Dynamic);
        }
        if token(&self.0, LBracket).is_some() {
            return expr().map(LabelName::Pattern);
        }
        let skip = if token(&self.0, Eq).is_some() { 2 } else { 0 };
        tokens(&self.0)
            .nth(skip)
            .filter(|t| is_name(t.kind()) || t.kind() == StringLit)
            .map(LabelName::Static)
    }

    /// `?` or `!`, if present.
    pub fn marker(&self) -> Option<SyntaxToken> {
        tokens(&self.0).find(|t| matches!(t.kind(), Question | Bang))
    }
}

impl Alias {
    pub fn name(&self) -> Option<SyntaxToken> {
        token(&self.0, Ident)
    }

    pub fn expr(&self) -> Option<Expr> {
        child(&self.0)
    }
}

impl EllipsisExpr {
    /// `T` in `...T`.
    pub fn expr(&self) -> Option<Expr> {
        child(&self.0)
    }
}

/// A clause of a comprehension.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Clause {
    For(ForClause),
    If(IfClause),
    Let(LetClause),
}

impl Comprehension {
    pub fn clauses(&self) -> impl Iterator<Item = Clause> {
        self.0.children().filter_map(|n| match n.kind() {
            SyntaxKind::ForClause => Some(Clause::For(ForClause(n))),
            SyntaxKind::IfClause => Some(Clause::If(IfClause(n))),
            SyntaxKind::LetClause => Some(Clause::Let(LetClause(n))),
            _ => None,
        })
    }

    pub fn body(&self) -> Option<StructLit> {
        child(&self.0)
    }
}

impl ForClause {
    fn names(&self) -> Vec<SyntaxToken> {
        tokens(&self.0)
            .skip(1)
            .take_while(|t| t.kind() != InKw)
            .filter(|t| is_name(t.kind()))
            .collect()
    }

    /// `k` in `for k, v in x`.
    pub fn key(&self) -> Option<SyntaxToken> {
        let names = self.names();
        (names.len() == 2).then(|| names[0].clone())
    }

    /// `v` in `for k, v in x` or `for v in x`.
    pub fn value(&self) -> Option<SyntaxToken> {
        self.names().pop()
    }

    pub fn source(&self) -> Option<Expr> {
        child(&self.0)
    }
}

impl IfClause {
    pub fn condition(&self) -> Option<Expr> {
        child(&self.0)
    }
}

impl LetClause {
    pub fn name(&self) -> Option<SyntaxToken> {
        tokens(&self.0).nth(1).filter(|t| is_name(t.kind()))
    }

    pub fn value(&self) -> Option<Expr> {
        child(&self.0)
    }
}

impl StructLit {
    pub fn decls(&self) -> impl Iterator<Item = Decl> {
        decls(&self.0)
    }
}

/// An element of a list literal.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Element {
    Expr(Expr),
    Comprehension(Comprehension),
    Ellipsis(EllipsisExpr),
}

impl ListLit {
    pub fn elements(&self) -> impl Iterator<Item = Element> {
        self.0.children().filter_map(|n| match n.kind() {
            SyntaxKind::Comprehension => Some(Element::Comprehension(Comprehension(n))),
            SyntaxKind::EllipsisExpr => Some(Element::Ellipsis(EllipsisExpr(n))),
            _ => Expr::cast(n).map(Element::Expr),
        })
    }
}

impl Name {
    pub fn token(&self) -> Option<SyntaxToken> {
        tokens(&self.0).next()
    }
}

impl Literal {
    pub fn token(&self) -> Option<SyntaxToken> {
        tokens(&self.0).next()
    }
}

impl Interpolation {
    /// The literal pieces: `InterpStart`, any `InterpMid`s and `InterpEnd`.
    pub fn pieces(&self) -> impl Iterator<Item = SyntaxToken> {
        tokens(&self.0).filter(|t| matches!(t.kind(), InterpStart | InterpMid | InterpEnd))
    }

    /// The interpolated expressions, in order.
    pub fn exprs(&self) -> impl Iterator<Item = Expr> {
        children(&self.0)
    }
}

impl ParenExpr {
    pub fn expr(&self) -> Option<Expr> {
        child(&self.0)
    }
}

impl UnaryExpr {
    pub fn op(&self) -> Option<SyntaxToken> {
        tokens(&self.0).next()
    }

    pub fn operand(&self) -> Option<Expr> {
        child(&self.0)
    }
}

impl BinaryExpr {
    pub fn lhs(&self) -> Option<Expr> {
        child(&self.0)
    }

    pub fn op(&self) -> Option<SyntaxToken> {
        tokens(&self.0).next()
    }

    pub fn rhs(&self) -> Option<Expr> {
        children(&self.0).nth(1)
    }
}

impl SelectorExpr {
    pub fn operand(&self) -> Option<Expr> {
        child(&self.0)
    }

    /// The selected label: an identifier, keyword or string.
    pub fn name(&self) -> Option<SyntaxToken> {
        tokens(&self.0).skip_while(|t| t.kind() != Dot).nth(1)
    }
}

impl IndexExpr {
    pub fn operand(&self) -> Option<Expr> {
        child(&self.0)
    }

    pub fn index(&self) -> Option<Expr> {
        children(&self.0).nth(1)
    }
}

impl SliceExpr {
    pub fn operand(&self) -> Option<Expr> {
        child(&self.0)
    }

    /// The bounds `lo` and `hi` of `x[lo:hi]`; either may be absent.
    pub fn bounds(&self) -> (Option<Expr>, Option<Expr>) {
        let (mut lo, mut hi, mut after_colon) = (None, None, false);
        for element in self.0.children_with_tokens().skip(1) {
            match element {
                rowan::NodeOrToken::Token(t) if t.kind() == Colon => after_colon = true,
                rowan::NodeOrToken::Node(n) => {
                    if let Some(e) = Expr::cast(n) {
                        if after_colon {
                            hi = Some(e)
                        } else {
                            lo = Some(e)
                        }
                    }
                }
                _ => {}
            }
        }
        (lo, hi)
    }
}

impl CallExpr {
    pub fn callee(&self) -> Option<Expr> {
        child(&self.0)
    }

    pub fn args(&self) -> impl Iterator<Item = Expr> {
        self.0
            .children()
            .find(|n| n.kind() == ArgList)
            .into_iter()
            .flat_map(|args| children::<Expr>(&args).collect::<Vec<_>>())
    }
}
