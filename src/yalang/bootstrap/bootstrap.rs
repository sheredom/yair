#[macro_use]
extern crate clap;
extern crate codemap;
extern crate logos;

use clap::App;
use codemap::*;
use logos::Logos;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::io::{self, Read};
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
        Token::And => (PrecedenceGroup::Bitwise, 0),
        Token::Or => (PrecedenceGroup::Bitwise, 1),
        Token::Xor => (PrecedenceGroup::Bitwise, 2),
        Token::As => (PrecedenceGroup::Cast, 0),
        Token::LParen | Token::RParen => (PrecedenceGroup::Parenthesis, u8::MAX),
        Token::Equality => (PrecedenceGroup::Comparison, 0),
        _ => todo!(),
    }
}

fn is_unary(x: Token) -> bool {
    matches!(x, Token::As)
    /*match x {
        Token::As => true,
        _ => false,
    }*/
}

#[derive(Logos, Copy, Clone, Debug, PartialEq)]
enum Token {
    #[token("return")]
    Return,

    #[token(":")]
    Colon,

    #[token(";")]
    Semicolon,

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

    #[token("==")]
    Equality,

    #[regex("[_a-zA-Z][_a-zA-Z0-9]*")]
    Identifier,

    #[regex("[1-9][0-9]*", priority = 2)]
    Integer,

    #[regex("[+-]?([0-9]+([.][0-9]*)?([eE][+-]?[0-9]+)?|[.][0-9]+([eE][+-]?[0-9]+)?)")]
    Float,

    // Logos requires one token variant to handle errors,
    // it can be named anything you wish.
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
    InvalidNonConcreteConstantsUsed(Range, Range),
    UnknownIdentifier(Range),
    ComparisonOperatorsAlwaysNeedParenthesis(Range, Token, Range, Token),
}

#[allow(dead_code)]
struct Parser<'a> {
    codemap: CodeMap,
    file: Arc<codemap::File>,
    functions: HashMap<&'a str, yair::Function>,
    module: String,
    lexer: logos::Lexer<'a, Token>,
    identifiers: HashMap<&'a str, yair::Value>,
    scoped_identifiers: Vec<Vec<&'a str>>,
}

