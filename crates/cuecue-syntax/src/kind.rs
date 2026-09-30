/// Token kinds (M1.1). Node kinds join them when the parser arrives (M1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u16)]
pub enum SyntaxKind {
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
