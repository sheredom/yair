#[macro_use]
extern crate clap;
extern crate codemap;
extern crate logos;

use clap::App;
use codemap::*;
use logos::Logos;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Write};
use std::sync::Arc;
use yair::io::*;
use yair::*;

#[derive(PartialEq, Eq)]
enum PrecedenceGroup {
    Arithmetic,
    Bitwise,
    Cast,
    Parenthesis,
    Comparison,
}

#[derive(Debug)]
enum OperandKind {
    Pointer((yair::Value, Option<yair::Location>)),
    Concrete(yair::Value),
    Float(f64),
    Integer(u64),
}

#[derive(Debug)]
struct Operand {
    range: Range,
    kind: OperandKind,
}

fn get_precedence(x: Token) -> (PrecedenceGroup, u8) {
    match x {
        Token::Mul | Token::Div | Token::Mod => (PrecedenceGroup::Arithmetic, 0),
        Token::Add | Token::Sub => (PrecedenceGroup::Arithmetic, 1),
        Token::Not => (PrecedenceGroup::Bitwise, 0),
        Token::And => (PrecedenceGroup::Bitwise, 1),
        Token::Or => (PrecedenceGroup::Bitwise, 2),
        Token::Xor => (PrecedenceGroup::Bitwise, 3),
        Token::As => (PrecedenceGroup::Cast, 0),
        Token::LParen | Token::RParen => (PrecedenceGroup::Parenthesis, u8::MAX),
        Token::Equals
        | Token::NotEquals
        | Token::LessThan
        | Token::LessThanEquals
        | Token::GreaterThan
        | Token::GreaterThanEquals => (PrecedenceGroup::Comparison, 0),
        _ => todo!(),
    }
}

fn is_unary(x: Token) -> bool {
    matches!(x, Token::As | Token::Not)
}

#[derive(Logos, Copy, Clone, Debug, PartialEq)]
enum Token {
    #[token("package")]
    Package,

    #[token("return")]
    Return,

    #[token(":")]
    Colon,

    #[token(";")]
    Semicolon,

    #[token("[")]
    LBracket,

    #[token("]")]
    RBracket,

    #[token("{")]
    LCurly,

    #[token("}")]
    RCurly,

    #[token("(")]
    LParen,

    #[token(")")]
    RParen,

    #[token("+")]
    Add,

    #[token("-")]
    Sub,

    #[token("*")]
    Mul,

    #[token("/")]
    Div,

    #[token("%")]
    Mod,

    #[token("&")]
    And,

    #[token("|")]
    Or,

    #[token("^")]
    Xor,

    #[token("as")]
    As,

    #[token(",")]
    Comma,

    #[token("!")]
    Not,

    #[token("==")]
    Equals,

    #[token("!=")]
    NotEquals,

    #[token("<")]
    LessThan,

    #[token("<=")]
    LessThanEquals,

    #[token(">")]
    GreaterThan,

    #[token(">=")]
    GreaterThanEquals,

    #[token("=")]
    Assignment,

    #[token("if")]
    If,

    #[token("else")]
    Else,

    #[token("while")]
    While,

    #[token("break")]
    Break,

    #[token("continue")]
    Continue,

    #[token("in")]
    In,

    #[token("..")]
    Range,

    #[regex("[_a-zA-Z][_a-zA-Z0-9]*")]
    Identifier,

    #[regex("[1-9][0-9]*", priority = 2)]
    Integer,

    #[regex("[+-]?([0-9]+([.][0-9]*)?([eE][+-]?[0-9]+)?|[.][0-9]+([eE][+-]?[0-9]+)?)")]
    Float,

    #[token("true")]
    True,

    #[token("false")]
    False,

    #[regex("\"(\\.|[^\"])*\"")]
    String,

    #[error]
    // Skip whitespace.
    #[regex(r"[ \t\r\n\f]+", logos::skip)]
    // Skip single-line comments.
    #[regex("//[^\n\r]+", logos::skip)]
    // Skip multi-line comments.
    #[regex("/\\*[^*]*\\*+(?:[^/*][^*]*\\*+)*/", logos::skip)]
    Error,
}

type Range = std::ops::Range<usize>;

enum ParseError {
    UnexpectedEndOfFile,
    ExpectedTokenNotFound(Token, Range),
    InvalidExpression(Range),
    OperatorsInDifferentPrecedenceGroups(Range, Token, Range, Token),
    TypesDoNotMatch(Range, yair::Type, Range, yair::Type),
    InvalidNonConcreteConstantUsed(Range),
    InvalidNonConcreteConstantsUsed(Range, Range),
    UnknownIdentifier(Range),
    ComparisonOperatorsAlwaysNeedParenthesis(Range, Token, Range, Token),
    UnknownIdentifierUsedInAssignment(Range),
    IdentifierShadowsPreviouslyDeclareIdentifier(Range, Range),
    ElseStatementWithoutIf(Range),
}

struct Lexer<'a> {
    lexer: logos::Lexer<'a, Token>,
    token: Option<Token>,
    slice: &'a str,
    span: Range,
}

impl<'a> Lexer<'a> {
    pub fn new(data: &'a str) -> Self {
        let mut lexer = Token::lexer(&data);
        let slice = lexer.slice();
        let span = lexer.span();
        let token = lexer.next();

        Self {
            lexer,
            token,
            slice,
            span,
        }
    }

    pub fn peek(&self) -> (Option<Token>, &'a str, Range) {
        (self.token, self.slice, self.span.clone())
    }

    pub fn next(&mut self) -> Option<(Token, &'a str, Range)> {
        let next = self
            .token
            .map(|token| (token, self.slice, self.span.clone()));

        self.slice = self.lexer.slice();
        self.span = self.lexer.span();
        self.token = self.lexer.next();

        next
    }
}

