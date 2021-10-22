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
}

fn get_precedence(x: Token) -> (PrecedenceGroup, u8) {
    match x {
        Token::Mul | Token::Div | Token::Mod => (PrecedenceGroup::Arithmetic, 0),
        Token::Add | Token::Sub => (PrecedenceGroup::Arithmetic, 1),
        _ => todo!(),
    }
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

    #[regex("[_a-zA-Z][_a-zA-Z0-9]*")]
    Identifier,

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
}

#[allow(dead_code)]
struct Parser<'a> {
    codemap: CodeMap,
    file: Arc<codemap::File>,
    functions: HashMap<&'a str, yair::Function>,
    module: String,
    lexer: logos::Lexer<'a, Token>,
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

    fn parse_constant(
        &mut self,
        ty: Type,
        context: &mut yair::Context,
    ) -> Result<yair::Value, ParseError> {
        if ty.is_int(context) {
            let str = self.lexer.slice();

            if let Ok(i) = str.parse::<i64>() {
                let bits = ty.get_bits(context) as u8;

                Ok(context.get_int_constant(bits, i))
            } else {
                // Make an error for this
                todo!();
            }
        } else if ty.is_uint(context) {
            let str = self.lexer.slice();

            if let Ok(i) = str.parse::<u64>() {
                let bits = ty.get_bits(context) as u8;

                Ok(context.get_uint_constant(bits, i))
            } else {
                // Make an error for this
                todo!();
            }
        } else if ty.is_float(context) {
            let str = self.lexer.slice();

            if let Ok(i) = str.parse::<f64>() {
                let bits = ty.get_bits(context) as u8;

                Ok(context.get_float_constant(bits, i))
            } else {
                // Make an error for this
                todo!();
            }
        } else {
            todo!();
        }
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
        operand_stack: &mut Vec<(Range, yair::Value)>,
        operator_stack: &mut Vec<(Range, Token)>,
        builder: &mut InstructionBuilder,
    ) -> Result<(), ParseError> {
        let y = operand_stack.pop().unwrap();
        let x = operand_stack.pop().unwrap();

        let op = if let Some(op) = operator_stack.pop() {
            op
        } else {
            // No operator between operands `42 13`
            return Err(ParseError::InvalidExpression(y.0));
        };

        let location = self.get_location(op.0.clone(), builder.borrow_context());

        let expr = match op.1 {
            Token::Add => builder.add(x.1, y.1, location),
            Token::Sub => builder.sub(x.1, y.1, location),
            Token::Mul => builder.mul(x.1, y.1, location),
            Token::Div => builder.div(x.1, y.1, location),
            Token::Mod => builder.rem(x.1, y.1, location),
            _ => todo!(),
        };

        operand_stack.push((op.0, expr));

        Ok(())
    }

    fn apply_if_lower_precedence_and_push_operator(
        &mut self,
        x: (Range, Token),
        operand_stack: &mut Vec<(Range, yair::Value)>,
        operator_stack: &mut Vec<(Range, Token)>,
        builder: &mut InstructionBuilder,
    ) -> Result<(), ParseError> {
        let x_precedence = get_precedence(x.1);

        while !operator_stack.is_empty() {
            let y = operator_stack.last().unwrap();

            let y_precedence = get_precedence(y.1);

            if x_precedence.0 != y_precedence.0 {
                return Err(ParseError::OperatorsInDifferentPrecedenceGroups(
                    y.0.clone(),
                    y.1,
                    x.0.clone(),
                    x.1,
                ));
            }

            if x_precedence.1 < y_precedence.1 {
                break;
            }

            self.apply(operand_stack, operator_stack, builder)?;
        }

        operator_stack.push(x);

        Ok(())
    }

    fn parse_expression(
        &mut self,
        ty: Type,
        builder: &mut InstructionBuilder,
    ) -> Result<yair::Value, ParseError> {
        let mut operand_stack = Vec::new();
        let mut operator_stack = Vec::new();

        loop {
            match self.lexer.next() {
                Some(Token::Float) => operand_stack.push((
                    self.lexer.span(),
                    self.parse_constant(ty, builder.borrow_context())?,
                )),
                Some(Token::Add) => self.apply_if_lower_precedence_and_push_operator(
                    (self.lexer.span(), Token::Add),
                    &mut operand_stack,
                    &mut &mut operator_stack,
                    builder,
                )?,
                Some(Token::Sub) => self.apply_if_lower_precedence_and_push_operator(
                    (self.lexer.span(), Token::Sub),
                    &mut operand_stack,
                    &mut &mut operator_stack,
                    builder,
                )?,
                Some(Token::Mul) => self.apply_if_lower_precedence_and_push_operator(
                    (self.lexer.span(), Token::Mul),
                    &mut operand_stack,
                    &mut &mut operator_stack,
                    builder,
                )?,
                Some(Token::Div) => self.apply_if_lower_precedence_and_push_operator(
                    (self.lexer.span(), Token::Div),
                    &mut operand_stack,
                    &mut &mut operator_stack,
                    builder,
                )?,
                Some(Token::Mod) => self.apply_if_lower_precedence_and_push_operator(
                    (self.lexer.span(), Token::Mod),
                    &mut operand_stack,
                    &mut &mut operator_stack,
                    builder,
                )?,
                Some(Token::Semicolon) => break,
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

        Ok(operand_stack.pop().unwrap().1)
    }

    fn parse_function(
        &mut self,
        identifier: &str,
        context: &mut yair::Context,
    ) -> Result<yair::Type, ParseError> {
        self.expect_symbol(Token::LParen)?;

        loop {
            match self.lexer.next() {
                Some(Token::RParen) => break,
                Some(_) =>
                /* Actually parse arguments */
                {
                    todo!()
                }
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
            .build();

        // TODO: support function declarations!
        self.expect_symbol(Token::LCurly)?;

        let args: Vec<yair::Type> = function
            .get_args(context)
            .map(|v| v.get_type(context))
            .collect();

        let mut builder = function.create_block(context);

        for arg in args {
            builder = builder.with_arg(arg);
        }

        let block = builder.build();

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
        fmt: &mut std::io::Stderr,
    ) -> Result<(), io::Error> {
        let range = match e {
            ParseError::UnexpectedEndOfFile => {
                writeln!(fmt, "error: Unexpected end of file")?;
                (data.len() - 1)..data.len()
            }
            ParseError::ExpectedTokenNotFound(_, range) => {
                let span = self.file.span;

                let span = span.subspan(range.start as u64, range.end as u64);

                let str = self.file.source_slice(span);

                writeln!(fmt, "error: Expected token '{}' not found", str)?;
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

                writeln!(fmt, "and:")?;

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

        let str = self.file.source_slice(span);

        writeln!(fmt, "{}", str)?;

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
            .display_error(e, &data, &mut std::io::stderr())
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
