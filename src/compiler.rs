use std::str::FromStr;

use rustc_hash::FxHashMap;
use strum::FromRepr;

use crate::{
    bytecode::{Chunk, Disassembler, OpCode},
    object::Object,
    scanner::{Scanner, Token, TokenKind},
    string::String,
    value::Value,
    vm::{self, Vm},
};

pub(crate) type ParseFn = fn(&mut CompilerCtx, bool) -> Result<(), CompilerError>;

#[derive(Copy, Clone, Default)]
pub(crate) struct ParseRule(Option<ParseFn>, Option<ParseFn>, Precedence);

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

type ParseRules = FxHashMap<TokenKind, ParseRule>;

// From lowest to higest precedence
#[derive(Copy, FromRepr, Clone, Debug, Default, PartialEq, PartialOrd)]
#[repr(u8)]
pub(crate) enum Precedence {
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

    #[allow(dead_code)]
    fn lower(self) -> Precedence {
        match self {
            Precedence::None => Precedence::None,
            _ => Precedence::from_repr(self as u8 - 1)
                .expect("could not find a precedence with for the provided u8"),
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct CompilerOptions {
    pub print_code: bool,
}

pub(crate) struct Compiler<'c> {
    options: Option<&'c CompilerOptions>,
}

pub(crate) struct CompilerCtx<'source> {
    chunk: Chunk,
    previous: Token<'source>,
    current: Token<'source>,
    scanner: Scanner<'source>,
    parse_rules: ParseRules,
    had_error: bool,
    panic_mode: bool,
    options: Option<&'source CompilerOptions>,
}

impl<'source> CompilerCtx<'source> {
    pub fn new(source: &'source str, options: Option<&'source CompilerOptions>) -> Self {
        Self {
            chunk: Chunk::new(),
            options,
            previous: Token::dummy(),
            current: Token::dummy(),
            scanner: Scanner::new(source),
            had_error: false,
            panic_mode: false,
            parse_rules: Self::create_parse_rules(),
        }
    }

    fn create_parse_rules() -> ParseRules {
        let mut rules = FxHashMap::default();
        rules.insert(
            TokenKind::LeftParen,
            ParseRule(Some(grouping), None, Precedence::None),
        );
        rules.insert(
            TokenKind::RightParen,
            ParseRule(None, None, Precedence::None),
        );
        rules.insert(
            TokenKind::LeftBrace,
            ParseRule(None, None, Precedence::None),
        );
        rules.insert(
            TokenKind::RightBrace,
            ParseRule(None, None, Precedence::None),
        );
        rules.insert(TokenKind::Comma, ParseRule(None, None, Precedence::None));
        rules.insert(TokenKind::Dot, ParseRule(None, None, Precedence::None));
        rules.insert(
            TokenKind::Minus,
            ParseRule(Some(unary), Some(binary), Precedence::Term),
        );
        rules.insert(
            TokenKind::Plus,
            ParseRule(None, Some(binary), Precedence::Term),
        );
        rules.insert(
            TokenKind::Semicolon,
            ParseRule(None, None, Precedence::None),
        );
        rules.insert(
            TokenKind::Slash,
            ParseRule(None, Some(binary), Precedence::Factor),
        );
        rules.insert(
            TokenKind::Star,
            ParseRule(None, Some(binary), Precedence::Factor),
        );
        rules.insert(
            TokenKind::Bang,
            ParseRule(Some(unary), None, Precedence::None),
        );
        rules.insert(
            TokenKind::BangEqual,
            ParseRule(None, Some(binary), Precedence::Equality),
        );
        rules.insert(TokenKind::Equal, ParseRule(None, None, Precedence::None));
        rules.insert(
            TokenKind::EqualEqual,
            ParseRule(None, Some(binary), Precedence::Equality),
        );
        rules.insert(
            TokenKind::Greater,
            ParseRule(None, Some(binary), Precedence::Comparison),
        );
        rules.insert(
            TokenKind::GreaterEqual,
            ParseRule(None, Some(binary), Precedence::Comparison),
        );
        rules.insert(
            TokenKind::Less,
            ParseRule(None, Some(binary), Precedence::Comparison),
        );
        rules.insert(
            TokenKind::LessEqual,
            ParseRule(None, Some(binary), Precedence::Comparison),
        );
        rules.insert(
            TokenKind::Identifier,
            ParseRule(Some(variable), None, Precedence::None),
        );
        rules.insert(
            TokenKind::String,
            ParseRule(Some(string), None, Precedence::None),
        );
        rules.insert(
            TokenKind::Number,
            ParseRule(Some(number), None, Precedence::None),
        );
        rules.insert(TokenKind::And, ParseRule(None, None, Precedence::None));
        rules.insert(TokenKind::Class, ParseRule(None, None, Precedence::None));
        rules.insert(TokenKind::Else, ParseRule(None, None, Precedence::None));
        rules.insert(
            TokenKind::False,
            ParseRule(Some(literal), None, Precedence::None),
        );
        rules.insert(TokenKind::For, ParseRule(None, None, Precedence::None));
        rules.insert(TokenKind::Fun, ParseRule(None, None, Precedence::None));
        rules.insert(TokenKind::If, ParseRule(None, None, Precedence::None));
        rules.insert(
            TokenKind::Nil,
            ParseRule(Some(literal), None, Precedence::None),
        );
        rules.insert(TokenKind::Or, ParseRule(None, None, Precedence::None));
        rules.insert(TokenKind::Print, ParseRule(None, None, Precedence::None));
        rules.insert(TokenKind::Return, ParseRule(None, None, Precedence::None));
        rules.insert(TokenKind::Super, ParseRule(None, None, Precedence::None));
        rules.insert(TokenKind::This, ParseRule(None, None, Precedence::None));
        rules.insert(
            TokenKind::True,
            ParseRule(Some(literal), None, Precedence::None),
        );
        rules.insert(TokenKind::Var, ParseRule(None, None, Precedence::None));
        rules.insert(TokenKind::While, ParseRule(None, None, Precedence::None));
        rules.insert(TokenKind::Comment, ParseRule(None, None, Precedence::None));
        rules.insert(TokenKind::Eof, ParseRule(None, None, Precedence::None));
        rules.insert(TokenKind::Dummy, ParseRule(None, None, Precedence::None));

        rules
    }
}

impl<'c> Compiler<'c> {
    pub fn new(options: Option<&'c CompilerOptions>) -> Self {
        Self { options }
    }