#[allow(dead_code)]
struct Parser<'a> {
    codemap: CodeMap,
    file: Arc<codemap::File>,
    functions: HashMap<&'a str, yair::Function>,
    package: String,
    lexer: Lexer<'a>,
    identifiers: HashMap<&'a str, (yair::Type, yair::Value, Range)>,
    scoped_identifiers: Vec<Vec<&'a str>>,
    merge_blocks: Vec<yair::Block>,
    continue_blocks: Vec<yair::Block>,
}

impl<'a> Parser<'a> {
    pub fn new(name: String, data: &'a str) -> Parser<'a> {
        let mut codemap = CodeMap::new();

        let file = codemap.add_file(name, data.to_string());

        Parser {
            codemap,
            file,
            functions: HashMap::new(),
            package: "".to_string(),
            lexer: Lexer::new(&data),
            identifiers: HashMap::new(),
            scoped_identifiers: Vec::new(),
            merge_blocks: Vec::new(),
            continue_blocks: Vec::new(),
        }
    }

    fn peek(&mut self) -> Token {
        self.lexer.peek().0.map_or(Token::Error, |token| token)
    }

    fn get_next(&mut self) -> Option<Token> {
        self.lexer.next().map(|next| next.0)
    }

    fn get_current_range(&self) -> Range {
        self.lexer.peek().2
    }

    fn get_current_str(&self) -> &'a str {
        self.lexer.peek().1
    }

    fn get_current_location(&mut self, context: &mut yair::Context) -> Option<yair::Location> {
        let range = self.get_current_range();

        self.get_location(range, context)
    }

    fn get_location(&self, range: Range, context: &mut yair::Context) -> Option<yair::Location> {
        let span = self.file.span;

        let span = span.subspan(range.start as u64, range.end as u64);

        let location = self.codemap.look_up_span(span);

        Some(context.get_location(
            location.file.name(),
            location.begin.line + 1,
            location.begin.column + 1,
        ))
    }

    fn expect_symbol(&mut self, token: Token) -> Result<(), ParseError> {
        if let Some(next) = self.get_next() {
            if next == token {
                Ok(())
            } else {
                Err(ParseError::ExpectedTokenNotFound(
                    token,
                    self.get_current_range(),
                ))
            }
        } else {
            Err(ParseError::UnexpectedEndOfFile)
        }
    }

    fn get_kind(&mut self, kind: OperandKind, builder: &mut InstructionBuilder) -> OperandKind {
        if let OperandKind::Pointer((value, location)) = kind {
            OperandKind::Concrete(builder.load(value, location))
        } else {
            kind
        }
    }

    fn apply(
        &mut self,
        operand_stack: &mut Vec<Operand>,
        operator_stack: &mut Vec<(Range, Token)>,
        builder: &mut InstructionBuilder,
    ) -> Result<(), ParseError> {
        let y = operand_stack.pop().unwrap();

        let op = if let Some(op) = operator_stack.pop() {
            op
        } else {
            // No operator between operands `42 13`
            return Err(ParseError::InvalidExpression(y.range));
        };

        let location = self.get_location(op.0.clone(), builder.borrow_context());

        if is_unary(op.1) {
            // TODO: Check that not has a int or bool type!

            match self.get_kind(y.kind, builder) {
                OperandKind::Concrete(value) => {
                    let expr = match op.1 {
                        Token::Not => builder.not(value, location),
                        _ => todo!(),
                    };

                    operand_stack.push(Operand {
                        range: op.0,
                        kind: OperandKind::Concrete(expr),
                    });

                    Ok(())
                }
                _ => Err(ParseError::InvalidNonConcreteConstantUsed(y.range)),
            }
        } else {
            let x = operand_stack.pop().unwrap();

            let x_kind = self.get_kind(x.kind, builder);

            let y_kind = self.get_kind(y.kind, builder);

            let (x_value, y_value) = match x_kind {
                OperandKind::Concrete(x_value) => match y_kind {
                    OperandKind::Concrete(y_value) => {
                        let x_ty = x_value.get_type(builder.borrow_context());
                        let y_ty = y_value.get_type(builder.borrow_context());

                        if x_ty != y_ty {
                            return Err(ParseError::TypesDoNotMatch(x.range, x_ty, y.range, y_ty));
                        }

                        (x_value, y_value)
                    }
                    OperandKind::Float(y_float) => {
                        let x_ty = x_value.get_type(builder.borrow_context());

                        if !x_ty.is_float(builder.borrow_context()) {
                            let bits = x_ty.get_bits(builder.borrow_context());

                            let y_value = builder
                                .borrow_context()
                                .get_float_constant(bits as u8, y_float);

                            (x_value, y_value)
                        } else {
                            todo!()
                        }
                    }
                    OperandKind::Integer(y_int) => {
                        let x_ty = x_value.get_type(builder.borrow_context());

                        if x_ty.is_float(builder.borrow_context()) {
                            let bits = x_ty.get_bits(builder.borrow_context());

                            let y_value = builder
                                .borrow_context()
                                .get_float_constant(bits as u8, y_int as f64);

                            (x_value, y_value)
                        } else if x_ty.is_int(builder.borrow_context()) {
                            let bits = x_ty.get_bits(builder.borrow_context());

                            let y_value = builder
                                .borrow_context()
                                .get_int_constant(bits as u8, y_int as i64);

                            (x_value, y_value)
                        } else if x_ty.is_uint(builder.borrow_context()) {
                            let bits = x_ty.get_bits(builder.borrow_context());

                            let y_value = builder
                                .borrow_context()
                                .get_uint_constant(bits as u8, y_int);

                            (x_value, y_value)
                        } else {
                            todo!()
                        }
                    }
                    _ => panic!("Unhandled"),
                },
                OperandKind::Float(x_float) => match y_kind {
                    OperandKind::Concrete(y_value) => {
                        let y_ty = y_value.get_type(builder.borrow_context());

                        if !y_ty.is_float(builder.borrow_context()) {
                            let bits = y_ty.get_bits(builder.borrow_context());

                            let x_value = builder
                                .borrow_context()
                                .get_float_constant(bits as u8, x_float);

                            (x_value, y_value)
                        } else {
                            todo!()
                        }
                    }
                    _ => {
                        return Err(ParseError::InvalidNonConcreteConstantsUsed(
                            x.range, y.range,
                        ))
                    }
                },
                OperandKind::Integer(x_int) => match y_kind {
                    OperandKind::Concrete(y_value) => {
                        let y_ty = y_value.get_type(builder.borrow_context());

                        if y_ty.is_float(builder.borrow_context()) {
                            let bits = y_ty.get_bits(builder.borrow_context());

                            let x_value = builder
                                .borrow_context()
                                .get_float_constant(bits as u8, x_int as f64);

                            (x_value, y_value)
                        } else if y_ty.is_int(builder.borrow_context()) {
                            let bits = y_ty.get_bits(builder.borrow_context());

                            let x_value = builder
                                .borrow_context()
                                .get_int_constant(bits as u8, x_int as i64);

                            (x_value, y_value)
                        } else if y_ty.is_uint(builder.borrow_context()) {
                            let bits = y_ty.get_bits(builder.borrow_context());

                            let x_value = builder
                                .borrow_context()
                                .get_uint_constant(bits as u8, x_int);

                            (x_value, y_value)
                        } else {
                            todo!()
                        }
                    }
                    _ => {
                        return Err(ParseError::InvalidNonConcreteConstantsUsed(
                            x.range, y.range,
                        ))
                    }
                },
                _ => panic!("Unhandled"),
            };

            let expr = match op.1 {
                Token::Add => builder.add(x_value, y_value, location),
                Token::Sub => builder.sub(x_value, y_value, location),
                Token::Mul => builder.mul(x_value, y_value, location),
                Token::Div => builder.div(x_value, y_value, location),
                Token::Mod => builder.rem(x_value, y_value, location),
                Token::And => builder.and(x_value, y_value, location),
                Token::Or => builder.or(x_value, y_value, location),
                Token::Xor => builder.xor(x_value, y_value, location),
                Token::Equals => builder.cmp_eq(x_value, y_value, location),
                Token::NotEquals => builder.cmp_ne(x_value, y_value, location),
                Token::LessThan => builder.cmp_lt(x_value, y_value, location),
                Token::LessThanEquals => builder.cmp_le(x_value, y_value, location),
                Token::GreaterThan => builder.cmp_gt(x_value, y_value, location),
                Token::GreaterThanEquals => builder.cmp_ge(x_value, y_value, location),
                _ => todo!(),
            };

            operand_stack.push(Operand {
                range: op.0,
                kind: OperandKind::Concrete(expr),
            });

            Ok(())
        }
    }

