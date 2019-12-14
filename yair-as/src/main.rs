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
use std::fs::File;
use std::io::{self, Read};
use yair::Module;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct MyParser;

use yair::*;

fn handle_quoted_ident(pair: Pair<'_, Rule>) -> &str {
    let str = pair.as_str();
    &str[1..(str.len() - 1)]
}

fn handle_unquoted_ident(pair: Pair<'_, Rule>) -> &str {
    pair.as_str()
}

fn handle_ident(pair: Pair<'_, Rule>) -> &str {
    if pair.as_rule() == Rule::quoted_ident {
        handle_quoted_ident(pair)
    } else if pair.as_rule() == Rule::unquoted_ident {
        handle_unquoted_ident(pair)
    } else {
        panic!("{:?}", pair)
    }
}

fn handle_module(pair: Pair<'_, Rule>, library: &mut Library) -> Module {
    let pairs = pair.into_inner();

    let name = handle_ident(pairs.peek().unwrap());

    for inner_pair in pairs.skip(1) {
        panic!("{:?}", inner_pair);
    }

    library.create_module().with_name(name).build()
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

    let parse = MyParser::parse(Rule::main, &data);

    let pairs = parse.unwrap_or_else(|e| {
        println!("{}", e);
        std::process::exit(1);
    });

    let mut library = Library::new();

    for pair in pairs {
        if pair.as_rule() == Rule::EOI {
            break;
        }

        assert_eq!(pair.as_rule(), Rule::module);

        handle_module(pair, &mut library);
    }

    let output = matches.value_of("output").unwrap();

    if output == "-" {
        let mut serializer = Serializer::new(io::stdout());
        library.serialize(&mut serializer).unwrap();
    } else {
        let file = File::create(output).unwrap();
        let mut serializer = Serializer::new(file);
        library.serialize(&mut serializer).unwrap();
    }
}
