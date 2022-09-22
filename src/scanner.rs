use std::str::Chars;

use strum_macros::{EnumCount, EnumIter};

#[derive(Clone, Copy, Debug, EnumCount, EnumIter, Hash, PartialEq, Eq)]
pub(crate) enum TokenKind {
    // Single-char tokens
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,

    // One or two char tokens
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,

    // Literals
    Identifier,
    String,
    Number,

    // Keywords
    And,
    Class,
    Else,
    False,
    For,
    Fun,
    If,
    Nil,
    Or,
    Print,
    Return,
    Super,
    This,
    True,
    Var,
    While,

    Comment,
    Eof,
    Dummy,
}

impl Default for TokenKind {
    fn default() -> Self {
        TokenKind::Dummy
    }
}

impl TokenKind {
    /// Returns `true` if the token kind is [`Eof`].
    ///
    /// [`Eof`]: TokenKind::Eof
    #[must_use]
    pub fn is_eof(&self) -> bool {
        matches!(self, Self::Eof)
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Token<'source> {
    pub kind: TokenKind,
    pub line: usize,
    pub start: usize,
    lexeme: Option<&'source str>,
}

impl<'source> Token<'source> {
    pub fn new(kind: TokenKind, line: usize, start: usize, lexeme: Option<&'source str>) -> Self {
        Self {
            kind,
            line,
            start,
            lexeme,
        }
    }

    pub fn dummy() -> Self {
        Self {
            kind: TokenKind::Dummy,
            ..Default::default()
        }
    }

    pub fn is_eof(&self) -> bool {
        self.kind.is_eof()
    }

    pub fn kind(&self) -> &TokenKind {
        &self.kind
    }

    pub fn lexeme(&self) -> Option<&str> {
        self.lexeme
    }
}

#[derive(Debug)]
pub(crate) struct Scanner<'source> {
    chars: Chars<'source>,
    source: &'source str,
    current: usize,
    start: usize,
    line: usize,
}

impl<'source> Scanner<'source> {
    pub fn new(source: &'source str) -> Self {
        let chars = source.chars();

        Self {
            chars,
            source,
            current: 0,
            start: 0,
            line: 1,
        }
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn is_at_end(&mut self) -> bool {
        self.chars.clone().peekable().peek().is_none()
    }

    pub fn advance(&mut self) -> Option<char> {
        self.next()
    }

    pub fn peek(&mut self) -> Option<char> {
        let mut peekable = self.chars.clone();

        peekable.next()
    }

    pub fn peek_next(&mut self) -> Option<char> {
        let mut peekable = self.chars.clone();

        peekable.next();
        peekable.next()
    }

    pub fn matches(&mut self, c: char) -> bool {
        if self.is_at_end() {
            return false;
        }

        if let Some(ch) = self.peek() {
            if ch == c {
                let _ = self.advance();
                return true;
            }
        }

        false
    }

    pub fn skip_whitespace(&mut self) {
        loop {
            let c = match self.peek() {
                Some(c) => c,
                None => return,
            };

            match c {
                ' ' | '\r' | '\t' => {
                    self.advance();
                }
                '\n' => {
                    self.line += 1;
                    self.advance();
                }
                _ => return,
            }
        }
    }

    pub fn comment(&mut self) -> Token<'source> {
        while let Some(c) = self.peek() {
            if c == '\n' {
                break;
            }

            self.advance();
        }

        self.make_token(TokenKind::Comment)
    }

    pub fn scan_token(&mut self) -> Result<Token<'source>, ScannerError> {
        self.skip_whitespace();

        self.start = self.current;

        if self.is_at_end() {
            return Ok(Token::new(TokenKind::Eof, self.line, self.start, None));
        }

        let c = self.advance().unwrap();

        if self.is_alpha(c) {
            return Ok(self.identifier());
        }

        if c.is_ascii_digit() {
            return Ok(self.number());
        }