    fn check_precedence(&mut self, x: (Range, Token), y: (Range, Token)) -> Result<(), ParseError> {
        let x_precedence = get_precedence(x.1);
        let y_precedence = get_precedence(y.1);

        if x_precedence.0 == PrecedenceGroup::Comparison
            || y_precedence.0 == PrecedenceGroup::Comparison
        {
            Err(ParseError::ComparisonOperatorsAlwaysNeedParenthesis(
                y.0.clone(),
                y.1,
                x.0.clone(),
                x.1,
            ))
        } else if x_precedence.0 == PrecedenceGroup::Parenthesis
            || y_precedence.0 == PrecedenceGroup::Parenthesis
        {
            // Parenthesis sit outside precedence groups because they are used to form pairs of precedence groups.
            Ok(())
        } else if x_precedence.0 != y_precedence.0 {
            Err(ParseError::OperatorsInDifferentPrecedenceGroups(
                y.0.clone(),
                y.1,
                x.0.clone(),
                x.1,
            ))
        } else {
            Ok(())
        }
    }

    fn apply_if_lower_precedence_and_push_operator(
        &mut self,
        x: (Range, Token),
        operand_stack: &mut Vec<Operand>,
        operator_stack: &mut Vec<(Range, Token)>,
        builder: &mut InstructionBuilder,
    ) -> Result<(), ParseError> {
        self.apply_if_lower_precedence(x.clone(), operand_stack, operator_stack, builder)?;

        operator_stack.push(x);

        Ok(())
    }

    fn next_token(&mut self) -> Result<Token, ParseError> {
        if let Some(next) = self.get_next() {
            Ok(next)
        } else {
            Err(ParseError::UnexpectedEndOfFile)
        }
    }

    fn apply_if_lower_precedence(
        &mut self,
        x: (Range, Token),
        operand_stack: &mut Vec<Operand>,
        operator_stack: &mut Vec<(Range, Token)>,
        builder: &mut InstructionBuilder,
    ) -> Result<(), ParseError> {
        let x_precedence = get_precedence(x.1);

        while !operator_stack.is_empty() {
            let y = operator_stack.last().unwrap();

            if x.1 == Token::RParen && y.1 == Token::LParen {
                operator_stack.pop();
                break;
            }

            self.check_precedence(x.clone(), y.clone())?;

            let y_precedence = get_precedence(y.1);

            if x_precedence.1 < y_precedence.1 {
                break;
            }

            self.apply(operand_stack, operator_stack, builder)?;
        }

        Ok(())
    }

    fn parse_integer(&mut self, _: &mut Context) -> Result<u64, ParseError> {
        if let Ok(i) = self.get_current_str().parse::<u64>() {
            Ok(i)
        } else {
            todo!()
        }
    }

    fn parse_float(&mut self, _: &mut Context) -> Result<f64, ParseError> {
        if let Ok(f) = self.get_current_str().parse::<f64>() {
            Ok(f)
        } else {
            todo!()
        }
    }

