extern crate codespan;
extern crate rmp_serde;
extern crate serde;

use clap::{Arg, Command};
use codespan::Files;
use codespan_reporting::term::emit_to_write_style;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use rmp_serde::Serializer;
use serde::Serialize;
use std::fs::File;
use std::io::{self, Read};
use std::process::exit;
use yair::io::assemble;

fn color_choice(color: &str) -> ColorChoice {
    match color {
        "auto" => ColorChoice::Auto,
        "always" => ColorChoice::Always,
        "ansi" => ColorChoice::AlwaysAnsi,
        "never" => ColorChoice::Never,
        _ => unreachable!("clap restricts color choices"),
    }
}

fn main() {
    let matches = Command::new("yair-as")
        .version("0.0.1")
        .author("Neil Henning <sheredom@gmail.com>")
        .about("Assemble a YAIR text file (.yail) to a YAIR binary file (.yair).")
        .arg(
            Arg::new("input")
                .help("The input .yail file (or - for stdin)")
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
            Arg::new("color")
                .long("color")
                .value_name("COLOR")
                .help("Whether to allow colored command line output")
                .default_value("auto")
                .value_parser(["auto", "always", "ansi", "never"]),
        )
        .get_matches();

    let input = matches.get_one::<String>("input").unwrap();

    let mut data = String::new();

    if input == "-" {
        io::stdin().read_to_string(&mut data).unwrap();
    } else {
        let mut file = File::open(input).unwrap();
        file.read_to_string(&mut data).unwrap();
    }

    let mut files = Files::new();
    let file = files.add(input, &data);

    let color = color_choice(matches.get_one::<String>("color").unwrap());

    let writer = StandardStream::stderr(color);
    let config = codespan_reporting::term::Config::default();

    let context = match assemble(file, &data) {
        Ok(l) => l,
        Err(d) => {
            emit_to_write_style(&mut writer.lock(), &config, &files, &d).unwrap();
            exit(1);
        }
    };

    let output = matches.get_one::<String>("output").unwrap();

    if output == "-" {
        let mut serializer = Serializer::new(io::stdout());
        context.serialize(&mut serializer)
    } else {
        let file = File::create(output).unwrap();
        let mut serializer = Serializer::new(file);
        context.serialize(&mut serializer)
    }
    .expect("Serde failed to serialize the library to a binary");
}
