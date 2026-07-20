extern crate rmp_serde;
extern crate serde;
extern crate yair;

use clap::{Arg, Command};
use std::fs::File;
use std::io::{self};
use yair::io::disassemble;
use yair::Context;

fn main() {
    let matches = Command::new("yair-dis")
        .version("0.0.1")
        .author("Neil Henning <sheredom@gmail.com>")
        .about("Disassemble a YAIR binary file (.yair) to a YAIR text file (.yail).")
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
                .help("Where to write the output disassembled .yail file (or - for stdout)")
                .default_value("-"),
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

    if output == "-" {
        disassemble(&context, io::stdout().lock())
    } else {
        let file = File::create(output).unwrap();
        disassemble(&context, file)
    }
    .expect("Could not write data");
}