    fn parse_expression(
        &mut self,
        ty: Type,
        builder: &mut InstructionBuilder,
    ) -> Result<yair::Value, ParseError> {
        let mut operand_stack = Vec::new();
        let mut operator_stack: Vec<(Range, Token)> = Vec::new();

        if ty.is_array(builder.borrow_context()) {
            // Array initializers always start with a  '{'.
            self.expect_symbol(Token::LCurly)?;

            let element_ty = ty.get_element(builder.borrow_context(), 0);

            let mut initializer = builder.borrow_context().get_undef(ty);

            for i in 0..ty.get_len(builder.borrow_context()) {
                let location = self.get_current_location(builder.borrow_context());
                let expr = self.parse_expression(element_ty, builder)?;

                initializer = builder.insert(initializer, expr, i, location);

                if self.peek() != Token::RCurly {
                    self.expect_symbol(Token::Comma)?;
                }
            }

            // Array initializers always end with a '}'.
            self.expect_symbol(Token::RCurly)?;

            return Ok(initializer);
        }

        loop {
            if let Some(peek) = self.lexer.peek().0 {
                match peek {
                    Token::Semicolon => break,
                    Token::LCurly => break,
                    Token::RCurly => break,
                    Token::RBracket => break,
                    Token::Comma => break,
                    _ => (),
                }
            }

            match self.get_next() {
                Some(Token::True) => operand_stack.push(Operand {
                    range: self.get_current_range(),
                    kind: OperandKind::Concrete(builder.borrow_context().get_bool_constant(true)),
                }),
                Some(Token::False) => operand_stack.push(Operand {
                    range: self.get_current_range(),
                    kind: OperandKind::Concrete(builder.borrow_context().get_bool_constant(false)),
                }),
                Some(Token::Integer) => operand_stack.push(Operand {
                    range: self.get_current_range(),
                    kind: OperandKind::Integer(self.parse_integer(builder.borrow_context())?),
                }),
                Some(Token::Float) => operand_stack.push(Operand {
                    range: self.get_current_range(),
                    kind: OperandKind::Float(self.parse_float(builder.borrow_context())?),
                }),
                Some(Token::LParen) => {
                    operator_stack.push((self.get_current_range(), Token::LParen))
                }
                Some(Token::RParen) => self.apply_if_lower_precedence(
                    (self.get_current_range(), Token::RParen),
                    &mut operand_stack,
                    &mut operator_stack,
                    builder,
                )?,
                Some(Token::As) => {
                    let range = self.get_current_range();

                    if !operator_stack.is_empty() {
                        self.check_precedence(
                            (range.clone(), Token::As),
                            operator_stack.last().unwrap().clone(),
                        )?;
                    }

                    let ty = self.parse_type(None, builder.borrow_context())?;

                    let expr = if let Some(x) = operand_stack.pop() {
                        let kind = self.get_kind(x.kind, builder);

                        match kind {
                            OperandKind::Concrete(v) => {
                                let location =
                                    self.get_location(range.clone(), builder.borrow_context());
                                builder.cast(v, ty, location)
                            }
                            OperandKind::Float(f) => {
                                if ty.is_float(builder.borrow_context()) {
                                    let bits = ty.get_bits(builder.borrow_context());
                                    builder.borrow_context().get_float_constant(bits as u8, f)
                                } else {
                                    todo!()
                                }
                            }
                            OperandKind::Integer(i) => {
                                if ty.is_float(builder.borrow_context()) {
                                    let bits = ty.get_bits(builder.borrow_context());
                                    builder
                                        .borrow_context()
                                        .get_float_constant(bits as u8, i as f64)
                                } else if ty.is_int(builder.borrow_context()) {
                                    let bits = ty.get_bits(builder.borrow_context());
                                    builder
                                        .borrow_context()
                                        .get_int_constant(bits as u8, i as i64)
                                } else if ty.is_uint(builder.borrow_context()) {
                                    let bits = ty.get_bits(builder.borrow_context());
                                    builder.borrow_context().get_uint_constant(bits as u8, i)
                                } else {
                                    todo!()
                                }
                            }
                            _ => panic!("Unhandled"),
                        }
                    } else {
                        todo!()
                    };

                    operand_stack.push(Operand {
                        range: range.clone(),
                        kind: OperandKind::Concrete(expr),
                    });
                }
                Some(x)
                    if matches!(
                        x,
                        Token::Add
                            | Token::Sub
                            | Token::Mul
                            | Token::Div
                            | Token::Mod
                            | Token::Not
                            | Token::And
                            | Token::Or
                            | Token::Xor
                            | Token::Equals
                            | Token::NotEquals
                            | Token::LessThan
                            | Token::LessThanEquals
                            | Token::GreaterThan
                            | Token::GreaterThanEquals
                    ) =>
                {
                    self.apply_if_lower_precedence_and_push_operator(
                        (self.get_current_range(), x),
                        &mut operand_stack,
                        &mut &mut operator_stack,
                        builder,
                    )?
                }
                Some(Token::Identifier) => {
                    let identifier = self.get_current_str();

                    if let Some((_, value, range)) = self.identifiers.get(identifier) {
                        let location = self.get_location(range.clone(), builder.borrow_context());
                        operand_stack.push(Operand {
                            range: self.get_current_range(),
                            kind: OperandKind::Pointer((*value, location)),
                        });
                    } else {
                        return Err(ParseError::UnknownIdentifier(self.get_current_range()));
                    }
                }
                Some(Token::LBracket) => {
                    let range = self.get_current_range();
                    let location = self.get_current_location(builder.borrow_context());

                    let u64_ty = builder.borrow_context().get_uint_type(64);
                    let index = self.parse_expression(u64_ty, builder)?;

                    let operand = operand_stack.pop().unwrap();

                    let operand = if let OperandKind::Pointer(ptr) = operand.kind {
                        ptr.0
                    } else {
                        todo!()
                    };

                    let expr = builder.index_into(operand, &[index], location);

                    self.expect_symbol(Token::RBracket)?;

                    operand_stack.push(Operand {
                        range,
                        kind: OperandKind::Pointer((expr, location)),
                    });
                }
                Some(_) => return Err(ParseError::UnknownIdentifier(self.get_current_range())),
                None => todo!(),
            }
        }

        // Handle the case where an expression is malformed like `foo=;`
        if operand_stack.is_empty() {
            return Err(ParseError::InvalidExpression(self.get_current_range()));
        }

        while !operator_stack.is_empty() {
            self.apply(&mut operand_stack, &mut operator_stack, builder)?;
        }

        let operand = operand_stack.pop().unwrap();

        let expr = match operand.kind {
            OperandKind::Pointer((value, location)) => builder.load(value, location),
            OperandKind::Concrete(v) => v,
            OperandKind::Float(f) => {
                if ty.is_float(builder.borrow_context()) {
                    let bits = ty.get_bits(builder.borrow_context());
                    builder.borrow_context().get_float_constant(bits as u8, f)
                } else {
                    todo!();
                }
            }
            OperandKind::Integer(i) => {
                if ty.is_float(builder.borrow_context()) {
                    let bits = ty.get_bits(builder.borrow_context());
                    builder
                        .borrow_context()
                        .get_float_constant(bits as u8, i as f64)
                } else if ty.is_int(builder.borrow_context()) {
                    let bits = ty.get_bits(builder.borrow_context());
                    builder
                        .borrow_context()
                        .get_int_constant(bits as u8, i as i64)
                } else if ty.is_uint(builder.borrow_context()) {
                    let bits = ty.get_bits(builder.borrow_context());
                    builder.borrow_context().get_uint_constant(bits as u8, i)
                } else {
                    todo!();
                }
            }
        };

        Ok(expr)
    }

