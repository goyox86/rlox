use std::{cell::RefCell, rc::Rc, str::FromStr};

use rlox_parser::scanner::{Scanner, Token, TokenKind};
use strum::{EnumCount, FromRepr};

use crate::{
    bytecode::{self, Chunk, OpCode},
    value::Value,
};

type ParseFn = fn(&mut CompilerCtx) -> Result<(), CompilerError>;

#[derive(Copy, Clone, Default)]
pub struct ParseRule(Option<ParseFn>, Option<ParseFn>, Precedence);

impl ParseRule {
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
    options: CompilerOptions,
    parser: Parser<'source>,
    source: &'source str,
}

struct CompilerCtx<'source> {
    chunk: Chunk,
    parser: Parser<'source>,
    options: &'source CompilerOptions,
}

impl<'source> CompilerCtx<'source> {
    pub fn new(source: &'source str, options: &'source CompilerOptions) -> Self {
        Self {
            chunk: Chunk::new(),
            parser: Parser::new(source),
            options,
        }
    }
}

impl<'source> Compiler<'source> {
    pub fn new(source: &'source str, options: CompilerOptions) -> Self {
        Self {
            options,
            parser: Parser::new(source),
            source,
        }
    }

    pub fn compile(&self, source: &'source str) -> Result<Chunk, CompilerError> {
        let mut ctx = CompilerCtx::new(source, &self.options);

        ctx.parser.advance();

        expression(&mut ctx)?;
        consume(&mut ctx, TokenKind::Eof, "Expect end of expression.")?;
        end(&mut ctx);

        Ok(ctx.chunk)
    }
}

fn expression(ctx: &mut CompilerCtx) -> Result<(), CompilerError> {
    Ok(parse_precedence(ctx, Precedence::Assignment)?)
}

fn grouping(ctx: &mut CompilerCtx) -> Result<(), CompilerError> {
    expression(ctx)?;

    Ok(consume(
        ctx,
        TokenKind::RightParen,
        "Expect ')' after expression",
    )?)
}

fn binary(ctx: &mut CompilerCtx) -> Result<(), CompilerError> {
    let previous_token = ctx.parser.previous().expect("expected token here");
    let rule = ctx.parser.get_parse_rule(previous_token.kind());

    parse_precedence(ctx, rule.precedence().higher())?;

    match previous_token.kind() {
        TokenKind::Plus => Ok(emit_byte(ctx, OpCode::Add as u8)?),
        TokenKind::Minus => Ok(emit_byte(ctx, OpCode::Substract as u8)?),
        TokenKind::Star => Ok(emit_byte(ctx, OpCode::Multiply as u8)?),
        TokenKind::Slash => Ok(emit_byte(ctx, OpCode::Divide as u8)?),
        _ => return Ok(()),
    }
}

fn unary(ctx: &mut CompilerCtx) -> Result<(), CompilerError> {
    let previous_token = ctx.parser.previous();
    let token_kind = previous_token.unwrap();

    parse_precedence(ctx, Precedence::Unary)?;

    if let TokenKind::Minus = *token_kind.kind() {
        return Ok(emit_byte(ctx, OpCode::Negate as u8)?);
    }

    Ok(())
}

fn number(ctx: &mut CompilerCtx) -> Result<(), CompilerError> {
    let previous_token = ctx.parser.previous().unwrap();
    let number: f64 = f64::from_str(previous_token.lexeme().unwrap()).unwrap();
    let value = Value::Number(number);

    Ok(emit_constant(ctx, value))
}

fn consume(
    ctx: &mut CompilerCtx,
    token_kind: TokenKind,
    error_msg: &str,
) -> Result<(), CompilerError> {
    if *ctx.parser.current().expect("token expected here").kind() == token_kind {
        ctx.parser.advance();
        return Ok(());
    }

    Err(CompilerError {
        msg: error_msg.into(),
        line: ctx.parser.current().unwrap().line,
    })
}

