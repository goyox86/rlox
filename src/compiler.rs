use std::{cell::RefCell, rc::Rc, str::FromStr};

use rlox_parser::scanner::{Scanner, Token, TokenKind};
use strum::{EnumCount, FromRepr};

use crate::{
    bytecode::{self, Chunk, OpCode},
    value::Value,
};

type ParseFn<'a> = fn(&'a Compiler<'a>) -> Result<(), CompilerError>;

#[derive(Copy, Clone, Debug, Default)]
pub struct ParseRule<'rule>(Option<ParseFn<'rule>>, Option<ParseFn<'rule>>, Precedence);

impl<'rule> ParseRule<'rule> {
    fn precedence(&self) -> &Precedence {
        &self.2
    }
}

// From lowest to higest precedence
#[derive(Copy, FromRepr, Clone, Debug, Default, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Precedence {
    #[default]
    None = 0,
    Assignment = 1,
    Or = 2,
    And = 3,
    Equality = 4,
    Comparison = 5,
    Term = 6,
    Factor = 7,
    Unary = 8,
    Call = 9,
    Primary = 10,
}

impl Precedence {
    fn higher(self) -> Precedence {
        match self {
            Precedence::Primary => Precedence::Primary,
            _ => Precedence::from_repr(self as u8 + 1)
                .expect("could not find a precedence with for the provided u8"),
        }
    }

    fn lower(self) -> Precedence {
        match self {
            Precedence::None => Precedence::None,
            _ => Precedence::from_repr(self as u8 - 1)
                .expect("could not find a precedence with for the provided u8"),
        }
    }
}

pub struct CompilerOptions {
    pub print_code: bool,
}

pub struct Compiler<'source> {
    source: &'source str,
    chunk: RefCell<Chunk>,
    parser: Parser<'source>,
    parse_rules: Vec<ParseRule<'source>>,
    options: CompilerOptions,
}

impl<'source> Compiler<'source> {
    pub fn new(source: &'source str, options: CompilerOptions) -> Self {
        let mut rules: [ParseRule; TokenKind::COUNT] =
            [ParseRule(None, None, Precedence::None); TokenKind::COUNT];
        rules[TokenKind::LeftParen as usize] =
            ParseRule(Some(Compiler::grouping), None, Precedence::None);
        rules[TokenKind::Minus as usize] = ParseRule(
            Some(Compiler::unary),
            Some(Compiler::binary),
            Precedence::Term,
        );
        rules[TokenKind::Plus as usize] = ParseRule(None, Some(Compiler::binary), Precedence::Term);
        rules[TokenKind::Slash as usize] =
            ParseRule(None, Some(Compiler::binary), Precedence::Factor);
        rules[TokenKind::Star as usize] =
            ParseRule(None, Some(Compiler::binary), Precedence::Factor);
        rules[TokenKind::Number as usize] =
            ParseRule(Some(Compiler::number), None, Precedence::None);
        rules;

        Self {
            source,
            chunk: RefCell::new(Chunk::new()),
            parser: Parser::new(source),
            parse_rules: Vec::from(rules),
            options,
        }
    }

    pub(crate) fn compile(&'source self) -> Result<Chunk, CompilerError> {
        self.parser.advance();

        self.expression()?;
        self.consume(TokenKind::Eof, "Expect end of expression.")?;
        self.end();

        Ok(self.chunk.borrow().clone())
    }

    fn expression(&'source self) -> Result<(), CompilerError> {
        self.parse_precedence(Precedence::Assignment);

        Ok(())
    }

    fn grouping(&'source self) -> Result<(), CompilerError> {
        self.expression()?;

        Ok(self.consume(TokenKind::RightParen, "Expect ')' after expression")?)
    }

    fn binary(&'source self) -> Result<(), CompilerError> {
        let previous_token = self.parser.previous().expect("expected token here");
        let rule = self.get_parse_rule(previous_token.kind());

        self.parse_precedence(rule.precedence().higher());

        match previous_token.kind() {
            TokenKind::Plus => Ok(self.emit_byte(OpCode::Add as u8)?),
            TokenKind::Minus => Ok(self.emit_byte(OpCode::Substract as u8)?),
            TokenKind::Star => Ok(self.emit_byte(OpCode::Multiply as u8)?),
            TokenKind::Slash => Ok(self.emit_byte(OpCode::Divide as u8)?),
            _ => return Ok(()),
        }
    }

    fn unary(&'source self) -> Result<(), CompilerError> {
        let previous_token = &self.parser.previous();

        let token_kind = previous_token.unwrap();

        self.expression();

        self.parse_precedence(Precedence::Unary);

        if let TokenKind::Minus = *token_kind.kind() {
            return Ok(self.emit_byte(OpCode::Negate as u8)?);
        }

        Ok(())
    }

    fn number(&self) -> Result<(), CompilerError> {
        let previous_token = self.parser.previous().unwrap();
        let number: f64 = f64::from_str(previous_token.lexeme().unwrap()).unwrap();
        let value = Value::Number(number);

        Ok(self.emit_constant(value))
    }