    fn push_scope(&mut self) {
        self.scoped_identifiers.push(Vec::new());
    }

    fn pop_scope(&mut self) {
        for identifier in self.scoped_identifiers.last().unwrap().iter() {
            self.identifiers.remove(identifier).unwrap();
        }

        self.scoped_identifiers.pop();
    }

    fn add_identifier(
        &mut self,
        identifier: &'a str,
        range: Range,
        ty: yair::Type,
        value: yair::Value,
    ) -> Result<(), ParseError> {
        if let Some(original) = self
            .identifiers
            .insert(identifier, (ty, value, range.clone()))
        {
            return Err(ParseError::IdentifierShadowsPreviouslyDeclareIdentifier(
                original.2, range,
            ));
        }

        self.scoped_identifiers.last_mut().unwrap().push(identifier);

        Ok(())
    }

    fn parse_if(
        &mut self,
        function: yair::Function,
        alloca_block: yair::Block,
        condition: yair::Value,
        block: yair::Block,
        context: &mut yair::Context,
    ) -> Result<yair::Block, ParseError> {
        self.expect_symbol(Token::LCurly)?;

        let location = self.get_location(self.lexer.peek().2, context);

        let mut exit_blocks = Vec::new();

        let (if_entry_block, if_exit_block) = self.parse_block(function, alloca_block, context)?;

        exit_blocks.push((if_exit_block, self.get_current_location(context)));

        let false_block = if self.peek() == Token::Else {
            self.expect_symbol(Token::Else)?;

            if self.peek() == Token::If {
                self.expect_symbol(Token::If)?;

                let else_if_block = function.create_block(context).build();

                let mut builder = else_if_block.create_instructions(context);

                let bool_ty = builder.borrow_context().get_bool_type();

                let expr = self.parse_expression(bool_ty, &mut builder)?;

                let exit_block = self.parse_if(
                    function,
                    alloca_block,
                    expr,
                    else_if_block,
                    builder.borrow_context(),
                )?;

                exit_blocks.push((exit_block, self.get_current_location(context)));

                Some(else_if_block)
            } else {
                self.expect_symbol(Token::LCurly)?;

                let (else_entry_block, else_exit_block) =
                    self.parse_block(function, alloca_block, context)?;

                exit_blocks.push((else_exit_block, self.get_current_location(context)));

                Some(else_entry_block)
            }
        } else {
            None
        };

        let merge_block = function.create_block(context).build();

        let false_block = false_block.map_or(merge_block, |block| block);

        block.create_instructions(context).conditional_branch(
            condition,
            if_entry_block,
            false_block,
            &[],
            &[],
            location,
        );

        for (exit_block, location) in exit_blocks {
            if !exit_block.has_terminator(context) {
                exit_block
                    .create_instructions(context)
                    .branch(merge_block, &[], location);
            }
        }

        Ok(merge_block)
    }

    fn parse_while(
        &mut self,
        function: yair::Function,
        alloca_block: yair::Block,
        block: yair::Block,
        context: &mut yair::Context,
    ) -> Result<yair::Block, ParseError> {
        let check_block = function.create_block(context).build();
        let body_block = function.create_block(context).build();
        let merge_block = function.create_block(context).build();

        self.merge_blocks.push(merge_block);
        self.continue_blocks.push(check_block);

        block
            .create_instructions(context)
            .branch(check_block, &[], None);

        let mut builder = check_block.create_instructions(context);

        let bool_ty = builder.borrow_context().get_bool_type();

        let condition = self.parse_expression(bool_ty, &mut builder)?;

        let location = self.get_current_location(builder.borrow_context());
        builder.conditional_branch(condition, body_block, merge_block, &[], &[], location);

        self.expect_symbol(Token::LCurly)?;

        let (while_entry_block, while_exit_block) =
            self.parse_block(function, alloca_block, context)?;

        body_block
            .create_instructions(context)
            .branch(while_entry_block, &[], None);

        if !while_exit_block.has_terminator(context) {
            while_exit_block
                .create_instructions(context)
                .branch(check_block, &[], None);
        }

        self.merge_blocks.pop();
        self.continue_blocks.pop();

        Ok(merge_block)
    }

