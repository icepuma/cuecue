macro_rules! syntax_kinds {
    ($($(#[$meta:meta])* $name:ident,)*) => {
        /// Token and node kinds of the syntax tree.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[repr(u16)]
        pub enum SyntaxKind {
            $($(#[$meta])* $name,)*
        }

        impl SyntaxKind {
            const ALL: &'static [SyntaxKind] = &[$(SyntaxKind::$name,)*];

            pub fn from_raw(raw: u16) -> Option<SyntaxKind> {
                Self::ALL.get(raw as usize).copied()
            }
        }
    };
}

syntax_kinds! {
    // Trivia
    Whitespace,
    Newline,
    Comment,

    // Literals and names
    Ident,
    Int,
    Float,
    /// A string or bytes literal without interpolation.
    StringLit,
    /// `"text\(`: the start of an interpolated literal.
    InterpStart,
    /// `)text\(`: between two interpolations.
    InterpMid,
    /// `)text"`: the end of an interpolated literal.
    InterpEnd,
    /// `@name(…)`
    Attribute,
    /// `_|_`
    Bottom,

    // Keywords
    NullKw,
    TrueKw,
    FalseKw,
    PackageKw,
    ImportKw,
    ForKw,
    InKw,
    IfKw,
    LetKw,

    // Operators and punctuation
    Plus,
    Minus,
    Star,
    Slash,
    AmpAmp,
    PipePipe,
    Amp,
    Pipe,
    EqEq,
    BangEq,
    EqTilde,
    BangTilde,
    Lt,
    Gt,
    LtEq,
    GtEq,
    Eq,
    Colon,
    Question,
    Bang,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Comma,
    Dot,
    Ellipsis,

    /// A character sequence that forms no token; the lexer reports why.
    Error,

    // Nodes
    SourceFile,
    PackageClause,
    ImportDecl,
    ImportSpec,
    /// `label: value`; a shorthand `a: b: c` nests fields.
    Field,
    /// `X=name?`, `(expr)` or `[pattern]`, with an optional `?` or `!`.
    Label,
    /// `X=expr`
    Alias,
    /// `...` or `...T`
    EllipsisExpr,
    Comprehension,
    ForClause,
    IfClause,
    LetClause,
    StructLit,
    ListLit,
    /// A reference to a field, alias, let or package.
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
    ArgList,
    /// Tokens the parser couldn't place.
    ErrorNode,
}

impl SyntaxKind {
    pub fn is_trivia(self) -> bool {
        matches!(self, Self::Whitespace | Self::Newline | Self::Comment)
    }

    pub fn is_keyword(self) -> bool {
        use SyntaxKind::*;
        matches!(
            self,
            NullKw | TrueKw | FalseKw | PackageKw | ImportKw | ForKw | InKw | IfKw | LetKw
        )
    }

    /// Whether a newline after this token ends a declaration or element, as if a comma
    /// followed it (CUE spec, "Commas").
    pub fn ends_line_with_comma(self) -> bool {
        use SyntaxKind::*;
        self.is_keyword()
            || matches!(
                self,
                Ident
                    | Int
                    | Float
                    | StringLit
                    | InterpEnd
                    | Attribute
                    | Bottom
                    | RParen
                    | RBracket
                    | RBrace
                    | Question
                    | Ellipsis
            )
    }

    /// Whether a float or SI literal may start after this token (CUE spec, "Numeric literals").
    pub(crate) fn allows_float_after(self) -> bool {
        use SyntaxKind::*;
        !(self.is_keyword()
            || matches!(
                self,
                Ident
                    | Int
                    | Float
                    | StringLit
                    | InterpEnd
                    | Bottom
                    | RParen
                    | RBracket
                    | RBrace
                    | Question
                    | Dot
            ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CueLanguage {}

impl rowan::Language for CueLanguage {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> SyntaxKind {
        SyntaxKind::from_raw(raw.0).expect("syntax kind out of range")
    }

    fn kind_to_raw(kind: SyntaxKind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(kind as u16)
    }
}

pub type SyntaxNode = rowan::SyntaxNode<CueLanguage>;
pub type SyntaxToken = rowan::SyntaxToken<CueLanguage>;
