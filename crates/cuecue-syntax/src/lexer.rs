//! Lossless lexer for CUE edition v1 (CUE spec, "Lexical elements"). Every byte of the
//! source belongs to exactly one token, trivia included.

use crate::SyntaxKind::{self, *};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    pub kind: SyntaxKind,
    pub len: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    /// Byte offset where the problem is.
    pub offset: u32,
    pub message: String,
}

#[derive(Debug, Default)]
pub struct Lexed {
    pub tokens: Vec<Token>,
    pub errors: Vec<LexError>,
}

pub fn lex(src: &str) -> Lexed {
    let mut lx = Lexer {
        src,
        pos: 0,
        prev: None,
        quotes: Vec::new(),
        out: Lexed::default(),
    };
    while lx.pos < src.len() {
        let start = lx.pos;
        let kind = lx.token();
        debug_assert!(lx.pos > start, "lexer made no progress");
        lx.out.tokens.push(Token {
            kind,
            len: (lx.pos - start) as u32,
        });
        if !kind.is_trivia() {
            lx.prev = Some(kind);
        }
    }
    if !lx.quotes.is_empty() {
        lx.error(src.len(), "string literal not terminated");
    }
    lx.out
}

/// An open string literal whose interpolation is being lexed.
#[derive(Clone, Copy)]
struct Quote {
    byte: u8,
    multiline: bool,
    hashes: usize,
    /// Unclosed `(` inside the current interpolation.
    depth: usize,
}

struct Lexer<'a> {
    src: &'a str,
    pos: usize,
    prev: Option<SyntaxKind>,
    quotes: Vec<Quote>,
    out: Lexed,
}

fn is_letter(c: char) -> bool {
    c.is_alphabetic() || c == '_' || c == '$'
}

fn is_ident_char(c: char) -> bool {
    is_letter(c) || c.is_numeric()
}