    fn parse_block(
        &mut self,
        function: yair::Function,
        alloca_block: yair::Block,
        context: &mut yair::Context,
    ) -> Result<(yair::Block, yair::Block), ParseError> {
        self.push_scope();

        let entry_block = function.create_block(context).build();

        let mut current_block = entry_block;
        let mut builder = current_block.create_instructions(context);

        loop {
            match self.get_next() {
                Some(Token::LCurly) => {
                    let location = self.get_location(self.lexer.peek().2, builder.borrow_context());

                    let (sub_entry_block, sub_exit_block) =
                        self.parse_block(function, alloca_block, builder.borrow_context())?;

                    current_block
                        .create_instructions(builder.borrow_context())
                        .branch(sub_entry_block, &[], location);

                    let location = self.get_location(self.lexer.peek().2, builder.borrow_context());

                    current_block = function.create_block(builder.borrow_context()).build();

                    if !sub_exit_block.has_terminator(builder.borrow_context()) {
                        sub_exit_block
                            .create_instructions(builder.borrow_context())
                            .branch(current_block, &[], location);
                    }
                    builder = current_block.create_instructions(context);
                }
                Some(Token::RCurly) => {
                    self.pop_scope();
                    return Ok((entry_block, current_block));
                }
                Some(Token::Identifier) => {
                    let identifier = self.get_current_str();
                    let identifier_range = self.get_current_range();

                    match self.next_token()? {
                        Token::Colon => {
                            let location = self.get_current_location(builder.borrow_context());

                            let ty = self.parse_type(None, builder.borrow_context())?;

                            let stack_alloc = {
                                let mut builder =
                                    alloca_block.create_instructions(builder.borrow_context());

                                let alloc = builder.stack_alloc(identifier, ty, location);

                                builder.pause_building();

                                alloc
                            };

                            self.add_identifier(identifier, identifier_range, ty, stack_alloc)?;

                            let (expr, location) = match self.next_token()? {
                                Token::Assignment => (
                                    self.parse_expression(ty, &mut builder)?,
                                    self.get_current_location(builder.borrow_context()),
                                ),
                                _ => todo!(),
                            };

                            self.expect_symbol(Token::Semicolon)?;

                            builder.store(stack_alloc, expr, location);
                        }
                        Token::Assignment => {
                            let location = self.get_current_location(builder.borrow_context());

                            let (ty, stack_alloc) =
                                if let Some(identifier) = self.identifiers.get(identifier) {
                                    (identifier.0, identifier.1)
                                } else {
                                    return Err(ParseError::UnknownIdentifierUsedInAssignment(
                                        identifier_range,
                                    ));
                                };

                            let expr = self.parse_expression(ty, &mut builder)?;

                            self.expect_symbol(Token::Semicolon)?;

                            builder.store(stack_alloc, expr, location);
                        }
                        _ => todo!(),
                    }
                }
                Some(Token::Return) => {
                    let location = self.get_current_location(builder.borrow_context());

                    let return_ty = function.get_return_type(builder.borrow_context());

                    let expr = self.parse_expression(return_ty, &mut builder)?;

                    self.expect_symbol(Token::Semicolon)?;

                    builder.ret_val(expr, location);

                    return Ok((entry_block, current_block));
                }
                Some(Token::If) => {
                    let bool_ty = builder.borrow_context().get_bool_type();

                    let expr = self.parse_expression(bool_ty, &mut builder)?;

                    let exit_block = self.parse_if(
                        function,
                        alloca_block,
                        expr,
                        current_block,
                        builder.borrow_context(),
                    )?;

                    current_block = exit_block;
                    builder = current_block.create_instructions(context);
                }
                Some(Token::While) => {
                    let exit_block = self.parse_while(
                        function,
                        alloca_block,
                        current_block,
                        builder.borrow_context(),
                    )?;

                    current_block = exit_block;
                    builder = current_block.create_instructions(context);

                    /*
                    This is for a for statement

                    let identifier = self.get_current_str();
                    let identifier_range = self.get_current_range();

                    self.push_scope();

                    self.expect_symbol(Token::Colon)?;

                    let location = self.get_current_location(builder.borrow_context());

                    let ty = self.parse_type(None, builder.borrow_context())?;

                    let stack_alloc = {
                        let mut builder =
                            alloca_block.create_instructions(builder.borrow_context());

                        let alloc = builder.stack_alloc(identifier, ty, location);

                        builder.pause_building();

                        alloc
                    };

                    self.add_identifier(identifier, identifier_range, ty, stack_alloc)?;

                    self.expect_symbol(Token::In)?;

                    let start = self.parse_expression(ty, &mut builder)?;

                    self.expect_symbol(Token::Range)?;

                    let end = self.parse_expression(ty, &mut builder)?;

                    self.expect_symbol(Token::LCurly)?;*/
                }
                Some(Token::Break) => {
                    let location = self.get_current_location(builder.borrow_context());

                    let merge_block = if let Some(merge_block) = self.merge_blocks.last() {
                        *merge_block
                    } else {
                        panic!("Don't have a merge block (break wasn't in a loop).");
                    };

                    builder.branch(merge_block, &[], location);

                    self.expect_symbol(Token::Semicolon)?;

                    self.expect_symbol(Token::RCurly)?;

                    return Ok((entry_block, current_block));
                }
                Some(Token::Continue) => {
                    let location = self.get_current_location(builder.borrow_context());

                    let continue_block = if let Some(continue_block) = self.continue_blocks.last() {
                        *continue_block
                    } else {
                        panic!("Don't have a continnue block (continue wasn't in a loop).");
                    };

                    builder.branch(continue_block, &[], location);

                    self.expect_symbol(Token::Semicolon)?;

                    self.expect_symbol(Token::RCurly)?;

                    return Ok((entry_block, current_block));
                }
                Some(Token::Else) => {
                    // We parse this in the if statement parsing, so if we find one here, its hanging around with no if!
                    return Err(ParseError::ElseStatementWithoutIf(self.get_current_range()));
                }
                Some(token) => panic!("Unhandled {:?}", token),
                None => return Err(ParseError::UnexpectedEndOfFile),
            }
        }
    }

