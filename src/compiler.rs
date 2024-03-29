use std::str::FromStr;

use rlox_common::Array;
use strum::FromRepr;

use crate::{
    bytecode::{Chunk, Disassembler, OpCode},
    scanner::{Scanner, ScannerError, Token, TokenKind},
    string::String,
    value::Value,
    vm::{self, Vm, HEAP},
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

/// From lowest to higest precedence.
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
}

#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct CompilerOptions {
    pub print_code: bool,
}

pub(crate) struct Compiler<'c> {
    options: Option<&'c CompilerOptions>,
}

impl<'c> Compiler<'c> {
    pub fn new(options: Option<&'c CompilerOptions>) -> Self {
        Self { options }
    }

    pub fn compile(&self, source: &'c str) -> Result<Chunk, CompilerError> {
        let mut ctx = CompilerCtx::new(source, self.options);

        advance(&mut ctx);
        while (!matches(&mut ctx, TokenKind::Eof)) {
            declaration(&mut ctx)?;
        }
        end(&mut ctx);

        Ok(ctx.chunk)
    }
}

/// A local variable
#[derive(Clone, Copy, Debug)]
pub(crate) struct Local<'l> {
    name: Token<'l>,
    depth: isize,
}

impl<'l> Local<'l> {
    fn new(name: Token<'l>, depth: isize) -> Self {
        Self { name, depth }
    }
}

impl<'l> Default for Local<'l> {
    fn default() -> Self {
        Self {
            name: Token::dummy(),
            depth: 0,
        }
    }
}

/// The compilation context. This struct holds all the state needed during compilation.
pub(crate) struct CompilerCtx<'source> {
    chunk: Chunk,
    previous: Token<'source>,
    current: Token<'source>,
    scanner: Scanner<'source>,
    had_error: bool,
    panic_mode: bool,
    options: Option<&'source CompilerOptions>,
    local_count: isize,
    scope_depth: isize,
    locals: Array<Local<'source>>,
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
            local_count: 0,
            scope_depth: 0,
            locals: Array::new(),
        }
    }
}

fn declaration(ctx: &mut CompilerCtx) -> Result<(), CompilerError> {
    if matches(ctx, TokenKind::Var) {
        var_declaration(ctx)?
    } else {
        statement(ctx)?
    }

    if ctx.panic_mode {
        synchronize(ctx)?
    }

    Ok(())
}

fn statement(ctx: &mut CompilerCtx) -> Result<(), CompilerError> {
    if matches(ctx, TokenKind::Print) {
        print_statement(ctx)?;
    } else if matches(ctx, TokenKind::If) {
        if_statement(ctx)?;
    } else if matches(ctx, TokenKind::While) {
        while_statement(ctx)?;
    } else if matches(ctx, TokenKind::LeftBrace) {
        begin_scope(ctx);
        block(ctx)?;
        end_scope(ctx);
    } else {
        expression_statement(ctx)?;
    }

    Ok(())
}

fn if_statement(ctx: &mut CompilerCtx) -> Result<(), CompilerError> {
    consume(ctx, TokenKind::LeftParen, "expect '(' after 'if'.")?;
    expression(ctx)?;
    consume(ctx, TokenKind::RightParen, "expect ')' after condition.")?;

    let then_jump = emit_jump(ctx, OpCode::JumpIfFalse);
    emit_byte(ctx, OpCode::Pop as u8);
    statement(ctx)?;

    let else_jump = emit_jump(ctx, OpCode::Jump);
    patch_jump(ctx, then_jump);
    emit_byte(ctx, OpCode::Pop as u8);

    if matches(ctx, TokenKind::Else) {
        statement(ctx)?;
    }
    patch_jump(ctx, else_jump);

    Ok(())
}

