extern crate rmp_serde;
extern crate serde;
extern crate yair;

use clap::{Arg, Command};
use std::fs::File;
use std::io::{self};
use yair::Context;

fn main() {
    let matches = Command::new("yair-verify")
        .version("0.0.1")
        .author("Neil Henning <sheredom@gmail.com>")
        .about("Verify a YAIR binary file (.yair) is valid.")
        .arg(
            Arg::new("input")
                .help("The input .yair file (or - for stdin)")
                .required(true)
                .index(1),
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

    match context.verify() {
        Ok(_) => (),
        Err(e) => println!("{}", e),
    }
}