    fn parse_function(
        &mut self,
        identifier: &str,
        context: &mut yair::Context,
    ) -> Result<yair::Type, ParseError> {
        self.expect_symbol(Token::LParen)?;

        let mut args = Vec::new();

        let mut parsed_one_arg = false;

        loop {
            match self.get_next() {
                Some(Token::RParen) => break,
                Some(Token::Identifier) => {
                    let name = self.get_current_str();
                    let range: Range = self.get_current_range();

                    if Some(Token::Colon) != self.get_next() {
                        return Err(ParseError::ExpectedTokenNotFound(
                            Token::Colon,
                            self.get_current_range(),
                        ));
                    }

                    // TODO: We should check that we aren't parsing a function definition again here!
                    let ty = self.parse_type(None, context)?;

                    args.push((name, range, ty));

                    parsed_one_arg = true;
                }
                Some(Token::Comma) => {
                    if !parsed_one_arg {
                        return Err(ParseError::ExpectedTokenNotFound(
                            Token::Comma,
                            self.get_current_range(),
                        ));
                    }
                }
                Some(_) => return Err(ParseError::InvalidExpression(self.get_current_range())),
                None => return Err(ParseError::UnexpectedEndOfFile),
            }
        }

        self.expect_symbol(Token::Colon)?;

        let return_ty = self.parse_type(None, context)?;

        let module = if let Some(module) = context
            .get_modules()
            .find(|m| m.get_name(context).as_str(context) == self.package)
        {
            module
        } else {
            context.create_module().with_name(&self.package).build()
        };

        let used_args: Vec<_> = args.iter().map(|(name, _, ty)| (*name, *ty)).collect();

        let function = module
            .create_function(context)
            .with_name(identifier)
            .with_return_type(return_ty)
            .with_args(&used_args)
            .build();

        if self.lexer.peek().0.map_or(Token::Error, |token| token) != Token::LCurly {
            panic!("Support function declarations!");
        }

        let arg_types: Vec<yair::Type> = function
            .get_args(context)
            .map(|v| v.get_type(context))
            .collect();

        let mut builder = function.create_block(context);

        for arg in arg_types {
            builder = builder.with_arg(arg);
        }

        let alloca_block = builder.build();

        self.push_scope();

        let mut entry_block_builder = alloca_block.create_instructions(context);

        for arg in args.iter().enumerate() {
            let (index, (name, range, ty)) = arg;

            let value = alloca_block.get_arg(entry_block_builder.borrow_context(), index);

            let location = value.get_location(entry_block_builder.borrow_context());
            let stack_alloc = entry_block_builder.stack_alloc(name, *ty, location);

            entry_block_builder.store(stack_alloc, value, location);

            self.add_identifier(name, range.clone(), *ty, stack_alloc)?;
        }

        entry_block_builder.pause_building();

        let return_is_void = return_ty.is_void(context);

        self.expect_symbol(Token::LCurly)?;
        let (entry_block, exit_block) = self.parse_block(function, alloca_block, context)?;

        if return_is_void {
            let location = self.get_current_location(context);
            exit_block.create_instructions(context).ret(location);
        }

        alloca_block
            .create_instructions(context)
            .branch(entry_block, &[], None);

        Ok(function.get_type(context))
    }

    fn parse_identifier(&mut self) -> Result<String, ParseError> {
        self.expect_symbol(Token::Identifier)?;
        Ok(self.get_current_str().to_string())
    }

    fn parse_type(
        &mut self,
        name: Option<&str>,
        context: &mut yair::Context,
    ) -> Result<yair::Type, ParseError> {
        let identifier = self.parse_identifier()?;

        let mut result = match identifier.as_str() {
            "function" => self.parse_function(name.unwrap(), context)?,
            "void" => context.get_void_type(),
            "bool" => context.get_bool_type(),
            "i8" => context.get_int_type(8),
            "i16" => context.get_int_type(16),
            "i32" => context.get_int_type(32),
            "i64" => context.get_int_type(64),
            "u8" => context.get_uint_type(8),
            "u16" => context.get_uint_type(16),
            "u32" => context.get_uint_type(32),
            "u64" => context.get_uint_type(64),
            "f16" => context.get_float_type(16),
            "f32" => context.get_float_type(32),
            "f64" => context.get_float_type(64),
            _ => panic!("Unknown type identifier {}", identifier),
        };

        // If we've got an array, parse that now.
        while self.peek() == Token::LBracket {
            self.expect_symbol(Token::LBracket)?;

            self.expect_symbol(Token::Integer)?;

            let length = self.parse_integer(context)?;

            result = context.get_array_type(result, length);

            self.expect_symbol(Token::RBracket)?;
        }

        Ok(result)
    }

    pub fn parse(&mut self, context: &mut yair::Context) -> Result<(), ParseError> {
        if self.peek() == Token::Package {
            self.expect_symbol(Token::Package)?;

            self.expect_symbol(Token::String)?;

            let str = self.get_current_str();
            self.package = str[1..(str.len() - 1)].to_string();
            // TODO: Check for bad package names?
        }

        let identifier = match self.parse_identifier() {
            Ok(i) => i,
            Err(ParseError::UnexpectedEndOfFile) => return Ok(()),
            Err(e) => return Err(e),
        };

        if Some(Token::Colon) != self.get_next() {
            return Err(ParseError::ExpectedTokenNotFound(
                Token::Colon,
                self.get_current_range(),
            ));
        }

        let _ty = self.parse_type(Some(&identifier), context)?;

        Ok(())
    }