fn while_statement(ctx: &mut CompilerCtx) -> Result<(), CompilerError> {
    let loop_start = ctx.chunk.code().len() as u16;

    consume(ctx, TokenKind::LeftParen, "expect '(' after 'while'.")?;
    expression(ctx)?;
    consume(ctx, TokenKind::RightParen, "expect ')' after condition.")?;

    let exit_jump = emit_jump(ctx, OpCode::JumpIfFalse);
    emit_byte(ctx, OpCode::Pop as u8);
    statement(ctx)?;
    emit_loop(ctx, loop_start);

    patch_jump(ctx, exit_jump);
    emit_byte(ctx, OpCode::Pop as u8);

    Ok(())
}

fn end_scope(ctx: &mut CompilerCtx) {
    ctx.scope_depth -= 1;

    while ctx.local_count > 0
        && (ctx.locals[(ctx.local_count - 1) as usize].depth > ctx.scope_depth)
    {
        emit_byte(ctx, OpCode::Pop as u8);
        ctx.local_count -= 1;
    }
}

fn begin_scope(ctx: &mut CompilerCtx) {
    ctx.scope_depth += 1;
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
    let number: f64 = f64::from_str(previous_token.lexeme()).unwrap();
    let value = Value::Number(number);

    emit_constant(ctx, value);
    Ok(())
}

fn string(ctx: &mut CompilerCtx, can_assign: bool) -> Result<(), CompilerError> {
    let lexeme = ctx.previous.lexeme();
    let chars = &lexeme[1..lexeme.len() - 1];
    let string_obj = String::new(chars);
    let string_value =
        Value::String(HEAP.with(|heap| heap.borrow_mut().allocate_string(string_obj)));

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
    named_variable(ctx, ctx.previous, can_assign)?;
    Ok(())
}

fn named_variable(
    ctx: &mut CompilerCtx,
    name: Token,
    can_assign: bool,
) -> Result<(), CompilerError> {
    let (mut get_op, mut set_op) = (OpCode::GetLocal as u8, OpCode::SetLocal as u8);
    let mut arg = resolve_local(ctx, name)?;

    if arg == -1 {
        arg = identifier_constant(ctx, name) as isize;
        get_op = OpCode::GetGlobal as u8;
        set_op = OpCode::SetGlobal as u8;
    }

    if can_assign && matches(ctx, TokenKind::Equal) {
        expression(ctx);
        emit_bytes(ctx, set_op, arg as u8);
    } else {
        emit_bytes(ctx, get_op, arg as u8);
    }

    Ok(())
}

fn resolve_local(ctx: &mut CompilerCtx, name: Token) -> Result<isize, CompilerError> {
    let current_locals = &ctx.locals[..ctx.local_count as usize];
    for (index, local) in current_locals.iter().enumerate() {
        if name == local.name {
            if local.depth == -1 {
                return Err(CompilerError {
                    msg: "can't read local variable in its own initializer.".into(),
                    line: ctx.current.line,
                });
            }
            return Ok(index as isize);
        }
    }

    Ok(-1)
}

