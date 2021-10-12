#[macro_use]
extern crate clap;

extern crate codemap;

use clap::App;
use codemap::{CodeMap, Span};
use std::collections::HashMap;
use std::fmt;
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
}

#[allow(dead_code)]
struct Parser<'a> {
    codemap: CodeMap,
    file: Arc<codemap::File>,
    span: Span,
    context: &'a mut Context,
    functions: HashMap<&'a str, yair::Function>,
    module: String,
}

impl<'a> Parser<'a> {
    pub fn new(name: String, data: String, context: &'a mut Context) -> Parser {
        let mut codemap = CodeMap::new();

        let file = codemap.add_file(name, data);

        let span = file.span;

        Parser {
            codemap,
            file,
            span,
            context,
            functions: HashMap::new(),
            module: "".to_string(),
        }
    }

    fn get_location(&mut self) -> Option<yair::Location> {
        let location = self.codemap.look_up_span(self.span);

        Some(self.context.get_location(
            location.file.name(),
            location.begin.line + 1,
            location.begin.column,
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

    fn parse_function(&mut self, identifier: &str) -> Result<yair::Type, ParseError> {
        self.skip_comments_or_whitespace()?;

        self.parse_char('(')?;

        loop {
            self.skip_comments_or_whitespace()?;

            if self.is_char(')') {
                self.parse_char(')')?;
                break;
            }

            // Actually parse arguments
            todo!();
        }

        self.skip_comments_or_whitespace()?;

        self.parse_char(':')?;

        self.skip_comments_or_whitespace()?;

        let return_ty = self.parse_type(None)?;

        let module = if let Some(module) = self
            .context
            .get_modules()
            .find(|m| m.get_name(self.context).as_str(self.context) == self.module)
        {
            module
        } else {
            self.context.create_module().with_name(&self.module).build()
        };

        let mut function = module
            .create_function(self.context)
            .with_name(identifier)
            .with_return_type(return_ty)
            .build();

        self.skip_comments_or_whitespace()?;

        // TODO: support function declarations!
        self.parse_char('{')?;

        self.skip_comments_or_whitespace()?;

        let args: Vec<yair::Type> = function.get_args(self.context).map(|v| v.get_type(self.context)).collect();

        let mut builder = function.create_block(self.context);

        for arg in args {
            builder = builder.with_arg(arg);
        }

        let block = builder.build();

        let mut builder = block.create_instructions(self.context);
        let mut paused = builder.pause_building();

        loop {
            if self.is_char('}') {
                if return_ty.is_void(self.context) {
                    let location = self.get_location();
                    builder = InstructionBuilder::resume_building(self.context, paused);
                    builder.ret(location);
                }

                self.parse_char('}')?;
                break;
            }
        }

        Ok(function.get_type(self.context))
    }

    fn parse_type(&mut self, name: Option<&str>) -> Result<yair::Type, ParseError> {
        let identifier = self.parse_identifier()?;

        match identifier.as_str() {
            "function" => self.parse_function(name.unwrap()),
            "void" => Ok(self.context.get_void_type()),
            _ => todo!(),
        }
    }

    fn is_char(&self, c: char) -> bool {
        let str = self.file.source_slice(self.span);

        str.starts_with(c)
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

    pub fn parse(&mut self) -> Result<(), ParseError> {
        self.skip_comments_or_whitespace()?;

        let identifier = self.parse_identifier()?;

        self.skip_comments_or_whitespace()?;

        self.parse_char(':')?;

        self.skip_comments_or_whitespace()?;

        let ty = self.parse_type(Some(&identifier))?;

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
            ParseError::IdentifierDidNotStartWithAlphabeticalCharacter(span) => {
                writeln!(
                    fmt,
                    "error: Identifier did not start with an alphabetical character"
                )?;
                span
            }
            _ => todo!(),
        };

        let location = self.codemap.look_up_span(span);

        writeln!(
            fmt,
            "{}:{}:{}",
            location.file.name(),
            location.begin.line + 1,
            location.begin.column
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

    let mut parser = Parser::new(input.to_string(), data, &mut context);

    if let Err(e) = parser.parse() {
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