    pub(crate) fn compile(&self, source: &'c str) -> Result<Chunk, CompilerError> {
        let mut ctx = CompilerCtx::new(source, self.options);

        advance(&mut ctx);
        while (!matches(&mut ctx, TokenKind::Eof)) {
            declaration(&mut ctx)?;
        }
        // expression(&mut ctx)?;
        // consume(&mut ctx, TokenKind::Eof, "expect end of expression.")?;
        end(&mut ctx);

        Ok(ctx.chunk)
    }
}

fn expression(ctx: &mut CompilerCtx) -> Result<(), CompilerError> {
    parse_precedence(ctx, Precedence::Assignment)
}

fn grouping(ctx: &mut CompilerCtx, can_assign: bool) -> Result<(), CompilerError> {
    expression(ctx)?;

    consume(ctx, TokenKind::RightParen, "expect ')' after expression.")
}

fn binary(ctx: &mut CompilerCtx, can_assign: bool) -> Result<(), CompilerError> {
    let previous_token = ctx.previous;
    let rule = get_parse_rule(ctx, previous_token.kind);

    parse_precedence(ctx, rule.precedence().higher())?;

    match previous_token.kind {
        TokenKind::BangEqual => emit_bytes(ctx, OpCode::Equal as u8, OpCode::Not as u8),
        TokenKind::EqualEqual => emit_byte(ctx, OpCode::Equal as u8),
        TokenKind::Greater => emit_byte(ctx, OpCode::Greater as u8),
        TokenKind::GreaterEqual => emit_bytes(ctx, OpCode::Less as u8, OpCode::Not as u8),
        TokenKind::Less => emit_byte(ctx, OpCode::Less as u8),
        TokenKind::LessEqual => emit_bytes(ctx, OpCode::Greater as u8, OpCode::Not as u8),
        TokenKind::Plus => emit_byte(ctx, OpCode::Add as u8),
        TokenKind::Minus => emit_byte(ctx, OpCode::Substract as u8),
        TokenKind::Star => emit_byte(ctx, OpCode::Multiply as u8),
        TokenKind::Slash => emit_byte(ctx, OpCode::Divide as u8),
        _ => (),
    };

    Ok(())
}

fn unary(ctx: &mut CompilerCtx, can_assign: bool) -> Result<(), CompilerError> {
    let token_kind = ctx.previous.kind;

    parse_precedence(ctx, Precedence::Unary)?;

    match token_kind {
        TokenKind::Bang => emit_byte(ctx, OpCode::Not as u8),
        TokenKind::Minus => emit_byte(ctx, OpCode::Negate as u8),
        _ => unreachable!(),
    }

    Ok(())
}

fn number(ctx: &mut CompilerCtx, can_assign: bool) -> Result<(), CompilerError> {
    let previous_token = ctx.previous;
    let number: f64 = f64::from_str(previous_token.lexeme().unwrap()).unwrap();
    let value = Value::Number(number);

    emit_constant(ctx, value);
    Ok(())
}

fn string(ctx: &mut CompilerCtx, can_assign: bool) -> Result<(), CompilerError> {
    let lexeme = ctx.previous.lexeme().unwrap();
    let chars = &lexeme[1..lexeme.len() - 1];
    let string_obj = String::new(chars);
    let string_value = Value::Obj(Object::allocate_string(string_obj));

    emit_constant(ctx, string_value);

    Ok(())
}

fn literal(ctx: &mut CompilerCtx, can_assign: bool) -> Result<(), CompilerError> {
    let previous_token = ctx.previous;

    match previous_token.kind {
        TokenKind::False => emit_byte(ctx, OpCode::AddFalse as u8),
        TokenKind::Nil => emit_byte(ctx, OpCode::AddNil as u8),
        TokenKind::True => emit_byte(ctx, OpCode::AddTrue as u8),
        _ => unreachable!(),
    }

    Ok(())
}

fn variable(ctx: &mut CompilerCtx, can_assign: bool) -> Result<(), CompilerError> {
    named_variable(ctx, ctx.previous, can_assign);

    Ok(())
}

fn named_variable(ctx: &mut CompilerCtx, name: Token, can_assign: bool) {
    let arg = identifier_constant(ctx, name);

    if can_assign && matches(ctx, TokenKind::Equal) {
        expression(ctx);
        emit_bytes(ctx, OpCode::SetGlobal as u8, arg);
    } else {
        emit_bytes(ctx, OpCode::GetGlobal as u8, arg);
    }
}

fn matches(ctx: &mut CompilerCtx, token_kind: TokenKind) -> bool {
    if !check(ctx, token_kind) {
        return false;
    }

    advance(ctx);
    true
}

fn check(ctx: &mut CompilerCtx, token_kind: TokenKind) -> bool {
    ctx.current.kind == token_kind
}

fn declaration(ctx: &mut CompilerCtx) -> Result<(), CompilerError> {
    if matches(ctx, TokenKind::Var) {
        var_declaration(ctx);
    } else {
        statement(ctx);
    }

    if ctx.panic_mode {
        synchronize(ctx);
    }

    Ok(())
}

fn var_declaration(ctx: &mut CompilerCtx) -> Result<(), CompilerError> {
    let global = parse_variable(ctx, "Expect variable name.")?;

    if matches(ctx, TokenKind::Equal) {
        expression(ctx)?;
    } else {
        emit_byte(ctx, OpCode::AddNil as u8);
    }

    consume(
        ctx,
        TokenKind::Semicolon,
        "Expect ';' after variable declaration.",
    )?;

    define_variable(ctx, global);

    Ok(())
}

fn define_variable(ctx: &mut CompilerCtx, global_index: u8) {
    emit_bytes(ctx, OpCode::DefineGlobal as u8, global_index as u8);
}

fn parse_variable(ctx: &mut CompilerCtx, error_msg: &str) -> Result<u8, CompilerError> {
    consume(ctx, TokenKind::Identifier, error_msg)?;
    let global_index = identifier_constant(ctx, ctx.previous);

    Ok(global_index)
}

fn identifier_constant(ctx: &mut CompilerCtx, token: Token) -> u8 {
    let lexeme = ctx.previous.lexeme().unwrap();
    let chars = &lexeme[1..lexeme.len() - 1];
    let string_obj = String::new(chars);
    let string_value = Value::Obj(Object::allocate_string(string_obj));
    make_constant(ctx, string_value)
}

fn statement(ctx: &mut CompilerCtx) -> Result<(), CompilerError> {
    if matches(ctx, TokenKind::Print) {
        print_statement(ctx)?;
    } else {
        expression_statement(ctx)?;
    }

    Ok(())
}

fn expression_statement(ctx: &mut CompilerCtx) -> Result<(), CompilerError> {
    expression(ctx)?;
    consume(ctx, TokenKind::Semicolon, "Expect ';' after expression.")?;
    emit_byte(ctx, OpCode::Pop as u8);
    Ok(())
}

fn print_statement(ctx: &mut CompilerCtx) -> Result<(), CompilerError> {
    expression(ctx)?;
    consume(ctx, TokenKind::Semicolon, "Expect ';' after value.")?;
    emit_byte(ctx, OpCode::Print as u8);
    Ok(())
}

fn synchronize(ctx: &mut CompilerCtx) -> Result<(), CompilerError> {
    ctx.panic_mode = false;

    while ctx.current.kind != TokenKind::Eof {
        if ctx.previous.kind == TokenKind::Semicolon {
            return Ok(());
        }

        if let TokenKind::Class
        | TokenKind::Fun
        | TokenKind::Var
        | TokenKind::For
        | TokenKind::If
        | TokenKind::While
        | TokenKind::Print
        | TokenKind::Return = ctx.current.kind
        {
            return Ok(());
        }

        advance(ctx)
    }

    Ok(())
}

#[inline(always)]
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

