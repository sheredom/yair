#[macro_use]
extern crate clap;
extern crate rmp_serde;
extern crate serde;
extern crate yair;

use clap::App;
use std::fs::File;
use std::io::{self, Cursor, Write};
use std::str::FromStr;
use yair::llvm::Llvm;
use yair::{CodeGen, CodeGenOutput, Library};

fn main() {
    let yaml = load_yaml!("yair-llvm.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let input = matches.value_of("input").unwrap();

    let library: Library = if input == "-" {
        rmp_serde::from_read(io::stdin())
    } else {
        let file = File::open(input).unwrap();
        rmp_serde::from_read(file)
    }
    .expect("Could not open malformed YAIR binary object");

    let output = matches.value_of("output").unwrap();

    let triple_triple = matches.value_of("target").unwrap();
    let code_gen_output = CodeGenOutput::from_str(matches.value_of("type").unwrap()).unwrap();

    if output == "-" {
        let mut cursor = Cursor::new(Vec::new());
        Llvm::generate(&library, triple_triple, code_gen_output, &mut cursor)
            .expect("Could not write data");

        io::stdout()
            .write_all(&cursor.into_inner())
            .expect("Could not write data");
    } else {
        let mut file = File::create(output).unwrap();
        Llvm::generate(&library, triple_triple, code_gen_output, &mut file)
            .expect("Could not write data");
    }
}
