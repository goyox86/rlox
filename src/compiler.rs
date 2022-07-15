use std::{cell::RefCell, rc::Rc, str::FromStr};

use rlox_parser::scanner::{Scanner, Token, TokenKind};
use strum::{EnumCount, FromRepr};

use crate::{
    bytecode::{self, Chunk, OpCode},
    value::Value,
};

const PARSE_RULES: [ParseRule; TokenKind::COUNT] = [
    // [TOKEN_LEFT_PAREN]
    ParseRule(Some(grouping), None, Precedence::None),
    // [TOKEN_RIGHT_PAREN]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_LEFT_BRACE]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_RIGHT_BRACE]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_COMMA]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_DOT]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_MINUS]
    ParseRule(Some(unary), Some(binary), Precedence::Term),
    // [TOKEN_PLUS]
    ParseRule(None, Some(binary), Precedence::Term),
    // [TOKEN_SEMICOLON]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_SLASH]
    ParseRule(None, Some(binary), Precedence::Factor),
    // [TOKEN_STAR]
    ParseRule(None, Some(binary), Precedence::Factor),
    // [TOKEN_BANG]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_BANG_EQUAL]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_EQUAL]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_EQUAL_EQUAL]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_GREATER]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_GREATER_EQUAL]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_LESS]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_LESS_EQUAL]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_IDENTIFIER]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_STRING]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_NUMBER]
    ParseRule(Some(number), None, Precedence::None),
    // [TOKEN_AND]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_CLASS]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_ELSE]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_FALSE]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_FOR]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_FUN]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_IF]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_NIL]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_OR]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_PRINT]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_RETURN]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_SUPER]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_THIS]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_TRUE]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_VAR]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_WHILE]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_COMMENT]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_EOF]
    ParseRule(None, None, Precedence::None),
    // [TOKEN_DUMMY]
    ParseRule(None, None, Precedence::None),
];

type ParseFn = fn(&mut CompilerCtx) -> Result<(), CompilerError>;

#[derive(Copy, Clone, Default)]
pub struct ParseRule(Option<ParseFn>, Option<ParseFn>, Precedence);

impl ParseRule {
    fn prefix(&self) -> Option<ParseFn> {
        self.0
    }

    fn infix(&self) -> Option<ParseFn> {
        self.1
    }

    fn precedence(&self) -> Precedence {
        self.2
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

pub struct Compiler {
    options: CompilerOptions,
}

struct CompilerCtx<'source> {
    chunk: Chunk,
    previous: Token<'source>,
    current: Token<'source>,
    scanner: Scanner<'source>,
    source: &'source str,
    had_error: bool,
    panic_mode: bool,
    options: &'source CompilerOptions,
}

impl<'source> CompilerCtx<'source> {
    pub fn new(source: &'source str, options: &'source CompilerOptions) -> Self {
        Self {
            chunk: Chunk::new(),
            options,
            previous: Token::dummy(),
            current: Token::dummy(),
            source,
            scanner: Scanner::new(source),
            had_error: false,
            panic_mode: false,
        }
    }
}

impl Compiler {
    pub fn new(source: &str, options: CompilerOptions) -> Self {
        Self { options }
    }

    pub fn compile(&self, source: &str) -> Result<Chunk, CompilerError> {
        let mut ctx = CompilerCtx::new(source, &self.options);

        advance(&mut ctx);
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
    let previous_token = ctx.previous;
    let rule = get_parse_rule(ctx, previous_token.kind);

    parse_precedence(ctx, rule.precedence().higher())?;

    match previous_token.kind {
        TokenKind::Plus => Ok(emit_byte(ctx, OpCode::Add as u8)?),
        TokenKind::Minus => Ok(emit_byte(ctx, OpCode::Substract as u8)?),
        TokenKind::Star => Ok(emit_byte(ctx, OpCode::Multiply as u8)?),
        TokenKind::Slash => Ok(emit_byte(ctx, OpCode::Divide as u8)?),
        _ => return Ok(()),
    }
}

fn unary(ctx: &mut CompilerCtx) -> Result<(), CompilerError> {
    let token_kind = ctx.previous.kind;

    parse_precedence(ctx, Precedence::Unary)?;

    if let TokenKind::Minus = token_kind {
        return Ok(emit_byte(ctx, OpCode::Negate as u8)?);
    }

    Ok(())
}

fn number(ctx: &mut CompilerCtx) -> Result<(), CompilerError> {
    let previous_token = ctx.previous;
    let number: f64 = f64::from_str(previous_token.lexeme().unwrap()).unwrap();
    let value = Value::Number(number);

    Ok(emit_constant(ctx, value))
}

fn advance(ctx: &mut CompilerCtx) {
    ctx.previous = ctx.current;
    ctx.current = ctx.scanner.scan_token();
}

fn consume(
    ctx: &mut CompilerCtx,
    token_kind: TokenKind,
    error_msg: &str,
) -> Result<(), CompilerError> {
    if ctx.current.kind == token_kind {
        advance(ctx);
        return Ok(());
    }

    Err(CompilerError {
        msg: error_msg.into(),
        line: ctx.current.line,
    })
}

fn end(ctx: &mut CompilerCtx) {
    emit_return(ctx);

    if ctx.options.print_code && !ctx.had_error {
        bytecode::Disassembler::disassemble_chunk(&ctx.chunk, "code");
    }
}

fn parse_precedence(ctx: &mut CompilerCtx, precedence: Precedence) -> Result<(), CompilerError> {
    advance(ctx);

    let mut parse_rule = get_parse_rule(ctx, ctx.previous.kind);
    let mut result = if let Some(prefix_fn) = parse_rule.prefix() {
        prefix_fn(ctx)
    } else {
        Err(CompilerError {
            msg: "Expect expression.".into(),
            line: ctx.current.line,
        })
    };

    while (precedence <= get_parse_rule(ctx, ctx.current.kind).precedence()) {
        advance(ctx);

        let parse_rule = get_parse_rule(ctx, ctx.previous.kind);
        if let Some(infix_fn) = parse_rule.infix() {
            result = infix_fn(ctx);
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
    let line = ctx.previous.line;

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

fn get_parse_rule(ctx: &mut CompilerCtx, token_kind: TokenKind) -> ParseRule {
    assert_ne!(token_kind, TokenKind::Dummy);

    PARSE_RULES[token_kind as usize]
}

#[derive(Debug)]
pub struct CompilerError {
    msg: String,
    line: usize,
}
