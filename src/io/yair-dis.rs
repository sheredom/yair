#[macro_use]
extern crate clap;
extern crate rmp_serde;
extern crate serde;
extern crate yair;

use clap::App;
use std::fs::File;
use std::io::{self};
use yair::io::disassemble;
use yair::Library;

fn main() {
    let yaml = load_yaml!("yair-dis.yml");
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

    if output == "-" {
        disassemble(&library, io::stdout().lock())
    } else {
        let file = File::create(output).unwrap();
        disassemble(&library, file)
    }
    .expect("Could not write data");
}