fn block(ctx: &mut CompilerCtx) -> Result<(), CompilerError> {
    while (!check(ctx, TokenKind::RightBrace) && !check(ctx, TokenKind::Eof)) {
        declaration(ctx)?;
    }

    consume(ctx, TokenKind::RightBrace, "expect '}' after block.")?;
    Ok(())
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

fn var_declaration(ctx: &mut CompilerCtx) -> Result<(), CompilerError> {
    let global = parse_variable(ctx, "expect variable name.")?;

    if matches(ctx, TokenKind::Equal) {
        expression(ctx)?;
    } else {
        emit_byte(ctx, OpCode::AddNil as u8);
    }

    consume(
        ctx,
        TokenKind::Semicolon,
        "expect ';' after variable declaration.",
    )?;

    define_variable(ctx, global);

    Ok(())
}

fn parse_variable(ctx: &mut CompilerCtx, error_msg: &str) -> Result<u8, CompilerError> {
    consume(ctx, TokenKind::Identifier, error_msg)?;

    declare_variable(ctx)?;
    if ctx.scope_depth > 0 {
        return Ok(0);
    }

    let variable_index = identifier_constant(ctx, ctx.previous);

    Ok(variable_index)
}

fn make_initialized(ctx: &mut CompilerCtx) {
    ctx.locals[ctx.local_count as usize - 1].depth = ctx.scope_depth;
}

fn declare_variable(ctx: &mut CompilerCtx) -> Result<(), CompilerError> {
    if ctx.scope_depth == 0 {
        return Ok(());
    }

    let name = ctx.previous;
    for local in &ctx.locals[..ctx.local_count as usize] {
        if local.depth != -1 && local.depth < ctx.scope_depth {
            break;
        }

        if name == local.name {
            return Err(CompilerError {
                msg: "already a variable with this name in this scope.".into(),
                line: ctx.current.line,
            });
        }
    }

    add_local(ctx, ctx.previous)
}

fn define_variable(ctx: &mut CompilerCtx, global_index: u8) {
    if ctx.scope_depth > 0 {
        make_initialized(ctx);
        return;
    }

    emit_bytes(ctx, OpCode::DefineGlobal as u8, global_index);
}

fn and_(ctx: &mut CompilerCtx, can_assign: bool) -> Result<(), CompilerError> {
    let end_jump = emit_jump(ctx, OpCode::JumpIfFalse);
    emit_byte(ctx, OpCode::Pop as u8);

    parse_precedence(ctx, Precedence::And);
    patch_jump(ctx, end_jump);

    Ok(())
}

fn or_(ctx: &mut CompilerCtx, can_assign: bool) -> Result<(), CompilerError> {
    let else_jump = emit_jump(ctx, OpCode::JumpIfFalse);
    let end_jump = emit_jump(ctx, OpCode::Jump);

    patch_jump(ctx, else_jump);
    emit_byte(ctx, OpCode::Pop as u8);

    parse_precedence(ctx, Precedence::Or);
    patch_jump(ctx, end_jump);

    Ok(())
}

fn identifier_constant(ctx: &mut CompilerCtx, token: Token) -> u8 {
    let chars = &ctx.previous.lexeme();
    let string_obj = String::new(chars);
    let string_value =
        Value::String(HEAP.with(|heap| heap.borrow_mut().allocate_string(string_obj)));

    make_constant(ctx, string_value)
}

fn add_local<'ctx>(ctx: &mut CompilerCtx<'ctx>, name: Token<'ctx>) -> Result<(), CompilerError> {
    ctx.local_count += 1;
    let local = Local::new(name, -1);
    ctx.locals.push(local);

    Ok(())
}

fn expression_statement(ctx: &mut CompilerCtx) -> Result<(), CompilerError> {
    expression(ctx)?;
    consume(ctx, TokenKind::Semicolon, "expect ';' after expression.")?;
    emit_byte(ctx, OpCode::Pop as u8);
    Ok(())
}

fn print_statement(ctx: &mut CompilerCtx) -> Result<(), CompilerError> {
    expression(ctx)?;
    consume(ctx, TokenKind::Semicolon, "expect ';' after value.")?;
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

        advance(ctx)?;
    }

    Ok(())
}

#[inline]
fn advance(ctx: &mut CompilerCtx) -> Result<(), CompilerError> {
    ctx.previous = ctx.current;
    ctx.current = ctx.scanner.scan_token()?;
    Ok(())
}

#[inline]
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

#[inline]
fn end(ctx: &mut CompilerCtx) {
    emit_return(ctx);

    if let Some(options) = ctx.options {
        if options.print_code && !ctx.had_error {
            let bytecode = Disassembler::disassemble_chunk(&ctx.chunk, "code");
            println!("{}", bytecode);
        }
    }
}

#[inline]
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
        return Err(CompilerError {
            msg: "invalid assignment target.".into(),
            line: ctx.current.line,
        });
    }

    result
}