    if let Some(options) = ctx.options {
        if options.print_code && !ctx.had_error {
            let bytecode = Disassembler::disassemble_chunk(&ctx.chunk, "code");
            println!("{}", bytecode);
        }
    }
}

fn parse_precedence(ctx: &mut CompilerCtx, precedence: Precedence) -> Result<(), CompilerError> {
    advance(ctx);

    let can_assign = precedence <= Precedence::Assignment;
    let parse_rule = get_parse_rule(ctx, ctx.previous.kind);
    let mut result = if let Some(prefix_fn) = parse_rule.prefix() {
        prefix_fn(ctx, can_assign)
    } else {
        Err(CompilerError {
            msg: "expect expression.".into(),
            line: ctx.current.line,
        })
    };

    while precedence <= get_parse_rule(ctx, ctx.current.kind).precedence() {
        advance(ctx);
        let parse_rule = get_parse_rule(ctx, ctx.previous.kind);
        if let Some(infix_fn) = parse_rule.infix() {
            result = infix_fn(ctx, can_assign);
        }
    }

    if can_assign && matches(ctx, TokenKind::Equal) {
        result = Err(CompilerError {
            msg: "invalid assignment target.".into(),
            line: ctx.current.line,
        });
    }

    result
}

#[inline(always)]
fn emit_return(ctx: &mut CompilerCtx) {
    emit_byte(ctx, OpCode::Return as u8)
}