        let result = match c {
            '(' => self.make_token(TokenKind::LeftParen),
            ')' => self.make_token(TokenKind::RightParen),
            '{' => self.make_token(TokenKind::LeftBrace),
            '}' => self.make_token(TokenKind::RightBrace),
            ';' => self.make_token(TokenKind::Semicolon),
            ',' => self.make_token(TokenKind::Comma),
            '.' => self.make_token(TokenKind::Dot),
            '-' => self.make_token(TokenKind::Minus),
            '+' => self.make_token(TokenKind::Plus),
            '/' => {
                if self.matches('/') {
                    self.comment()
                } else {
                    self.make_token(TokenKind::Slash)
                }
            }
            '*' => self.make_token(TokenKind::Star),
            '!' => {
                if self.matches('=') {
                    self.make_token(TokenKind::BangEqual)
                } else {
                    self.make_token(TokenKind::Bang)
                }
            }
            '=' => {
                if self.matches('=') {
                    self.make_token(TokenKind::EqualEqual)
                } else {
                    self.make_token(TokenKind::Equal)
                }
            }
            '<' => {
                if self.matches('=') {
                    self.make_token(TokenKind::LessEqual)
                } else {
                    self.make_token(TokenKind::Less)
                }
            }
            '>' => {
                if self.matches('=') {
                    self.make_token(TokenKind::GreaterEqual)
                } else {
                    self.make_token(TokenKind::Greater)
                }
            }
            '"' => self.string()?,
            _ => unreachable!(),
        };