    fn consume(&self, token_kind: TokenKind, error_msg: &str) -> Result<(), CompilerError> {
        if *self.parser.current().expect("token expected here").kind() == token_kind {
            self.parser.advance();
            return Ok(());
        }

        Err(CompilerError(error_msg.to_string()))
    }

    fn end(&self) {
        self.emit_return();

        if self.options.print_code && !self.parser.had_error {
            bytecode::Disassembler::disassemble_chunk(&self.chunk.borrow(), "code");
        }
    }

    pub fn parse_precedence(&'source self, precedence: Precedence) -> Result<(), CompilerError> {
        self.parser.advance();

        let parse_rule = self.get_parse_rule(self.parser.previous().unwrap().kind());

        if parse_rule.0.is_none() {
            return Err(CompilerError("Expect expression.".into()));
        }

        parse_rule.0.unwrap()(self)?;

        let current_token = self.parser.current().expect("expected token here");
        let parse_rule = self.get_parse_rule(self.parser.current().unwrap().kind());
        while (precedence <= *parse_rule.precedence()) {
            self.parser.advance();
            let parse_rule = self.get_parse_rule(self.parser.previous().unwrap().kind());
            if parse_rule.1.is_some() {
                parse_rule.1.unwrap()(self)?;
            } else {
                return Ok(());
            }
        }

        Ok(())
    }

    fn emit_return(&self) -> Result<(), CompilerError> {
        self.emit_byte(OpCode::Return as u8)
    }

    fn emit_constant(&self, value: Value) {
        let constant_idx = self.make_constant(value);
        self.emit_bytes(OpCode::AddConstant as u8, constant_idx)
    }

    fn emit_byte(&self, byte: u8) -> Result<(), CompilerError> {
        let line = self
            .parser
            .previous()
            .expect("expected to have a token here")
            .line;

        Ok(self.chunk.borrow_mut().write(byte, line))
    }

    fn emit_bytes(&self, byte1: u8, byte2: u8) {
        self.emit_byte(byte1);
        self.emit_byte(byte2);
    }

    fn make_constant(&self, value: Value) -> u8 {
        let constant_idx = self.chunk.borrow_mut().add_constant(value) as u8;
        constant_idx
    }

    fn get_parse_rule(&self, token_kind: &TokenKind) -> ParseRule<'source> {
        self.parse_rules[*token_kind as usize]
    }
}

#[derive(Debug)]
pub struct CompilerError(String);

#[derive(Debug)]
struct Parser<'source> {
    current: RefCell<Option<Token<'source>>>,
    previous: RefCell<Option<Token<'source>>>,
    scanner: RefCell<Scanner<'source>>,
    source: &'source str,
    parse_rules: Vec<ParseRule<'source>>,
    had_error: bool,
    panic_mode: bool,
}

impl<'source> Parser<'source> {
    fn new(source: &'source str) -> Self {
        let mut rules: [ParseRule; TokenKind::COUNT] =
            [ParseRule(None, None, Precedence::None); TokenKind::COUNT];
        rules[TokenKind::LeftParen as usize] =
            ParseRule(Some(Compiler::grouping), None, Precedence::None);
        rules[TokenKind::Minus as usize] = ParseRule(
            Some(Compiler::unary),
            Some(Compiler::binary),
            Precedence::Term,
        );
        rules[TokenKind::Plus as usize] = ParseRule(None, Some(Compiler::binary), Precedence::Term);
        rules[TokenKind::Slash as usize] =
            ParseRule(None, Some(Compiler::binary), Precedence::Factor);
        rules[TokenKind::Star as usize] =
            ParseRule(None, Some(Compiler::binary), Precedence::Factor);
        rules[TokenKind::Number as usize] =
            ParseRule(Some(Compiler::number), None, Precedence::None);
        rules;

        Self {
            current: RefCell::new(None),
            previous: RefCell::new(None),
            source,
            scanner: RefCell::new(Scanner::new(source)),
            had_error: false,
            panic_mode: false,
            parse_rules: rules.to_vec(),
        }
    }

    fn advance(&self) {
        self.previous.swap(&self.current);

        let mut current_token = self.scanner.borrow_mut().scan_token();
        *self.current.borrow_mut() = Some(current_token);
    }

    // fn error_at_current(&mut self, msg: &str) {
    //     let current_token = self
    //         .current
    //         .as_ref()
    //         .expect("self.current must have a token at this point");
    //     self.error_at(current_token, msg);
    // }
    //
    // fn error_at(&mut self, token: &Token, msg: &str) {
    //     if self.panic_mode {
    //         return;
    //     };
    //     self.panic_mode = true;
    //
    //     eprintln!("[line {}] Error", token.line);
    //
    //     if token.is_eof() {
    //         eprintln!(" at end");
    //     } else {
    //         eprintln!(" at '{}'", token.lexeme().unwrap())
    //     }
    //
    //     eprintln!(": {}", msg);
    // }

    fn get_parse_rule(&self, token_kind: &TokenKind) -> ParseRule<'source> {
        self.parse_rules[*token_kind as usize]
    }

    pub fn current(&self) -> Option<Token<'source>> {
        self.current.borrow().to_owned()
    }

    pub fn previous(&self) -> Option<Token<'source>> {
        self.previous.borrow().to_owned()
    }
}