impl Lexer<'_> {
    fn rest(&self) -> &str {
        &self.src[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn peek_at(&self, n: usize) -> Option<char> {
        self.rest().chars().nth(n)
    }

    fn eat_while(&mut self, f: impl Fn(char) -> bool) -> usize {
        let start = self.pos;
        while let Some(c) = self.peek().filter(|&c| f(c)) {
            self.pos += c.len_utf8();
        }
        self.pos - start
    }

    fn error(&mut self, offset: usize, message: impl Into<String>) {
        self.out.errors.push(LexError {
            offset: offset as u32,
            message: message.into(),
        });
    }

    fn token(&mut self) -> SyntaxKind {
        let rest = self.rest();
        let c = self.peek().unwrap();
        match c {
            '\n' => {
                self.pos += 1;
                Newline
            }
            ' ' | '\t' | '\r' => {
                self.eat_while(|c| matches!(c, ' ' | '\t' | '\r'));
                Whitespace
            }
            '/' if rest.starts_with("//") => {
                self.eat_while(|c| c != '\n');
                Comment
            }
            '"' | '\'' => self.string(0),
            '#' => {
                let hashes = rest.bytes().take_while(|&b| b == b'#').count();
                match rest.as_bytes().get(hashes) {
                    Some(b'"' | b'\'') => self.string(hashes),
                    _ => self.ident(),
                }
            }
            '_' if rest.starts_with("_|_") => {
                self.pos += 3;
                Bottom
            }
            '@' => self.attribute(),
            '0'..='9' => self.number(),
            '.' if rest.starts_with("...") => {
                self.pos += 3;
                Ellipsis
            }
            '.' if self.float_allowed() && self.peek_at(1).is_some_and(|c| c.is_ascii_digit()) => {
                self.number()
            }
            ')' if self.quotes.last().is_some_and(|q| q.depth == 0) => {
                self.pos += 1;
                let quote = self.quotes.pop().unwrap();
                self.string_body(quote, true)
            }
            c if is_letter(c) => self.ident(),
            _ => self.punct(),
        }
    }

    fn float_allowed(&self) -> bool {
        self.prev.is_none_or(SyntaxKind::allows_float_after)
    }

    fn ident(&mut self) -> SyntaxKind {
        let start = self.pos;
        if self.rest().starts_with("_#") {
            self.pos += 2;
        } else if self.rest().starts_with('#') {
            self.pos += 1;
        }
        // A lone `#` or `_#` is an identifier too, as in CUE's own scanner (`#: "a": 1`).
        if self.peek().is_some_and(is_letter) {
            self.eat_while(is_ident_char);
        }
        match &self.src[start..self.pos] {
            "null" => NullKw,
            "true" => TrueKw,
            "false" => FalseKw,
            "package" => PackageKw,
            "import" => ImportKw,
            "for" => ForKw,
            "in" => InKw,
            "if" => IfKw,
            "let" => LetKw,
            _ => Ident,
        }
    }

    fn punct(&mut self) -> SyntaxKind {
        const TWO: [(&str, SyntaxKind); 8] = [
            ("&&", AmpAmp),
            ("||", PipePipe),
            ("==", EqEq),
            ("!=", BangEq),
            ("=~", EqTilde),
            ("!~", BangTilde),
            ("<=", LtEq),
            (">=", GtEq),
        ];
        if let Some(&(text, kind)) = TWO.iter().find(|(t, _)| self.rest().starts_with(t)) {
            self.pos += text.len();
            return kind;
        }
        let c = self.peek().unwrap();
        self.pos += c.len_utf8();
        let kind = match c {
            '+' => Plus,
            '-' => Minus,
            '*' => Star,
            '/' => Slash,
            '&' => Amp,
            '|' => Pipe,
            '<' => Lt,
            '>' => Gt,
            '=' => Eq,
            ':' => Colon,
            '?' => Question,
            '!' => Bang,
            '(' => LParen,
            ')' => RParen,
            '[' => LBracket,
            ']' => RBracket,
            '{' => LBrace,
            '}' => RBrace,
            ',' => Comma,
            '.' => Dot,
            _ => {
                self.error(
                    self.pos - c.len_utf8(),
                    format!("illegal character U+{:04X} '{c}'", c as u32),
                );
                Error
            }
        };
        if let Some(q) = self.quotes.last_mut() {
            match kind {
                LParen => q.depth += 1,
                RParen => q.depth -= 1,
                _ => {}
            }
        }
        kind
    }

    fn number(&mut self) -> SyntaxKind {
        let start = self.pos;
        let float_ok = self.float_allowed();
        let decimal = |c: char| c.is_ascii_digit() || c == '_';

        for (prefix, radix, name) in [
            ("0x", 16, "hexadecimal"),
            ("0X", 16, "hexadecimal"),
            ("0o", 8, "octal"),
            ("0b", 2, "binary"),
        ] {
            if self.rest().starts_with(prefix) {
                self.pos += 2;
                if self.eat_while(|c| c.is_digit(radix) || c == '_') == 0 {
                    self.error(start, format!("illegal {name} number"));
                }
                return Int;
            }
        }

        let mut float = false;
        if self.rest().starts_with('.') {
            // `.5`: only reached when a float may start here.
            self.pos += 1;
            self.eat_while(decimal);
            float = true;
        } else {
            self.eat_while(decimal);
            if !float_ok {
                return Int;
            }
            if self.peek() == Some('.') && self.peek_at(1) != Some('.') {
                self.pos += 1;
                self.eat_while(decimal);
                float = true;
            }
        }
        if let Some('K' | 'M' | 'G' | 'T' | 'P') = self.peek() {
            self.pos += 1;
            if self.peek() == Some('i') {
                self.pos += 1;
            }
            return Int;
        }
        if let Some('e' | 'E') = self.peek() {
            self.pos += 1;
            if let Some('+' | '-') = self.peek() {
                self.pos += 1;
            }
            if self.eat_while(decimal) == 0 {
                self.error(start, "exponent has no digits");
            }
            float = true;
        }
        if float { Float } else { Int }
    }

    fn string(&mut self, hashes: usize) -> SyntaxKind {
        let start = self.pos;
        self.pos += hashes;
        let byte = self.src.as_bytes()[self.pos];
        let triple = if byte == b'"' { "\"\"\"" } else { "'''" };
        let multiline = self.rest().starts_with(triple);
        if multiline {
            self.pos += 3;
            if !(self.rest().starts_with('\n') || self.rest().starts_with("\r\n")) {
                self.error(start, "expected newline after multiline quote");
            }
        } else {
            self.pos += 1;
        }
        let quote = Quote {
            byte,
            multiline,
            hashes,
            depth: 0,
        };
        self.string_body(quote, false)
    }

    /// Lexes string content up to the closing quote or the next interpolation.
    fn string_body(&mut self, quote: Quote, continued: bool) -> SyntaxKind {
        let closer: String =
            std::iter::repeat_n(quote.byte as char, if quote.multiline { 3 } else { 1 })
                .chain(std::iter::repeat_n('#', quote.hashes))
                .collect();
        let escape: String = std::iter::once('\\')
            .chain(std::iter::repeat_n('#', quote.hashes))
            .collect();
        let done = if continued { InterpEnd } else { StringLit };
        loop {
            let rest = self.rest();
            if rest.is_empty() || (rest.starts_with('\n') && !quote.multiline) {
                self.error(self.pos, "string literal not terminated");
                return done;
            }
            if rest.starts_with(&closer) {
                self.pos += closer.len();
                return done;
            }
            if rest.starts_with(&escape) {
                self.pos += escape.len();
                if self.peek() == Some('(') {
                    self.pos += 1;
                    self.quotes.push(quote);
                    return if continued { InterpMid } else { InterpStart };
                }
                self.escape(quote);
                continue;
            }
            self.pos += self.peek().unwrap().len_utf8();
        }
    }

    /// Checks one escape sequence; the escape delimiter is already consumed.
    fn escape(&mut self, quote: Quote) {
        let start = self.pos;
        let bytes = quote.byte == b'\'';
        let Some(c) = self.peek() else { return };
        self.pos += c.len_utf8();
        let hex = |this: &mut Self, n: usize, radix: u32| -> Option<u32> {
            let digits = this
                .rest()
                .get(..n)
                .filter(|d| d.chars().all(|c| c.is_digit(radix)));
            let value = digits.and_then(|d| u32::from_str_radix(d, radix).ok());
            if value.is_some() {
                this.pos += n;
            }
            value
        };
        let ok = match c {
            'a' | 'b' | 'f' | 'n' | 'r' | 't' | 'v' | '/' | '\\' | '\n' => true,
            '\'' => bytes,
            '"' => !bytes,
            '\r' if self.peek() == Some('\n') => {
                self.pos += 1;
                true
            }
            'x' if bytes => hex(self, 2, 16).is_some(),
            'u' => hex(self, 4, 16).is_some(),
            'U' => hex(self, 8, 16).is_some_and(|v| v <= 0x10FFFF),
            '0'..='7' if bytes => {
                self.pos -= 1;
                hex(self, 3, 8).is_some_and(|v| v <= 255)
            }
            _ => false,
        };
        if !ok {
            self.error(start, "unknown escape sequence");
        }
    }

    fn attribute(&mut self) -> SyntaxKind {
        let start = self.pos;
        self.pos += 1;
        if self.eat_while(is_ident_char) == 0 || self.peek() != Some('(') {
            self.error(start, "attribute must be `@name(…)`");
            return Attribute;
        }
        // The body is a balanced token sequence, so lex it with the regular rules (strings,
        // multiline strings and comments included) and count brackets.
        let mut depth = 0usize;
        while self.pos < self.src.len() {
            match self.token() {
                LParen | LBracket | LBrace => depth += 1,
                RParen | RBracket | RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        return Attribute;
                    }
                }
                _ => {}
            }
        }
        self.error(start, "attribute not terminated");
        Attribute
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lexes `src` and renders non-trivia tokens as `Kind(text)`, asserting losslessness.
    fn tokens(src: &str) -> Vec<String> {
        let lexed = lex(src);
        let mut pos = 0;
        let mut out = Vec::new();
        for t in &lexed.tokens {
            let text = &src[pos..pos + t.len as usize];
            pos += t.len as usize;
            if !t.kind.is_trivia() {
                out.push(format!("{:?}({text})", t.kind));
            }
        }
        assert_eq!(pos, src.len(), "tokens must cover the source");
        out
    }

    fn errors(src: &str) -> Vec<String> {
        lex(src).errors.into_iter().map(|e| e.message).collect()
    }

    #[test]
    fn identifiers_and_keywords() {
        assert_eq!(
            tokens("a _x9 fieldName αβ #Def _#hidden _ __x null if"),
            [
                "Ident(a)",
                "Ident(_x9)",
                "Ident(fieldName)",
                "Ident(αβ)",
                "Ident(#Def)",
                "Ident(_#hidden)",
                "Ident(_)",
                "Ident(__x)",
                "NullKw(null)",
                "IfKw(if)"
            ]
        );
    }

    #[test]
    fn operators() {
        assert_eq!(
            tokens("+ && == < = ( ) - || != > : { } * & =~ <= ? [ ] , / | !~ >= ! _|_ ... ."),
            [
                "Plus(+)",
                "AmpAmp(&&)",
                "EqEq(==)",
                "Lt(<)",
                "Eq(=)",
                "LParen(()",
                "RParen())",
                "Minus(-)",
                "PipePipe(||)",
                "BangEq(!=)",
                "Gt(>)",
                "Colon(:)",
                "LBrace({)",
                "RBrace(})",
                "Star(*)",
                "Amp(&)",
                "EqTilde(=~)",
                "LtEq(<=)",
                "Question(?)",
                "LBracket([)",
                "RBracket(])",
                "Comma(,)",
                "Slash(/)",
                "Pipe(|)",
                "BangTilde(!~)",
                "GtEq(>=)",
                "Bang(!)",
                "Bottom(_|_)",
                "Ellipsis(...)",
                "Dot(.)"
            ]
        );
    }

    #[test]
    fn numbers_from_the_spec() {
        for src in [
            "42",
            "1.5G",
            "1.3Ki",
            "170_141_183_460_469_231_731_687_303_715_884_105_727",
            "0xBad_Face",
            "0o755",
            "0b0101_0001",
        ] {
            assert_eq!(tokens(src), [format!("Int({src})")], "{src}");
        }
        for src in [
            "0.",
            "72.40",
            "072.40",
            "2.71828",
            "1.e+0",
            "6.67428e-11",
            "1E6",
            ".25",
            ".12345E+5",
        ] {
            assert_eq!(tokens(src), [format!("Float({src})")], "{src}");
        }
        assert_eq!(errors("0x"), ["illegal hexadecimal number"]);
    }

    #[test]
    fn floats_do_not_follow_values() {
        assert_eq!(tokens("a + 3.2Ti"), ["Ident(a)", "Plus(+)", "Int(3.2Ti)"]);
        assert_eq!(
            tokens("a 3.2Ti"),
            ["Ident(a)", "Int(3)", "Dot(.)", "Int(2)", "Ident(Ti)"]
        );
        assert_eq!(tokens("a + .5e3"), ["Ident(a)", "Plus(+)", "Float(.5e3)"]);
        assert_eq!(
            tokens("a .5e3"),
            ["Ident(a)", "Dot(.)", "Int(5)", "Ident(e3)"]
        );
    }

    #[test]
    fn strings_from_the_spec() {
        for src in [
            r"'a\000\xab'",
            r"'\007'",
            r"'\377'",
            r#""\n""#,
            r#""\"""#,
            r"'Hello, world!\n'",
            r#""日本語""#,
            r#""日本\U00008a9e""#,
            r"'\xffÿ'",
            r##"#"This is not an \(interpolation)"#"##,
            r##"#"The sequence "\U0001F604" renders as \#U0001F604."#"##,
            r"'\xe6\x97\xa5\xe6\x9c\xac\xe8\xaa\x9e'",
        ] {
            assert_eq!(tokens(src), [format!("StringLit({src})")], "{src}");
            assert_eq!(errors(src), Vec::<String>::new(), "{src}");
        }
        assert_eq!(errors(r"'\xa'"), ["unknown escape sequence"]);
        assert_eq!(errors(r#""\U00110000""#), ["unknown escape sequence"]);
        assert_eq!(errors(r#""\x41""#), ["unknown escape sequence"]);
        assert_eq!(
            errors("\"abc\nd\""),
            [
                "string literal not terminated",
                "string literal not terminated"
            ]
        );
    }

    #[test]
    fn multiline_strings() {
        let src = "\"\"\"\n    lily:\n    picking \\\n    bugs\n    \"\"\"";
        assert_eq!(tokens(src), [format!("StringLit({src})")]);
        assert!(errors(src).is_empty());
    }

    #[test]
    fn interpolation() {
        assert_eq!(
            tokens(r#""Hello, \( name )!""#),
            [
                r#"InterpStart("Hello, \()"#,
                "Ident(name)",
                r#"InterpEnd()!")"#
            ]
        );
        assert_eq!(
            tokens(r#""a\(f(x))b\("c\(1)")d""#),
            [
                r#"InterpStart("a\()"#,
                "Ident(f)",
                "LParen(()",
                "Ident(x)",
                "RParen())",
                r#"InterpMid()b\()"#,
                r#"InterpStart("c\()"#,
                "Int(1)",
                r#"InterpEnd()")"#,
                r#"InterpEnd()d")"#
            ]
        );
        assert_eq!(
            tokens(r##"#"This is an \#(x)"#"##),
            [
                r##"InterpStart(#"This is an \#()"##,
                "Ident(x)",
                r##"InterpEnd()"#)"##
            ]
        );
    }

    #[test]
    fn comments_attributes_and_newlines() {
        let lexed = lex("a: 1 @go(A,\"x)\") // doc\n");
        let kinds: Vec<_> = lexed.tokens.iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            [
                Ident, Colon, Whitespace, Int, Whitespace, Attribute, Whitespace, Comment, Newline
            ]
        );
        assert!(lexed.errors.is_empty());
        assert!(Attribute.ends_line_with_comma() && RBrace.ends_line_with_comma());
        assert!(!Colon.ends_line_with_comma() && !Plus.ends_line_with_comma());
    }

    #[test]
    fn illegal_characters() {
        assert_eq!(
            errors("a: `x`"),
            [
                "illegal character U+0060 '`'",
                "illegal character U+0060 '`'"
            ]
        );
        assert_eq!(tokens("#: _#"), ["Ident(#)", "Colon(:)", "Ident(_#)"]);
    }

    #[test]
    fn attributes_hold_any_balanced_tokens() {
        let src = "@test(debugCheck, \"\"\"\n\t(int){ 42 ) }\n\t\"\"\", 'it\\'s')";
        assert_eq!(tokens(src), [format!("Attribute({src})")]);
        assert!(errors(src).is_empty());
        assert_eq!(errors("@go(a"), ["attribute not terminated"]);
    }
}