        Ok(result)
    }

    pub fn make_token(&mut self, kind: TokenKind) -> Token<'source> {
        let lexeme = &self.source[self.start..self.current];

        Token::new(kind, self.line, self.start, Some(lexeme))
    }

    fn string(&mut self) -> Result<Token<'source>, ScannerError> {
        while let Some(c) = self.peek() {
            if c == '\"' {
                break;
            }
            if c == '\n' {
                self.line += 1;
            }

            self.advance();
        }

        if self.advance().is_none() {
            return Err(ScannerError {
                msg: "unterminated string literal".into(),
                line: self.line,
            });
        }

        Ok(self.make_token(TokenKind::String))
    }

    fn number(&mut self) -> Token<'source> {
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }

        if let Some(c) = self.peek() {
            if let Some(after_dot) = self.peek_next() {
                if c == '.' && after_dot.is_ascii_digit() {
                    self.advance();

                    while let Some(c) = self.peek() {
                        if c.is_ascii_digit() {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        self.make_token(TokenKind::Number)
    }

    fn identifier(&mut self) -> Token<'source> {
        while let Some(c) = self.peek() {
            if self.is_alpha(c) || c.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }

        self.make_token(self.identifier_kind())
    }

    fn identifier_kind(&self) -> TokenKind {
        match &self.source[self.start..self.start + 1] {
            "a" => self.check_keyword(1, 2, "nd", TokenKind::And),
            "c" => self.check_keyword(1, 4, "lass", TokenKind::Class),
            "e" => self.check_keyword(1, 3, "lse", TokenKind::Else),
            "f" => {
                if self.current - self.start > 1 {
                    match &self.source[self.start + 1..self.start + 2] {
                        "a" => self.check_keyword(2, 3, "lse", TokenKind::False),
                        "o" => self.check_keyword(2, 1, "r", TokenKind::For),
                        "u" => self.check_keyword(2, 1, "n", TokenKind::Fun),
                        _ => TokenKind::Identifier,
                    }
                } else {
                    TokenKind::Identifier
                }
            }
            "i" => self.check_keyword(1, 1, "f", TokenKind::If),
            "n" => self.check_keyword(1, 2, "il", TokenKind::Nil),
            "o" => self.check_keyword(1, 1, "r", TokenKind::Or),
            "p" => self.check_keyword(1, 4, "rint", TokenKind::Print),
            "r" => self.check_keyword(1, 5, "eturn", TokenKind::Return),
            "s" => self.check_keyword(1, 4, "uper", TokenKind::Super),
            "t" => {
                if self.current - self.start > 1 {
                    match &self.source[self.start + 1..self.start + 2] {
                        "h" => self.check_keyword(2, 2, "is", TokenKind::This),
                        "r" => self.check_keyword(2, 2, "ue", TokenKind::True),
                        _ => TokenKind::Identifier,
                    }
                } else {
                    TokenKind::Identifier
                }
            }
            "v" => self.check_keyword(1, 2, "ar", TokenKind::Var),
            "w" => self.check_keyword(1, 4, "hile", TokenKind::While),
            _ => TokenKind::Identifier,
        }
    }

    fn is_alpha(&self, c: char) -> bool {
        c.is_alphabetic() || c == '_'
    }

    fn check_keyword(
        &self,
        start: usize,
        len: usize,
        rest: &str,
        token_kind: TokenKind,
    ) -> TokenKind {
        if self.current - self.start == start + len
            && &self.source[(self.start + start)..self.current] == rest
        {
            return token_kind;
        }

        TokenKind::Identifier
    }

    pub fn next(&mut self) -> Option<char> {
        match self.chars.next() {
            Some(ch) => {
                self.current += 1;
                Some(ch)
            }
            None => None,
        }
    }
}

impl<'source> Iterator for Scanner<'source> {
    type Item = Token<'source>;

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.scan_token().unwrap();
        match token.kind() {
            TokenKind::Eof => None,
            _ => Some(token),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ScannerError {
    msg: String,
    line: usize,
}

impl ScannerError {
    pub fn msg(&self) -> &str {
        &self.msg
    }

    pub fn line(&self) -> usize {
        self.line
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = "print \"This is a test\"\nvar a = 1;";

    #[test]
    fn is_at_end_works() {
        let mut scanner = Scanner::new(SOURCE);

        assert_ne!(true, scanner.is_at_end());
        assert_ne!(None, scanner.peek());

        for _ in 0..SOURCE.len() {
            scanner.next();
        }

        assert_eq!(true, scanner.is_at_end());
        assert_eq!(None, scanner.peek());
    }

    #[test]
    fn is_at_end_works_on_empty() {
        let mut scanner = Scanner::new("");

        assert_eq!(true, scanner.is_at_end());
    }

    #[test]
    fn next_works() {
        let mut scanner = Scanner::new(SOURCE);

        assert_eq!(false, scanner.is_at_end());

        for c in SOURCE.chars() {
            assert_eq!(c, scanner.next().unwrap());
        }

        assert_eq!(true, scanner.is_at_end());
    }

    #[test]
    fn next_works_on_empty() {
        let mut scanner = Scanner::new("");

        assert_eq!(None, scanner.next());
        assert_eq!(true, scanner.is_at_end());
    }

    #[test]
    fn advance_works() {
        let mut scanner = Scanner::new(SOURCE);

        assert_eq!(false, scanner.is_at_end());

        for c in SOURCE.chars() {
            assert_eq!(c, scanner.advance().unwrap());
        }

        assert_eq!(true, scanner.is_at_end());
    }

    #[test]
    fn advance_works_on_empty() {
        let mut scanner = Scanner::new("");

        assert_eq!(None, scanner.next());
        assert_eq!(true, scanner.is_at_end());
    }

    #[test]
    fn peek_works() {
        let mut scanner = Scanner::new(SOURCE);

        scanner.next();
        assert_eq!(Some('r'), scanner.peek());
        assert_eq!(Some('r'), scanner.next());
    }

    #[test]
    fn peek_works_at_beginning() {
        let mut scanner = Scanner::new(SOURCE);

        assert_eq!(false, scanner.is_at_end());
        assert_eq!(Some('p'), scanner.peek());
    }

    #[test]
    fn peek_works_on_empty() {
        let mut scanner = Scanner::new("");

        assert_eq!(None, scanner.peek());
    }

    #[test]
    fn peek_next_works() {
        let mut scanner = Scanner::new(SOURCE);

        scanner.next();
        assert_eq!(Some('i'), scanner.peek_next());
        assert_eq!(Some('r'), scanner.next());
    }

    #[test]
    fn peek_next_works_at_beginning() {
        let mut scanner = Scanner::new(SOURCE);

        assert_eq!(false, scanner.is_at_end());
        assert_eq!(Some('r'), scanner.peek_next());
    }

    #[test]
    fn peek_next_works_on_empty() {
        let mut scanner = Scanner::new("");

        assert_eq!(None, scanner.peek_next());
    }

    #[test]
    fn matches_works() {
        let mut scanner = Scanner::new(SOURCE);

        scanner.next();
        // matches and advances
        assert_eq!(true, scanner.matches('r'));
        assert_eq!(Some('i'), scanner.peek());
        // does not match and does not advance
        assert_eq!(false, scanner.matches('z'));
        assert_eq!(Some('i'), scanner.next());
    }

    #[test]
    fn matches_works_on_empty() {
        let mut scanner = Scanner::new("");

        assert_eq!(false, scanner.matches('p'));
    }

    #[test]
    fn skip_whitespace_works_only_spaces() {
        let mut scanner = Scanner::new(" \t \t      \n  \t  \n   \t   ");

        scanner.scan_token();

        assert_eq!(true, scanner.is_at_end());
        assert_eq!(3, scanner.line());
    }

    #[test]
    fn skip_whitespace_with_comments_only() {
        let mut scanner = Scanner::new("//this should all be ignored");

        scanner.scan_token();

        assert_eq!(true, scanner.is_at_end());
        assert_eq!(1, scanner.line());
    }

    #[test]
    fn scan_token_comment_and_newlines() {
        let mut scanner = Scanner::new("//this should all be ignored\n//so should this\n\n\n");

        let token = scanner.scan_token().unwrap();
        assert_eq!("//this should all be ignored", token.lexeme().unwrap());
        assert_eq!(false, scanner.is_at_end());
        assert_eq!(1, scanner.line());
        let token = scanner.scan_token().unwrap();
        assert_eq!("//so should this", token.lexeme().unwrap());
        assert_eq!(false, scanner.is_at_end());
        assert_eq!(2, scanner.line());
        // still have whitespeces to consume
        scanner.scan_token();
        assert_eq!(true, scanner.is_at_end());
    }

    #[test]
    fn skip_whitespace_works_spaces_and_valid_chars() {
        let mut scanner = Scanner::new(" \t \t      1\n  \t  \n   \t   ");

        scanner.scan_token();

        assert_eq!(false, scanner.is_at_end());
        assert_eq!(1, scanner.line());

        scanner.scan_token();

        assert_eq!(true, scanner.is_at_end());
        // not sure
        assert_eq!(3, scanner.line());
    }

    #[test]
    fn scan_token_skip_whitespace_works_on_empty() {
        let mut scanner = Scanner::new("");

        scanner.scan_token();

        assert_eq!(true, scanner.is_at_end());
    }

    #[test]
    fn scan_token_left_paren() {
        let mut scanner = Scanner::new("(");

        assert_eq!(TokenKind::LeftParen, *scanner.scan_token().unwrap().kind());
    }

    #[test]
    fn scan_token_right_paren() {
        let mut scanner = Scanner::new(")");

        assert_eq!(TokenKind::RightParen, *scanner.scan_token().unwrap().kind());
    }

    #[test]
    fn scan_token_left_brace() {
        let mut scanner = Scanner::new("{");

        assert_eq!(TokenKind::LeftBrace, *scanner.scan_token().unwrap().kind());
    }

    #[test]
    fn scan_token_right_brace() {
        let mut scanner = Scanner::new("}");

        assert_eq!(TokenKind::RightBrace, *scanner.scan_token().unwrap().kind());
    }

    #[test]
    fn scan_token_semicolon() {
        let mut scanner = Scanner::new(";");

        assert_eq!(TokenKind::Semicolon, *scanner.scan_token().unwrap().kind());
    }

    #[test]
    fn scan_token_comma() {
        let mut scanner = Scanner::new(",");

        assert_eq!(TokenKind::Comma, *scanner.scan_token().unwrap().kind());
    }

    #[test]
    fn scan_token_dot() {
        let mut scanner = Scanner::new(".");

        assert_eq!(TokenKind::Dot, *scanner.scan_token().unwrap().kind());
    }

    #[test]
    fn scan_token_minus() {
        let mut scanner = Scanner::new("-");

        assert_eq!(TokenKind::Minus, *scanner.scan_token().unwrap().kind());
    }

    #[test]
    fn scan_token_plus() {
        let mut scanner = Scanner::new("+");

        assert_eq!(TokenKind::Plus, *scanner.scan_token().unwrap().kind());
    }

    #[test]
    fn scan_token_slash() {
        let mut scanner = Scanner::new("/");

        assert_eq!(TokenKind::Slash, *scanner.scan_token().unwrap().kind());
    }

    #[test]
    fn scan_token_star() {
        let mut scanner = Scanner::new("*");

        assert_eq!(TokenKind::Star, *scanner.scan_token().unwrap().kind());
    }

    #[test]
    fn scan_token_bang() {
        let mut scanner = Scanner::new("!");

        assert_eq!(TokenKind::Bang, *scanner.scan_token().unwrap().kind());
    }

    #[test]
    fn scan_token_bang_equal() {
        let mut scanner = Scanner::new("!=");

        assert_eq!(TokenKind::BangEqual, *scanner.scan_token().unwrap().kind());
    }

    #[test]
    fn scan_token_equal() {
        let mut scanner = Scanner::new("=");

        assert_eq!(TokenKind::Equal, *scanner.scan_token().unwrap().kind());
    }

    #[test]
    fn scan_token_equal_equal() {
        let mut scanner = Scanner::new("==");

        assert_eq!(TokenKind::EqualEqual, *scanner.scan_token().unwrap().kind());
    }

    #[test]
    fn scan_token_greater() {
        let mut scanner = Scanner::new(">");

        assert_eq!(TokenKind::Greater, *scanner.scan_token().unwrap().kind());
    }

    #[test]
    fn scan_token_greater_equal() {
        let mut scanner = Scanner::new(">=");

        assert_eq!(
            TokenKind::GreaterEqual,
            *scanner.scan_token().unwrap().kind()
        );
    }

    #[test]
    fn scan_token_less() {
        let mut scanner = Scanner::new("<");

        assert_eq!(TokenKind::Less, *scanner.scan_token().unwrap().kind());
    }

    #[test]
    fn scan_token_less_equal() {
        let mut scanner = Scanner::new("<=");

        assert_eq!(TokenKind::LessEqual, *scanner.scan_token().unwrap().kind());
    }

    #[test]
    fn scan_token_string() {
        let mut scanner = Scanner::new("\"this is a test string\"");

        let token = scanner.scan_token().unwrap();
        assert_eq!(TokenKind::String, *token.kind());
        assert_eq!("\"this is a test string\"", token.lexeme().unwrap());
        assert_eq!(1, scanner.line());
    }

    #[test]
    fn scan_token_string_multiline() {
        let mut scanner = Scanner::new("\"this is a test string\n and this is the second line\"");

        let token = scanner.scan_token().unwrap();
        assert_eq!(TokenKind::String, *token.kind());
        assert_eq!(
            "\"this is a test string\n and this is the second line\"",
            token.lexeme().unwrap()
        );
        assert_eq!(2, scanner.line());
    }

    #[test]
    #[should_panic(expected = "unterminated string literal")]
    fn scan_token_string_unterminated() {
        let mut scanner = Scanner::new("\"this is a test string");

        let token = scanner.scan_token().unwrap();
        assert_eq!(
            "\"this is an unterminated test string",
            token.lexeme().unwrap()
        );
        assert_eq!(1, scanner.line());
    }

    #[test]
    fn scan_token_number_integer() {
        let mut scanner = Scanner::new("42");

        let token = scanner.scan_token().unwrap();
        assert_eq!(TokenKind::Number, *token.kind());
        assert_eq!("42", token.lexeme().unwrap());
        assert_eq!(1, scanner.line());
    }

    #[test]
    fn scan_token_number_float() {
        let mut scanner = Scanner::new("7.65");

        let token = scanner.scan_token().unwrap();
        assert_eq!(TokenKind::Number, *token.kind());
        assert_eq!("7.65", token.lexeme().unwrap());
        assert_eq!(1, scanner.line());
    }

    #[test]
    fn scan_token_id() {
        let mut scanner = Scanner::new("valid_name");

        let token = scanner.scan_token().unwrap();
        assert_eq!(TokenKind::Identifier, *token.kind());
        assert_eq!("valid_name", token.lexeme().unwrap());
        assert_eq!(1, scanner.line());
    }

    #[test]
    fn scan_token_id_underscore() {
        let mut scanner = Scanner::new("_also_valid_id");

        let token = scanner.scan_token().unwrap();
        assert_eq!(TokenKind::Identifier, *token.kind());
        assert_eq!("_also_valid_id", token.lexeme().unwrap());
        assert_eq!(1, scanner.line());
    }

    #[test]
    fn scan_token_id_kw_and() {
        let mut scanner = Scanner::new("and");

        let token = scanner.scan_token().unwrap();
        assert_eq!(TokenKind::And, *token.kind());
        assert_eq!("and", token.lexeme().unwrap());
        assert_eq!(1, scanner.line());
    }

    #[test]
    fn scan_token_id_kw_class() {
        let mut scanner = Scanner::new("class");

        let token = scanner.scan_token().unwrap();
        assert_eq!(TokenKind::Class, *token.kind());
        assert_eq!("class", token.lexeme().unwrap());
        assert_eq!(1, scanner.line());
    }

    #[test]
    fn scan_token_id_kw_else() {
        let mut scanner = Scanner::new("else");

        let token = scanner.scan_token().unwrap();
        assert_eq!(TokenKind::Else, *token.kind());
        assert_eq!("else", token.lexeme().unwrap());
        assert_eq!(1, scanner.line());
    }

    #[test]
    fn scan_token_id_kw_false() {
        let mut scanner = Scanner::new("false");

        let token = scanner.scan_token().unwrap();
        assert_eq!(TokenKind::False, *token.kind());
        assert_eq!("false", token.lexeme().unwrap());
        assert_eq!(1, scanner.line());
    }

    #[test]
    fn scan_token_id_kw_for() {
        let mut scanner = Scanner::new("for");

        let token = scanner.scan_token().unwrap();
        assert_eq!(TokenKind::For, *token.kind());
        assert_eq!("for", token.lexeme().unwrap());
        assert_eq!(1, scanner.line());
    }

    #[test]
    fn scan_token_id_kw_fun() {
        let mut scanner = Scanner::new("fun");

        let token = scanner.scan_token().unwrap();
        assert_eq!(TokenKind::Fun, *token.kind());
        assert_eq!("fun", token.lexeme().unwrap());
        assert_eq!(1, scanner.line());
    }

    #[test]
    fn scan_token_id_kw_if() {
        let mut scanner = Scanner::new("if");

        let token = scanner.scan_token().unwrap();
        assert_eq!(TokenKind::If, *token.kind());
        assert_eq!("if", token.lexeme().unwrap());
        assert_eq!(1, scanner.line());
    }

    #[test]
    fn scan_token_id_kw_nil() {
        let mut scanner = Scanner::new("nil");

        let token = scanner.scan_token().unwrap();
        assert_eq!(TokenKind::Nil, *token.kind());
        assert_eq!("nil", token.lexeme().unwrap());
        assert_eq!(1, scanner.line());
    }

    #[test]
    fn scan_token_id_kw_or() {
        let mut scanner = Scanner::new("or");

        let token = scanner.scan_token().unwrap();
        assert_eq!(TokenKind::Or, *token.kind());
        assert_eq!("or", token.lexeme().unwrap());
        assert_eq!(1, scanner.line());
    }

    #[test]
    fn scan_token_id_kw_print() {
        let mut scanner = Scanner::new("print");

        let token = scanner.scan_token().unwrap();
        assert_eq!(TokenKind::Print, *token.kind());
        assert_eq!("print", token.lexeme().unwrap());
        assert_eq!(1, scanner.line());
    }

    #[test]
    fn scan_token_id_kw_return() {
        let mut scanner = Scanner::new("return");

        let token = scanner.scan_token().unwrap();
        assert_eq!(TokenKind::Return, *token.kind());
        assert_eq!("return", token.lexeme().unwrap());
        assert_eq!(1, scanner.line());
    }

    #[test]
    fn scan_token_id_kw_super() {
        let mut scanner = Scanner::new("super");

        let token = scanner.scan_token().unwrap();
        assert_eq!(TokenKind::Super, *token.kind());
        assert_eq!("super", token.lexeme().unwrap());
        assert_eq!(1, scanner.line());
    }

    #[test]
    fn scan_token_id_kw_this() {
        let mut scanner = Scanner::new("this");

        let token = scanner.scan_token().unwrap();
        assert_eq!(TokenKind::This, *token.kind());
        assert_eq!("this", token.lexeme().unwrap());
        assert_eq!(1, scanner.line());
    }

    #[test]
    fn scan_token_id_kw_true() {
        let mut scanner = Scanner::new("true");

        let token = scanner.scan_token().unwrap();
        assert_eq!(TokenKind::True, *token.kind());
        assert_eq!("true", token.lexeme().unwrap());
        assert_eq!(1, scanner.line());
    }

    #[test]
    fn scan_token_id_kw_var() {
        let mut scanner = Scanner::new("var");

        let token = scanner.scan_token().unwrap();
        assert_eq!(TokenKind::Var, *token.kind());
        assert_eq!("var", token.lexeme().unwrap());
        assert_eq!(1, scanner.line());
    }

    #[test]
    fn scan_token_id_kw_while() {
        let mut scanner = Scanner::new("while");

        let token = scanner.scan_token().unwrap();
        assert_eq!(TokenKind::While, *token.kind());
        assert_eq!("while", token.lexeme().unwrap());
        assert_eq!(1, scanner.line());
    }

    #[test]
    fn scan_token_id_contains_but_not_exact() {
        let mut scanner = Scanner::new("variable");

        let token = scanner.scan_token().unwrap();
        assert_eq!(TokenKind::Identifier, *token.kind());
        assert_eq!("variable", token.lexeme().unwrap());
        assert_eq!(1, scanner.line());
    }

    #[test]
    fn iterator() {
        let scanner = Scanner::new(SOURCE);

        let expected_tokens = vec![
            Token::new(TokenKind::Print, 1, 0, Some("print")),
            Token::new(TokenKind::String, 1, 6, Some("\"This is a test\"")),
            Token::new(TokenKind::Var, 2, 23, Some("var")),
            Token::new(TokenKind::Identifier, 2, 27, Some("a")),
            Token::new(TokenKind::Equal, 2, 29, Some("=")),
            Token::new(TokenKind::Number, 2, 31, Some("1")),
            Token::new(TokenKind::Semicolon, 2, 32, Some(";")),
        ];

        let tokens: Vec<Token> = scanner.into_iter().collect();

        assert_eq!(expected_tokens, tokens);
    }
}