fn get_parse_rule(ctx: &mut CompilerCtx, token_kind: TokenKind) -> ParseRule {
    assert_ne!(token_kind, TokenKind::Dummy);

    match token_kind {
        TokenKind::LeftParen => ParseRule(Some(grouping), None, Precedence::None),
        TokenKind::RightParen => ParseRule(None, None, Precedence::None),
        TokenKind::LeftBrace => ParseRule(None, None, Precedence::None),
        TokenKind::RightBrace => ParseRule(None, None, Precedence::None),
        TokenKind::Comma => ParseRule(None, None, Precedence::None),
        TokenKind::Dot => ParseRule(None, None, Precedence::None),
        TokenKind::Minus => ParseRule(Some(unary), Some(binary), Precedence::Term),
        TokenKind::Plus => ParseRule(None, Some(binary), Precedence::Term),
        TokenKind::Semicolon => ParseRule(None, None, Precedence::None),
        TokenKind::Slash => ParseRule(None, Some(binary), Precedence::Factor),
        TokenKind::Star => ParseRule(None, Some(binary), Precedence::Factor),
        TokenKind::Bang => ParseRule(Some(unary), None, Precedence::None),
        TokenKind::BangEqual => ParseRule(None, Some(binary), Precedence::Equality),
        TokenKind::Equal => ParseRule(None, None, Precedence::None),
        TokenKind::EqualEqual => ParseRule(None, Some(binary), Precedence::Equality),
        TokenKind::Greater => ParseRule(None, Some(binary), Precedence::Comparison),
        TokenKind::GreaterEqual => ParseRule(None, Some(binary), Precedence::Comparison),
        TokenKind::Less => ParseRule(None, Some(binary), Precedence::Comparison),
        TokenKind::LessEqual => ParseRule(None, Some(binary), Precedence::Comparison),
        TokenKind::Identifier => ParseRule(Some(variable), None, Precedence::None),
        TokenKind::String => ParseRule(Some(string), None, Precedence::None),
        TokenKind::Number => ParseRule(Some(number), None, Precedence::None),
        TokenKind::And => ParseRule(None, Some(and_), Precedence::And),
        TokenKind::Class => ParseRule(None, None, Precedence::None),
        TokenKind::Else => ParseRule(None, None, Precedence::None),
        TokenKind::False => ParseRule(Some(literal), None, Precedence::None),
        TokenKind::For => ParseRule(None, None, Precedence::None),
        TokenKind::Fun => ParseRule(None, None, Precedence::None),
        TokenKind::If => ParseRule(None, None, Precedence::None),
        TokenKind::Nil => ParseRule(Some(literal), None, Precedence::None),
        TokenKind::Or => ParseRule(None, Some(or_), Precedence::Or),
        TokenKind::Print => ParseRule(None, None, Precedence::None),
        TokenKind::Return => ParseRule(None, None, Precedence::None),
        TokenKind::Super => ParseRule(None, None, Precedence::None),
        TokenKind::This => ParseRule(None, None, Precedence::None),
        TokenKind::True => ParseRule(Some(literal), None, Precedence::None),
        TokenKind::Var => ParseRule(None, None, Precedence::None),
        TokenKind::While => ParseRule(None, None, Precedence::None),
        TokenKind::Comment => ParseRule(None, None, Precedence::None),
        TokenKind::Eof => ParseRule(None, None, Precedence::None),
        TokenKind::Dummy => ParseRule(None, None, Precedence::None),
    }
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
fn emit_jump(ctx: &mut CompilerCtx, jump_op: OpCode) -> u16 {
    emit_byte(ctx, jump_op as u8);
    emit_byte(ctx, 0xff);
    emit_byte(ctx, 0xff);

    (ctx.chunk.len() - 2) as u16
}

#[inline(always)]
fn patch_jump(ctx: &mut CompilerCtx, offset: u16) {
    let jump = ctx.chunk.len() as u16 - offset - 2;
    let jump_bytes = jump.to_ne_bytes();

    let offset = offset as usize;
    ctx.chunk.code_mut()[offset] = jump_bytes[0];
    ctx.chunk.code_mut()[offset + 1] = jump_bytes[1];
}

#[inline(always)]
fn emit_loop(ctx: &mut CompilerCtx, loop_start: u16) {
    emit_byte(ctx, OpCode::Loop as u8);

    let offset = (ctx.chunk.code().len() as u16) - loop_start + 2;
    let offset_bytes = offset.to_ne_bytes();

    emit_byte(ctx, offset_bytes[0]);
    emit_byte(ctx, offset_bytes[1]);
}

#[inline(always)]
fn make_constant(ctx: &mut CompilerCtx, value: Value) -> u8 {
    ctx.chunk.add_constant(value) as u8
}

#[derive(Debug, PartialEq, Eq)]
pub struct CompilerError {
    msg: std::string::String,
    line: usize,
}

impl CompilerError {
    pub fn msg(&self) -> &std::string::String {
        &self.msg
    }

    pub fn line(&self) -> usize {
        self.line
    }
}

impl From<ScannerError> for CompilerError {
    fn from(scanner_error: ScannerError) -> Self {
        Self {
            msg: scanner_error.msg().to_owned(),
            line: scanner_error.line(),
        }
    }
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

    #[test]
    fn expr_stmt_missing_semicolon_error() {
        let compiler = Compiler::new(None);
        let expected_error = Err(CompilerError {
            msg: "expect ';' after expression.".into(),
            line: 1,
        });

        assert_eq!(expected_error, compiler.compile("2 + 2"));
    }

    #[test]
    fn var_decl_missing_semicolon_error() {
        let compiler = Compiler::new(None);
        let expected_error = Err(CompilerError {
            msg: "expect ';' after variable declaration.".into(),
            line: 1,
        });

        assert_eq!(expected_error, compiler.compile("var answer = 42"));
    }

    #[test]
    fn invalid_assigment_target_error() {
        let compiler = Compiler::new(None);
        let expected_error = Err(CompilerError {
            msg: "invalid assignment target.".into(),
            line: 1,
        });

        assert_eq!(expected_error, compiler.compile("2 + 2 = 42;"));
    }

    #[test]
    fn already_defined_local_error() {
        let compiler = Compiler::new(None);

        let expected_error = Err(CompilerError {
            msg: "already a variable with this name in this scope.".into(),
            line: 1,
        });
        assert_eq!(
            expected_error,
            compiler.compile("{ var a = \"foo\"; var a = \"bar\"; }")
        );
    }

    #[test]
    fn using_itself_in_initializer_error() {
        let compiler = Compiler::new(None);

        let expected_error = Err(CompilerError {
            msg: "can't read local variable in its own initializer.".into(),
            line: 1,
        });
        assert_eq!(expected_error, compiler.compile("{ var a = a; }"));
    }

    #[test]
    fn invalid_if_stmt_errors() {
        let compiler = Compiler::new(None);

        let expected_error = Err(CompilerError {
            msg: "expect ')' after condition.".into(),
            line: 1,
        });
        assert_eq!(expected_error, compiler.compile("if (a == 1 {}"));

        let expected_error = Err(CompilerError {
            msg: "expect '(' after 'if'.".into(),
            line: 1,
        });
        assert_eq!(expected_error, compiler.compile("if a == 1) {}"));
    }

    #[test]
    fn invalid_while_stmt_errors() {
        let compiler = Compiler::new(None);

        let expected_error = Err(CompilerError {
            msg: "expect ')' after condition.".into(),
            line: 1,
        });
        assert_eq!(expected_error, compiler.compile("while (a == 1 {}"));

        let expected_error = Err(CompilerError {
            msg: "expect '(' after 'while'.".into(),
            line: 1,
        });
        assert_eq!(expected_error, compiler.compile("while a == 1) {}"));
    }
}