fn end(ctx: &mut CompilerCtx) {
    emit_return(ctx);

    if ctx.options.print_code && !ctx.parser.had_error {
        bytecode::Disassembler::disassemble_chunk(&ctx.chunk, "code");
    }
}

fn parse_precedence(ctx: &mut CompilerCtx, precedence: Precedence) -> Result<(), CompilerError> {
    ctx.parser.advance();

    let mut prefix_rule = ctx
        .parser
        .get_parse_rule(ctx.parser.previous().unwrap().kind());

    let mut result = if let Some(prefix_rule) = prefix_rule.0 {
        prefix_rule(ctx)
    } else {
        Err(CompilerError {
            msg: "Expect expression.".into(),
            line: ctx.parser.current().unwrap().line,
        })
    };

    while (precedence
        <= *ctx
            .parser
            .get_parse_rule(ctx.parser.current().unwrap().kind())
            .precedence())
    {
        ctx.parser.advance();
        let infix_rule = ctx
            .parser
            .get_parse_rule(ctx.parser.previous().unwrap().kind());

        if let Some(infix_rule) = infix_rule.1 {
            result = infix_rule(ctx);
        }
    }

    result
}

fn emit_return(ctx: &mut CompilerCtx) -> Result<(), CompilerError> {
    emit_byte(ctx, OpCode::Return as u8)
}

fn emit_constant(ctx: &mut CompilerCtx, value: Value) {
    let constant_idx = make_constant(ctx, value);
    emit_bytes(ctx, OpCode::AddConstant as u8, constant_idx)
}

fn emit_byte(ctx: &mut CompilerCtx, byte: u8) -> Result<(), CompilerError> {
    let line = ctx
        .parser
        .previous()
        .expect("expected to have a token here")
        .line;

    Ok(ctx.chunk.write(byte, line))
}

fn emit_bytes(ctx: &mut CompilerCtx, byte1: u8, byte2: u8) {
    emit_byte(ctx, byte1);
    emit_byte(ctx, byte2);
}

fn make_constant(ctx: &mut CompilerCtx, value: Value) -> u8 {
    let constant_idx = ctx.chunk.add_constant(value) as u8;
    constant_idx
}

#[derive(Debug)]
pub struct CompilerError {
    msg: String,
    line: usize,
}

struct Parser<'source> {
    current: Option<Token<'source>>,
    parse_rules: Vec<ParseRule>,
    previous: Option<Token<'source>>,
    scanner: Scanner<'source>,
    source: &'source str,
    had_error: bool,
    panic_mode: bool,
}

impl<'source> Parser<'source> {
    fn new(source: &'source str) -> Self {
        let mut rules: [ParseRule; TokenKind::COUNT] =
            [ParseRule(None, None, Precedence::None); TokenKind::COUNT];
        rules[TokenKind::LeftParen as usize] = ParseRule(Some(grouping), None, Precedence::None);
        rules[TokenKind::Minus as usize] = ParseRule(Some(unary), Some(binary), Precedence::Term);
        rules[TokenKind::Plus as usize] = ParseRule(None, Some(binary), Precedence::Term);
        rules[TokenKind::Slash as usize] = ParseRule(None, Some(binary), Precedence::Factor);
        rules[TokenKind::Star as usize] = ParseRule(None, Some(binary), Precedence::Factor);
        rules[TokenKind::Number as usize] = ParseRule(Some(number), None, Precedence::None);
        rules;

        Self {
            current: None,
            previous: None,
            source,
            scanner: Scanner::new(source),
            had_error: false,
            panic_mode: false,
            parse_rules: rules.to_vec(),
        }
    }

    fn advance(&mut self) {
        self.previous = self.current;

        let mut current_token = self.scanner.scan_token();
        self.current = Some(current_token);
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

    fn get_parse_rule(&self, token_kind: &TokenKind) -> ParseRule {
        self.parse_rules[*token_kind as usize]
    }

    pub fn current(&self) -> Option<Token<'source>> {
        self.current
    }

    pub fn previous(&self) -> Option<Token<'source>> {
        self.previous
    }
}