impl<'a> Parser<'a> {
    pub fn new(name: String, data: &'a str) -> Parser<'a> {
        let mut codemap = CodeMap::new();

        let file = codemap.add_file(name, data.to_string());

        Parser {
            codemap,
            file,
            functions: HashMap::new(),
            module: "".to_string(),
            lexer: Token::lexer(&data),
            identifiers: HashMap::new(),
            scoped_identifiers: Vec::new(),
        }
    }

    fn get_current_location(&mut self, context: &mut yair::Context) -> Option<yair::Location> {
        let range = self.lexer.span();

        self.get_location(range, context)
    }

    fn get_location(
        &mut self,
        range: Range,
        context: &mut yair::Context,
    ) -> Option<yair::Location> {
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
        if let Some(next) = self.lexer.next() {
            if next == token {
                Ok(())
            } else {
                Err(ParseError::ExpectedTokenNotFound(token, self.lexer.span()))
            }
        } else {
            Err(ParseError::UnexpectedEndOfFile)
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

        if is_unary(op.1) {
            todo!();
        } else {
            let x = operand_stack.pop().unwrap();

            let (x_value, y_value) = match x.kind {
                OperandKind::Concrete(x_value) => match y.kind {
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
                },
                OperandKind::Float(x_float) => match y.kind {
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
                OperandKind::Integer(x_int) => match y.kind {
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
            };

            let location = self.get_location(op.0.clone(), builder.borrow_context());

            let expr = match op.1 {
                Token::Add => builder.add(x_value, y_value, location),
                Token::Sub => builder.sub(x_value, y_value, location),
                Token::Mul => builder.mul(x_value, y_value, location),
                Token::Div => builder.div(x_value, y_value, location),
                Token::Mod => builder.rem(x_value, y_value, location),
                Token::And => builder.and(x_value, y_value, location),
                Token::Or => builder.or(x_value, y_value, location),
                Token::Xor => builder.xor(x_value, y_value, location),
                Token::Equality => builder.cmp_eq(x_value, y_value, location),
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
        if let Ok(i) = self.lexer.slice().parse::<u64>() {
            Ok(i)
        } else {
            todo!()
        }
    }

    fn parse_float(&mut self, _: &mut Context) -> Result<f64, ParseError> {
        if let Ok(f) = self.lexer.slice().parse::<f64>() {
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

        loop {
            match self.lexer.next() {
                Some(Token::Integer) => operand_stack.push(Operand {
                    range: self.lexer.span(),
                    kind: OperandKind::Integer(self.parse_integer(builder.borrow_context())?),
                }),
                Some(Token::Float) => operand_stack.push(Operand {
                    range: self.lexer.span(),
                    kind: OperandKind::Float(self.parse_float(builder.borrow_context())?),
                }),
                Some(Token::LParen) => operator_stack.push((self.lexer.span(), Token::LParen)),
                Some(Token::RParen) => self.apply_if_lower_precedence(
                    (self.lexer.span(), Token::RParen),
                    &mut operand_stack,
                    &mut operator_stack,
                    builder,
                )?,
                Some(Token::As) => {
                    let range = self.lexer.span();

                    if !operator_stack.is_empty() {
                        self.check_precedence(
                            (range.clone(), Token::As),
                            operator_stack.last().unwrap().clone(),
                        )?;
                    }

                    let ty = self.parse_type(None, builder.borrow_context())?;

                    let expr = if let Some(x) = operand_stack.pop() {
                        match x.kind {
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
                            | Token::And
                            | Token::Or
                            | Token::Xor
                            | Token::Equality
                    ) =>
                {
                    self.apply_if_lower_precedence_and_push_operator(
                        (self.lexer.span(), x),
                        &mut operand_stack,
                        &mut &mut operator_stack,
                        builder,
                    )?
                }
                Some(Token::Semicolon) => break,
                Some(Token::Identifier) => {
                    let identifier = self.lexer.slice();

                    if let Some(identifier) = self.identifiers.get(identifier) {
                        operand_stack.push(Operand {
                            range: self.lexer.span(),
                            kind: OperandKind::Concrete(*identifier),
                        });
                    } else {
                        return Err(ParseError::UnknownIdentifier(self.lexer.span()));
                    }
                }
                Some(_) => todo!(),
                None => todo!(),
            }
        }

        // Handle the case where an expression is malformed like `foo=;`
        if operand_stack.is_empty() {
            return Err(ParseError::InvalidExpression(self.lexer.span()));
        }

        while operand_stack.len() != 1 {
            self.apply(&mut operand_stack, &mut operator_stack, builder)?;
        }

        let operand = operand_stack.pop().unwrap();

        let expr = match operand.kind {
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

    fn pop_scope(&mut self) {
        for identifier in self.scoped_identifiers.last().unwrap().iter() {
            self.identifiers.remove(identifier).unwrap();
        }

        self.scoped_identifiers.pop();
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
            match self.lexer.next() {
                Some(Token::RParen) => break,
                Some(Token::Identifier) => {
                    let name = self.lexer.slice();

                    if Some(Token::Colon) != self.lexer.next() {
                        return Err(ParseError::ExpectedTokenNotFound(
                            Token::Colon,
                            self.lexer.span(),
                        ));
                    }

                    // TODO: We should check that we aren't parsing a function definition again here!
                    let ty = self.parse_type(Some(&identifier), context)?;

                    args.push((name, ty));

                    parsed_one_arg = true;
                }
                Some(Token::Comma) => {
                    if !parsed_one_arg {
                        return Err(ParseError::ExpectedTokenNotFound(
                            Token::Comma,
                            self.lexer.span(),
                        ));
                    }
                }
                Some(_) => return Err(ParseError::InvalidExpression(self.lexer.span())),
                None => return Err(ParseError::UnexpectedEndOfFile),
            }
        }

        self.expect_symbol(Token::Colon)?;

        let return_ty = self.parse_type(None, context)?;

        let module = if let Some(module) = context
            .get_modules()
            .find(|m| m.get_name(context).as_str(context) == self.module)
        {
            module
        } else {
            context.create_module().with_name(&self.module).build()
        };

        let function = module
            .create_function(context)
            .with_name(identifier)
            .with_return_type(return_ty)
            .with_args(&args)
            .build();

        self.scoped_identifiers.push(Vec::new());

        // TODO: support function declarations!
        self.expect_symbol(Token::LCurly)?;

        let arg_types: Vec<yair::Type> = function
            .get_args(context)
            .map(|v| v.get_type(context))
            .collect();

        let mut builder = function.create_block(context);

        for arg in arg_types {
            builder = builder.with_arg(arg);
        }

        let block = builder.build();

        for arg in args.iter().zip(block.get_args(context)) {
            self.identifiers.insert(arg.0 .0, arg.1);
            self.scoped_identifiers.last_mut().unwrap().push(arg.0 .0);
        }

        let return_is_void = return_ty.is_void(context);

        let mut builder = block.create_instructions(context);

        loop {
            match self.lexer.next() {
                Some(Token::RCurly) => {
                    if return_is_void {
                        let location = self.get_current_location(builder.borrow_context());
                        builder.ret(location);
                    }

                    break;
                }
                Some(Token::Return) => {
                    let location = self.get_current_location(builder.borrow_context());

                    let expr = self.parse_expression(return_ty, &mut builder)?;

                    builder.ret_val(expr, location);

                    // TODO: This is a total bodge to make the borrow checker happy. Maybe consider adding a Default::default() to the builder for these cases?
                    builder = block.create_instructions(context);
                }
                Some(_) =>
                /* Handle other statements */
                {
                    todo!()
                }
                None => return Err(ParseError::UnexpectedEndOfFile),
            }
        }

        self.pop_scope();

        Ok(function.get_type(context))
    }

    fn parse_identifier(&mut self) -> Result<String, ParseError> {
        self.expect_symbol(Token::Identifier)?;
        Ok(self.lexer.slice().to_string())
    }

    fn parse_type(
        &mut self,
        name: Option<&str>,
        context: &mut yair::Context,
    ) -> Result<yair::Type, ParseError> {
        let identifier = self.parse_identifier()?;

        match identifier.as_str() {
            "function" => self.parse_function(name.unwrap(), context),
            "void" => Ok(context.get_void_type()),
            "bool" => Ok(context.get_bool_type()),
            "i8" => Ok(context.get_int_type(8)),
            "i16" => Ok(context.get_int_type(16)),
            "i32" => Ok(context.get_int_type(32)),
            "i64" => Ok(context.get_int_type(64)),
            "u8" => Ok(context.get_uint_type(8)),
            "u16" => Ok(context.get_uint_type(16)),
            "u32" => Ok(context.get_uint_type(32)),
            "u64" => Ok(context.get_uint_type(64)),
            "f16" => Ok(context.get_float_type(16)),
            "f32" => Ok(context.get_float_type(32)),
            "f64" => Ok(context.get_float_type(64)),
            _ => todo!(),
        }
    }

    pub fn parse(&mut self, context: &mut yair::Context) -> Result<(), ParseError> {
        let identifier = match self.parse_identifier() {
            Ok(i) => i,
            Err(ParseError::UnexpectedEndOfFile) => return Ok(()),
            Err(e) => return Err(e),
        };

        if Some(Token::Colon) != self.lexer.next() {
            return Err(ParseError::ExpectedTokenNotFound(
                Token::Colon,
                self.lexer.span(),
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
        let range = match e {
            ParseError::UnexpectedEndOfFile => {
                writeln!(fmt, "error: Unexpected end of file")?;
                (data.len() - 1)..data.len()
            }
            ParseError::ExpectedTokenNotFound(token, range) => {
                writeln!(fmt, "error: Expected token '{:?}' not found", token)?;
                range
            }
            ParseError::InvalidExpression(range) => {
                writeln!(fmt, "error: Invalid expression")?;
                range
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

                let span = self.file.span;

                let span = span.subspan(x_range.start as u64, x_range.end as u64);

                let location = self.codemap.look_up_span(span);

                writeln!(
                    fmt,
                    "{}:{}:{}",
                    location.file.name(),
                    location.begin.line + 1,
                    location.begin.column + 1
                )?;

                let span = self.file.span;

                let span = span.subspan(x_range.start as u64, x_range.end as u64);

                let pos = span.low();

                let line = self.file.find_line(pos);

                let str = self.file.source_line(line);

                writeln!(fmt, "  {}", str)?;

                let line_col = self.file.find_line_col(pos);

                writeln!(fmt, "  {}^", " ".repeat(line_col.column))?;

                y_range
            }
            ParseError::TypesDoNotMatch(x_range, x_ty, y_range, y_ty) => {
                writeln!(
                    fmt,
                    "error: Types '{}' and '{}' do not match",
                    x_ty.get_displayer(context),
                    y_ty.get_displayer(context),
                )?;

                let span = self.file.span;

                let span = span.subspan(x_range.start as u64, x_range.end as u64);

                let location = self.codemap.look_up_span(span);

                writeln!(
                    fmt,
                    "{}:{}:{}",
                    location.file.name(),
                    location.begin.line + 1,
                    location.begin.column + 1
                )?;

                let span = self.file.span;

                let span = span.subspan(x_range.start as u64, x_range.end as u64);

                let pos = span.low();

                let line = self.file.find_line(pos);

                let str = self.file.source_line(line);

                writeln!(fmt, "  {}", str)?;

                let line_col = self.file.find_line_col(pos);

                writeln!(fmt, "  {}^", " ".repeat(line_col.column))?;

                y_range
            }
            ParseError::InvalidNonConcreteConstantsUsed(x_range, y_range) => {
                writeln!(fmt, "error: Invalid non-concrete constant used")?;

                let span = self.file.span;

                let span = span.subspan(x_range.start as u64, x_range.end as u64);

                let location = self.codemap.look_up_span(span);

                writeln!(
                    fmt,
                    "{}:{}:{}",
                    location.file.name(),
                    location.begin.line + 1,
                    location.begin.column + 1
                )?;

                let span = self.file.span;

                let span = span.subspan(x_range.start as u64, x_range.end as u64);

                let pos = span.low();

                let line = self.file.find_line(pos);

                let str = self.file.source_line(line);

                writeln!(fmt, "  {}", str)?;

                let line_col = self.file.find_line_col(pos);

                writeln!(fmt, "  {}^", " ".repeat(line_col.column))?;

                y_range
            }
            ParseError::UnknownIdentifier(range) => {
                let span = self.file.span;
                let span = span.subspan(range.start as u64, range.end as u64);
                let str = self.file.source_slice(span);

                writeln!(fmt, "error: Unknown identifier '{}' used", str)?;

                range
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

                let span = self.file.span;

                let span = span.subspan(x_range.start as u64, x_range.end as u64);

                let location = self.codemap.look_up_span(span);

                writeln!(
                    fmt,
                    "{}:{}:{}",
                    location.file.name(),
                    location.begin.line + 1,
                    location.begin.column + 1
                )?;

                let span = self.file.span;

                let span = span.subspan(x_range.start as u64, x_range.end as u64);

                let pos = span.low();

                let line = self.file.find_line(pos);

                let str = self.file.source_line(line);

                writeln!(fmt, "  {}", str)?;

                let line_col = self.file.find_line_col(pos);

                writeln!(fmt, "  {}^", " ".repeat(line_col.column))?;

                y_range
            }
        };

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

        writeln!(fmt, "  {}^", " ".repeat(line_col.column))?;

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
