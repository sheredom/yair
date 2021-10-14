#[macro_use]
extern crate clap;

extern crate codemap;

use clap::App;
use codemap::{CodeMap, Span};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::io::{self, Read};
use std::sync::Arc;
use yair::io::*;
use yair::*;

enum ParseError {
    UnclosedComment(Span),
    IdentifierDidNotStartWithAlphabeticalCharacter(Span),
    ExpectedCharacterNotFound(Span, char),
    ExpectedStringNotFound(Span, String),
    ExpectedSymbolNotFound(Span, String),
}

#[allow(dead_code)]
struct Parser<'a> {
    codemap: CodeMap,
    file: Arc<codemap::File>,
    span: Span,
    functions: HashMap<&'a str, yair::Function>,
    module: String,
}

impl<'a> Parser<'a> {
    pub fn new(name: String, data: String) -> Parser<'a> {
        let mut codemap = CodeMap::new();

        let file = codemap.add_file(name, data);

        let span = file.span;

        Parser {
            codemap,
            file,
            span,
            functions: HashMap::new(),
            module: "".to_string(),
        }
    }

    fn get_location(&mut self, context: &mut yair::Context) -> Option<yair::Location> {
        let location = self.codemap.look_up_span(self.span);

        Some(context.get_location(
            location.file.name(),
            location.begin.line + 1,
            location.begin.column + 1,
        ))
    }

    fn skip_comments_or_whitespace(&mut self) -> Result<(), ParseError> {
        let mut changed_something = false;

        loop {
            {
                let str = self.file.source_slice(self.span);

                let trimmed = str.trim_start();

                let trimmed_count = str.len() - trimmed.len();

                if trimmed_count > 0 {
                    changed_something = true;

                    self.span = self.span.subspan(trimmed_count as u64, self.span.len());
                }
            }

            {
                let str = self.file.source_slice(self.span);

                if str.starts_with("//") {
                    changed_something = true;

                    let trimmed = str.trim_start_matches(|x| x != '\r' && x != '\n');

                    let trimmed_count = str.len() - trimmed.len();

                    self.span = self.span.subspan(trimmed_count as u64, self.span.len());
                }
            }

            {
                let str = self.file.source_slice(self.span);

                if let Some(str) = str.strip_prefix("/*") {
                    changed_something = true;

                    let end = str.find("*/");

                    if end.is_none() {
                        return Err(ParseError::UnclosedComment(self.span));
                    }

                    // +2 just to re-add the '/*', and another +2 for the closing '*/'.
                    let trimmed_count = end.unwrap() + 2 + 2;

                    self.span = self.span.subspan(trimmed_count as u64, self.span.len());
                }
            }

            if changed_something {
                changed_something = false;

                continue;
            }

            break;
        }

        Ok(())
    }

    fn parse_identifier(&mut self) -> Result<String, ParseError> {
        let str = self.file.source_slice(self.span);

        if !str.starts_with(char::is_alphabetic) {
            return Err(ParseError::IdentifierDidNotStartWithAlphabeticalCharacter(
                self.span.subspan(0, 1),
            ));
        }

        let trimmed = str.trim_start_matches(char::is_alphanumeric);

        let trimmed_count = (str.len() - trimmed.len()) as u64;

        let identifier = self.file.source_slice(self.span.subspan(0, trimmed_count));

        self.span = self.span.subspan(trimmed_count, self.span.len());

        Ok(identifier.to_string())
    }

    fn is_str(&self, s: &str) -> bool {
        let str = self.file.source_slice(self.span);

        str.starts_with(s)
    }

    #[allow(dead_code)]
    fn parse_str(&mut self, s: &str) -> Result<(), ParseError> {
        let str = self.file.source_slice(self.span);

        if !str.starts_with(s) {
            return Err(ParseError::ExpectedStringNotFound(
                self.span.subspan(0, s.len() as u64),
                s.to_string(),
            ));
        }

        self.span = self.span.subspan(s.len() as u64, self.span.len());

        Ok(())
    }

    fn is_constant(&self, ty: Type, context: &mut yair::Context) -> bool {
        if ty.is_integral(context) {
            if self.is_str("0b") {
                panic!("Handling binary constants!");
            } else if self.is_str("0x") {
                panic!("Handling hex constants!");
            } else {
                let str = self.file.source_slice(self.span);

                str.len() != str.trim_start_matches(char::is_numeric).len()
            }
        } else {
            todo!();
        }
    }

    fn parse_constant(
        &mut self,
        ty: Type,
        context: &mut yair::Context,
    ) -> Result<yair::Value, ParseError> {
        if ty.is_integral(context) {
            if self.is_str("0b") {
                panic!("Handling binary constants!");
            } else if self.is_str("0x") {
                panic!("Handling hex constants!");
            } else {
                let str = self.file.source_slice(self.span);

                let len = str.len() - str.trim_start_matches(char::is_numeric).len();

                self.span = self.span.subspan(len as u64, self.span.len());

                if let Ok(i) = str[0..len].parse::<i64>() {
                    let bits = ty.get_bits(context) as u8;

                    Ok(context.get_int_constant(bits, i))
                } else {
                    // Make an error for this
                    todo!();
                }
            }
        } else {
            todo!();
        }
    }

    fn parse_function(
        &mut self,
        identifier: &str,
        context: &mut yair::Context,
    ) -> Result<yair::Type, ParseError> {
        self.skip_comments_or_whitespace()?;

        self.parse_char('(')?;

        loop {
            self.skip_comments_or_whitespace()?;

            if self.is_char(')') {
                self.parse_char(')')?;
                break;
            }

            // Actually parse arguments
            todo!()
        }

        self.skip_comments_or_whitespace()?;

        self.parse_char(':')?;

        self.skip_comments_or_whitespace()?;

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

        self.skip_comments_or_whitespace()?;

        // TODO: support function declarations!
        self.parse_char('{')?;

        self.skip_comments_or_whitespace()?;

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
            self.skip_comments_or_whitespace()?;

            if self.is_char('}') {
                if return_is_void {
                    let paused = builder.pause_building();
                    let location = self.get_location(context);
                    builder = InstructionBuilder::resume_building(context, paused);
                    builder.ret(location);
                }

                self.parse_char('}')?;
                break;
            }

            if self.is_symbol("return") {
                self.parse_symbol("return")?;

                self.skip_comments_or_whitespace()?;

                let paused = builder.pause_building();
                let is_constant = self.is_constant(return_ty, context);
                builder = InstructionBuilder::resume_building(context, paused);

                if is_constant {
                    let paused = builder.pause_building();
                    let cnst = self.parse_constant(return_ty, context)?;
                    let location = self.get_location(context);
                    builder = InstructionBuilder::resume_building(context, paused);
                    builder.ret_val(cnst, location);

                    // TODO: This is a total bodge to make the borrow checker happy. Maybe consider adding a Default::default() to the builder for these cases?
                    builder = block.create_instructions(context);
                }

                self.parse_char(';')?;
            } else {
                let str = self.file.source_slice(self.span);
                panic!("TODO {}", str);
            }
        }

        Ok(function.get_type(context))
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

    fn is_char(&self, c: char) -> bool {
        let str = self.file.source_slice(self.span);

        str.starts_with(c)
    }

    fn is_symbol(&self, s: &str) -> bool {
        let str = self.file.source_slice(self.span);

        // If we find the string - it must be followed by whitespace!
        if let Some(s) = str.strip_prefix(s) {
            s.starts_with(char::is_whitespace)
        } else {
            false
        }
    }

    fn parse_char(&mut self, c: char) -> Result<(), ParseError> {
        if !self.is_char(c) {
            return Err(ParseError::ExpectedCharacterNotFound(
                self.span.subspan(0, 1),
                c,
            ));
        }

        self.span = self.span.subspan(1, self.span.len());

        Ok(())
    }

    fn parse_symbol(&mut self, s: &str) -> Result<(), ParseError> {
        if !self.is_symbol(s) {
            return Err(ParseError::ExpectedSymbolNotFound(
                self.span.subspan(0, s.len() as u64),
                s.to_string(),
            ));
        }

        self.span = self.span.subspan(s.len() as u64, self.span.len());

        Ok(())
    }

    pub fn parse(&mut self, context: &mut yair::Context) -> Result<(), ParseError> {
        self.skip_comments_or_whitespace()?;

        let identifier = self.parse_identifier()?;

        self.skip_comments_or_whitespace()?;

        self.parse_char(':')?;

        self.skip_comments_or_whitespace()?;

        let _ty = self.parse_type(Some(&identifier), context)?;

        self.skip_comments_or_whitespace()?;

        Ok(())
    }

    pub fn display_error(&self, e: ParseError, fmt: &mut std::io::Stderr) -> Result<(), io::Error> {
        let span = match e {
            ParseError::UnclosedComment(span) => {
                writeln!(fmt, "error: Multi-line comment `/* */` was not closed")?;
                span
            }
            ParseError::ExpectedCharacterNotFound(span, char) => {
                writeln!(fmt, "error: Expected character '{}' not found", char)?;
                span
            }
            ParseError::ExpectedStringNotFound(span, str) => {
                writeln!(fmt, "error: Expected string '{}' not found", str)?;
                span
            }
            ParseError::ExpectedSymbolNotFound(span, symbol) => {
                writeln!(fmt, "error: Expected symbol '{}' not found", symbol)?;
                span
            }
            ParseError::IdentifierDidNotStartWithAlphabeticalCharacter(span) => {
                writeln!(
                    fmt,
                    "error: Identifier did not start with an alphabetical character"
                )?;
                span
            }
        };

        let location = self.codemap.look_up_span(span);

        writeln!(
            fmt,
            "{}:{}:{}",
            location.file.name(),
            location.begin.line + 1,
            location.begin.column + 1
        )?;

        let str = self.file.source_slice(self.span);

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

    let mut parser = Parser::new(input.to_string(), data);

    if let Err(e) = parser.parse(&mut context) {
        parser.display_error(e, &mut std::io::stderr()).unwrap();
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
