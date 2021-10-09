#[macro_use]
extern crate clap;

extern crate codemap;

use clap::App;
use codemap::{CodeMap, Span};
use std::fs::File;
use std::io::{self, Read};
use std::sync::Arc;
use yair::io::*;
use yair::*;

#[derive(Debug)]
enum ParseError {
    UnclosedComment(Span),
}

#[allow(dead_code)]
struct Parser<'a> {
    codemap: CodeMap,
    file: Arc<codemap::File>,
    span: Span,
    context: &'a mut Context,
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
        }
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

                    let trimmed = str.trim_start_matches(|x| x != '\r' || x != '\n');

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

    pub fn parse(&mut self) -> Result<(), ParseError> {
        self.skip_comments_or_whitespace()?;

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

    parser.parse().unwrap();

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
