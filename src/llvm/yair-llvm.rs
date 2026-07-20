extern crate rmp_serde;
extern crate serde;
extern crate yair;

use clap::{Arg, Command};
use std::fs::File;
use std::io::{self, Cursor, Write};
use std::str::FromStr;
use yair::llvm::Llvm;
use yair::{CodeGen, CodeGenOutput, Context};

fn main() {
    let matches = Command::new("yair-llvm")
        .version("0.0.1")
        .author("Neil Henning <sheredom@gmail.com>")
        .about("Compile a YAIR binary file (.yair) to LLVM.")
        .arg(
            Arg::new("input")
                .help("The input .yair file (or - for stdin)")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("FILE")
                .help("Where to write the output assembled .yair file (or - for stdout)")
                .default_value("-"),
        )
        .arg(
            Arg::new("type")
                .short('t')
                .long("type")
                .value_name("TYPE")
                .help("What output file type to produce (object or assembly file)")
                .default_value("Object"),
        )
        .arg(
            Arg::new("target")
                .long("target")
                .value_name("TARGET")
                .help("What target triple to generate code for")
                .default_value("aarch64-apple-darwin"),
        )
        .get_matches();

    let input = matches.get_one::<String>("input").unwrap();

    let context: Context = if input == "-" {
        rmp_serde::from_read(io::stdin())
    } else {
        let file = File::open(input).unwrap();
        rmp_serde::from_read(file)
    }
    .expect("Could not open malformed YAIR binary object");

    let output = matches.get_one::<String>("output").unwrap();

    let triple_triple = matches.get_one::<String>("target").unwrap();
    let code_gen_output =
        CodeGenOutput::from_str(matches.get_one::<String>("type").unwrap()).unwrap();

    if output == "-" {
        let mut cursor = Cursor::new(Vec::new());
        Llvm::generate(&context, triple_triple, code_gen_output, &mut cursor)
            .expect("Could not write data");

        io::stdout()
            .write_all(&cursor.into_inner())
            .expect("Could not write data");
    } else {
        let mut file = File::create(output).unwrap();
        Llvm::generate(&context, triple_triple, code_gen_output, &mut file)
            .expect("Could not write data");
    }
}
