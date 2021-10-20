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

#[derive(Logos, Debug, PartialEq)]
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

    #[regex("[_a-zA-Z][_a-zA-Z0-9]*")]
    Identifier,

    #[regex("[1-9][0-9]*")]
    Integer,

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

    fn get_location(&mut self, context: &mut yair::Context) -> Option<yair::Location> {
        let span = self.file.span;

        let range = self.lexer.span();

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
        if ty.is_integral(context) {
            self.expect_symbol(Token::Integer)?;

            let str = self.lexer.slice();

            if let Ok(i) = str.parse::<i64>() {
                let bits = ty.get_bits(context) as u8;

                Ok(context.get_int_constant(bits, i))
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

    fn parse_expression(
        &mut self,
        ty: Type,
        context: &mut yair::Context,
        _paused: &mut PausedInstructionBuilder,
    ) -> Result<yair::Value, ParseError> {
        let val = self.parse_constant(ty, context)?;

        Ok(val)
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
                        let paused = builder.pause_building();
                        let location = self.get_location(context);
                        builder = InstructionBuilder::resume_building(context, paused);
                        builder.ret(location);
                    }

                    break;
                }
                Some(Token::Return) => {
                    let mut paused = builder.pause_building();
                    let expr = self.parse_expression(return_ty, context, &mut paused)?;
                    builder = InstructionBuilder::resume_building(context, paused);

                    let paused = builder.pause_building();
                    let location = self.get_location(context);
                    builder = InstructionBuilder::resume_building(context, paused);
                    builder.ret_val(expr, location);

                    // TODO: This is a total bodge to make the borrow checker happy. Maybe consider adding a Default::default() to the builder for these cases?
                    builder = block.create_instructions(context);

                    self.expect_symbol(Token::Semicolon)?;
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
            ParseError::ExpectedTokenNotFound(token, range) => {
                writeln!(fmt, "error: Expected token '{:?}' not found", token)?;
                range
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