#[inline(always)]
fn emit_constant(ctx: &mut CompilerCtx, value: Value) {
    let constant_idx = make_constant(ctx, value);
    emit_bytes(ctx, OpCode::AddConstant as u8, constant_idx)
}

#[inline(always)]
fn emit_byte(ctx: &mut CompilerCtx, byte: u8) {
    let line = ctx.previous.line;

    ctx.chunk.write(byte, line)
}

#[inline(always)]
fn emit_bytes(ctx: &mut CompilerCtx, byte1: u8, byte2: u8) {
    emit_byte(ctx, byte1);
    emit_byte(ctx, byte2);
}

#[inline(always)]
fn make_constant(ctx: &mut CompilerCtx, value: Value) -> u8 {
    ctx.chunk.add_constant(value) as u8
}

fn get_parse_rule(ctx: &mut CompilerCtx, token_kind: TokenKind) -> ParseRule {
    assert_ne!(token_kind, TokenKind::Dummy);

    *ctx.parse_rules.get(&token_kind).unwrap()
}

#[derive(Debug, PartialEq, Eq)]
pub struct CompilerError {
    msg: std::string::String,
    line: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unary_negation_error() {
        let compiler = Compiler::new(None);
        let expected_error = Err(CompilerError {
            msg: "expect expression.".into(),
            line: 1,
        });

        assert_eq!(expected_error, compiler.compile("-"));
    }

    #[test]
    fn substraction_error() {
        let compiler = Compiler::new(None);
        let expected_error = Err(CompilerError {
            msg: "expect expression.".into(),
            line: 1,
        });

        assert_eq!(expected_error, compiler.compile("2 -"));
    }

    #[test]
    fn addition_error() {
        let compiler = Compiler::new(None);
        let expected_error = Err(CompilerError {
            msg: "expect expression.".into(),
            line: 1,
        });

        assert_eq!(expected_error, compiler.compile("2 +"));
    }

    #[test]
    fn multiplication_error() {
        let compiler = Compiler::new(None);
        let expected_error = Err(CompilerError {
            msg: "expect expression.".into(),
            line: 1,
        });

        assert_eq!(expected_error, compiler.compile("2 *"));
    }

    #[test]
    fn division_error() {
        let compiler = Compiler::new(None);
        let expected_error = Err(CompilerError {
            msg: "expect expression.".into(),
            line: 1,
        });

        assert_eq!(expected_error, compiler.compile("2 /"));
    }

    #[test]
    fn grouping_unclosed_paren_error() {
        let compiler = Compiler::new(None);
        let expected_error = Err(CompilerError {
            msg: "expect ')' after expression.".into(),
            line: 1,
        });

        assert_eq!(expected_error, compiler.compile("(2 + 2"));
    }
}
