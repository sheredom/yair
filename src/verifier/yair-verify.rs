#[macro_use]
extern crate clap;
extern crate rmp_serde;
extern crate serde;
extern crate yair;

use clap::App;
use std::fs::File;
use std::io::{self};
use yair::Context;

fn main() {
    let yaml = load_yaml!("yair-verify.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let input = matches.value_of("input").unwrap();

    let context: Context = if input == "-" {
        rmp_serde::from_read(io::stdin())
    } else {
        let file = File::open(input).unwrap();
        rmp_serde::from_read(file)
    }
    .expect("Could not open malformed YAIR binary object");

    match context.verify() {
        Ok(_) => (),
        Err(e) => println!("{}", e),
    }
}
