#[macro_use]
extern crate clap;
extern crate pest;
#[macro_use]
extern crate pest_derive;
extern crate rmp_serde;
extern crate serde;
extern crate yair;

use clap::App;
use pest::iterators::Pair;
use pest::Parser;
use rmp_serde::Serializer;
use serde::Serialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read};
use yair::*;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct MyParser;

#[derive(Default)]
struct ParseState<'a> {
    pub library: Library,
    pub name_to_mod: HashMap<&'a str, Module>,
    pub name_and_mod_to_var: HashMap<(Module, &'a str), Value>,
    pub name_and_mod_to_func: HashMap<(Module, &'a str), Function>,
}

impl<'a> ParseState<'a> {
    pub fn new() -> Self {
        Default::default()
    }

    fn parse_quoted_ident(&self, pair: Pair<'a, Rule>) -> &'a str {
        let str = pair.as_str();
        &str[1..(str.len() - 1)]
    }

    fn parse_unquoted_ident(&self, pair: Pair<'a, Rule>) -> &'a str {
        pair.as_str()
    }

    fn parse_ident(&self, pair: Pair<'a, Rule>) -> &'a str {
        if pair.as_rule() == Rule::quoted_ident {
            self.parse_quoted_ident(pair)
        } else if pair.as_rule() == Rule::unquoted_ident {
            self.parse_unquoted_ident(pair)
        } else {
            panic!("{:?}", pair)
        }
    }

    fn parse_type(&mut self, pair: Pair<'a, Rule>) -> Type {
        match pair.as_str() {
            "void" => self.library.get_void_ty(),
            "bool" => self.library.get_bool_ty(),
            "i8" => self.library.get_int_ty(8),
            "i16" => self.library.get_int_ty(16),
            "i32" => self.library.get_int_ty(32),
            "i64" => self.library.get_int_ty(64),
            "u8" => self.library.get_uint_ty(8),
            "u16" => self.library.get_uint_ty(16),
            "u32" => self.library.get_uint_ty(32),
            "u64" => self.library.get_uint_ty(64),
            "f16" => self.library.get_float_ty(16),
            "f32" => self.library.get_float_ty(32),
            "f64" => self.library.get_float_ty(64),
            s => {
                if s.starts_with('<') {
                    let mut pairs = pair.into_inner();
                    let ty = self.parse_type(pairs.nth(0).unwrap());
                    let width = pairs.nth(0).unwrap().as_str().parse::<u8>().unwrap();
                    self.library.get_vec_type(ty, width)
                } else if s.starts_with('[') {
                    let mut pairs = pair.into_inner();
                    let ty = self.parse_type(pairs.nth(0).unwrap());
                    let width = pairs.nth(0).unwrap().as_str().parse::<usize>().unwrap();
                    self.library.get_array_ty(ty, width)
                } else if s.starts_with('{') {
                    let mut tys = Vec::new();

                    for inner_pair in pair.into_inner() {
                        tys.push(self.parse_type(inner_pair));
                    }

                    self.library.get_struct_ty(&tys)
                } else if s.starts_with('*') {
                    let ty = self.parse_type(pair.into_inner().nth(0).unwrap());
                    self.library.get_ptr_type(ty, Domain::CrossDevice)
                } else {
                    panic!("{:?}", s);
                }
            }
        }
    }

    fn parse_var(&mut self, pair: Pair<'a, Rule>, module: Module, export: bool) {
        let mut pairs = pair.into_inner();

        let name = self.parse_ident(pairs.nth(0).unwrap());

        let ty = self.parse_type(pairs.nth(0).unwrap());

        let domain = Domain::CrossDevice;

        let global = module
            .create_global(&mut self.library)
            .with_name(name)
            .with_type(ty)
            .with_domain(domain)
            .with_export(export)
            .build();

        self.name_and_mod_to_var.insert((module, name), global);
    }

    fn parse_func(&mut self, pair: Pair<'a, Rule>, module: Module, export: bool) {
        let mut pairs = pair.into_inner();

        let name = self.parse_ident(pairs.nth(0).unwrap());
        let ty = self.parse_type(pairs.nth_back(0).unwrap());

        let mut args = Vec::new();

        while let Some(x) = pairs.nth(0) {
            let name = self.parse_ident(x);
            let ty = self.parse_type(pairs.nth_back(0).unwrap());
            args.push((name, ty));
        }

        let mut builder = module
            .create_function(&mut self.library)
            .with_name(name)
            .with_return_type(ty)
            .with_export(export);

        for (name, ty) in args {
            builder = builder.with_argument(name, ty);
        }

        let function = builder.build();

        self.name_and_mod_to_func.insert((module, name), function);
    }

    fn parse_mod(&mut self, pair: Pair<'a, Rule>) {
        let pairs = pair.into_inner();

        let name = self.parse_ident(pairs.peek().unwrap());

        let module = self.library.create_module().with_name(name).build();

        self.name_to_mod.insert(name, module);

        let mut is_next_export = false;

        for inner_pair in pairs.skip(1) {
            if inner_pair.as_rule() == Rule::export {
                is_next_export = true;
            } else if inner_pair.as_rule() == Rule::var {
                self.parse_var(inner_pair, module, is_next_export);
                is_next_export = false;
            } else if inner_pair.as_rule() == Rule::func {
                self.parse_func(inner_pair, module, is_next_export);
                is_next_export = false;
            } else {
                panic!("{:?}", inner_pair);
            }
        }
    }

    pub fn parse(&mut self, str: &'a str) {
        let parse = MyParser::parse(Rule::main, &str);

        let pairs = parse.unwrap_or_else(|e| {
            println!("{}", e);
            std::process::exit(1);
        });

        for pair in pairs {
            if pair.as_rule() == Rule::EOI {
                break;
            }

            assert_eq!(pair.as_rule(), Rule::module);

            self.parse_mod(pair);
        }
    }
}

fn main() {
    let yaml = load_yaml!("clap.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let input = matches.value_of("input").unwrap();

    let mut data = String::new();

    if input == "-" {
        io::stdin().read_to_string(&mut data).unwrap();
    } else {
        let mut file = File::open(input).unwrap();
        file.read_to_string(&mut data).unwrap();
    }

    let mut parse_state = ParseState::new();
    parse_state.parse(&data);

    let output = matches.value_of("output").unwrap();

    if output == "-" {
        let mut serializer = Serializer::new(io::stdout());
        parse_state.library.serialize(&mut serializer).unwrap();
    } else {
        let file = File::create(output).unwrap();
        let mut serializer = Serializer::new(file);
        parse_state.library.serialize(&mut serializer).unwrap();
    }
}