    pub fn display_error(
        &self,
        e: ParseError,
        data: &str,
        context: &mut yair::Context,
        fmt: &mut std::io::Stderr,
    ) -> Result<(), io::Error> {
        let write_range = |fmt: &mut std::io::Stderr, range: Range| -> Result<(), io::Error> {
            let span = self.file.span;
            let span = span.subspan(range.start as u64, range.end as u64);

            let location = self.codemap.look_up_span(span);

            writeln!(
                fmt,
                "{}:{}:{}",
                location.file.name(),
                location.begin.line + 1,
                location.begin.column + 1
            )?;

            let pos = span.low();

            let line = self.file.find_line(pos);

            let str = self.file.source_line(line);

            writeln!(fmt, "  {}", str)?;

            let line_col = self.file.find_line_col(pos);

            writeln!(fmt, "  {}^", " ".repeat(line_col.column))
        };

        match e {
            ParseError::UnexpectedEndOfFile => {
                writeln!(fmt, "error: Unexpected end of file")?;
                write_range(fmt, (data.len() - 1)..data.len())?;
            }
            ParseError::ExpectedTokenNotFound(token, range) => {
                writeln!(fmt, "error: Expected token '{:?}' not found", token)?;
                write_range(fmt, range)?;
            }
            ParseError::InvalidExpression(range) => {
                writeln!(fmt, "error: Invalid expression")?;
                write_range(fmt, range)?;
            }
            ParseError::OperatorsInDifferentPrecedenceGroups(x_range, _, y_range, _) => {
                let span = self.file.span;
                let span = span.subspan(x_range.start as u64, x_range.end as u64);
                let x_str = self.file.source_slice(span);

                let span = self.file.span;
                let span = span.subspan(y_range.start as u64, y_range.end as u64);
                let y_str = self.file.source_slice(span);

                writeln!(
                    fmt,
                    "error: Operators '{}' and '{}' are in different precedence groups",
                    x_str, y_str
                )?;
                write_range(fmt, x_range)?;
                write_range(fmt, y_range)?;
            }
            ParseError::TypesDoNotMatch(x_range, x_ty, y_range, y_ty) => {
                writeln!(
                    fmt,
                    "error: Types '{}' and '{}' do not match",
                    x_ty.get_displayer(context),
                    y_ty.get_displayer(context),
                )?;
                write_range(fmt, x_range)?;
                write_range(fmt, y_range)?;
            }
            ParseError::InvalidNonConcreteConstantsUsed(x_range, y_range) => {
                writeln!(fmt, "error: Invalid non-concrete constant used")?;
                write_range(fmt, x_range)?;
                write_range(fmt, y_range)?;
            }
            ParseError::InvalidNonConcreteConstantUsed(range) => {
                writeln!(fmt, "error: Invalid non-concrete constant used")?;
                write_range(fmt, range)?;
            }
            ParseError::UnknownIdentifier(range) => {
                let span = self.file.span;
                let span = span.subspan(range.start as u64, range.end as u64);
                let str = self.file.source_slice(span);

                writeln!(fmt, "error: Unknown identifier '{}' used", str)?;
                write_range(fmt, range)?;
            }
            ParseError::ComparisonOperatorsAlwaysNeedParenthesis(x_range, _, y_range, _) => {
                let span = self.file.span;
                let span = span.subspan(x_range.start as u64, x_range.end as u64);
                let x_str = self.file.source_slice(span);

                let span = self.file.span;
                let span = span.subspan(y_range.start as u64, y_range.end as u64);
                let y_str = self.file.source_slice(span);

                writeln!(
                    fmt,
                    "error: Comparison operators always need parenthesis in expressions: '{}' and '{}'",
                    x_str, y_str
                )?;

                write_range(fmt, x_range)?;
                write_range(fmt, y_range)?;
            }
            ParseError::UnknownIdentifierUsedInAssignment(range) => {
                let span = self.file.span;
                let span = span.subspan(range.start as u64, range.end as u64);
                let str = self.file.source_slice(span);

                writeln!(
                    fmt,
                    "error: Unknown identifier '{}' used in assignment",
                    str
                )?;
                writeln!(
                    fmt,
                    "       Did you mean to declare the variable `{} : <type> =` instead?",
                    str
                )?;
                write_range(fmt, range)?;
            }
            ParseError::IdentifierShadowsPreviouslyDeclareIdentifier(x_range, y_range) => {
                let span = self.file.span;
                let span = span.subspan(y_range.start as u64, y_range.end as u64);
                let y_str = self.file.source_slice(span);

                writeln!(
                    fmt,
                    "error: Identifier '{}' shadows a previously declared identifier:",
                    y_str
                )?;

                write_range(fmt, y_range)?;
                write_range(fmt, x_range)?;
            }
            ParseError::ElseStatementWithoutIf(range) => {
                writeln!(fmt, "error: Else statement without an if")?;
                write_range(fmt, range)?;
            }
        }

        Ok(())
    }
}

fn main() {
    let yaml = load_yaml!("bootstrap.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let input = matches.value_of("input").unwrap();

    let mut data = String::new();

    if input == "-" {
        io::stdin().read_to_string(&mut data).unwrap();
    } else {
        let mut file = File::open(input).unwrap();
        file.read_to_string(&mut data).unwrap();
    }

    let mut context = yair::Context::new();

    let mut parser = Parser::new(input.to_string(), &data);

    if let Err(e) = parser.parse(&mut context) {
        parser
            .display_error(e, &data, &mut context, &mut std::io::stderr())
            .unwrap();
        std::process::exit(1);
    }

    if let Err(e) = context.verify() {
        eprintln!("Verification error: {}", e);
        std::process::exit(1);
    }

    let emit = matches.value_of("emit").unwrap();

    match emit {
        "yair" => {
            let output = matches.value_of("output").unwrap();

            if output == "-" {
                disassemble(&context, io::stdout().lock())
            } else {
                let file = File::create(output).unwrap();
                disassemble(&context, file)
            }
            .expect("Could not write data");
        }
        _ => {
            todo!()
        }
    }
}
