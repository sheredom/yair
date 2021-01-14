#[macro_use]
extern crate clap;
extern crate codespan;
extern crate rmp_serde;
extern crate serde;

use clap::App;
use codespan::Files;
use codespan_reporting::term::termcolor::StandardStream;
use codespan_reporting::term::{emit, ColorArg};
use rmp_serde::Serializer;
use serde::Serialize;
use std::fs::File;
use std::io::{self, Read};
use std::process::exit;
use std::str::FromStr;
use yair::io::assemble;

fn main() {
    let yaml = load_yaml!("yair-as.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let input = matches.value_of("input").unwrap();

    let mut data = String::new();

    if input == "-" {
        io::stdin().read_to_string(&mut data).unwrap();
    } else {
        let mut file = File::open(input).unwrap();
        file.read_to_string(&mut data).unwrap();
    }

    let mut files = Files::new();
    let file = files.add(input, &data);

    let color = ColorArg::from_str(matches.value_of("color").unwrap()).unwrap();

    let writer = StandardStream::stderr(color.into());
    let config = codespan_reporting::term::Config::default();

    let library = match assemble(file, &data) {
        Ok(l) => l,
        Err(d) => {
            emit(&mut writer.lock(), &config, &files, &d).unwrap();
            exit(1);
        }
    };

    let output = matches.value_of("output").unwrap();

    if output == "-" {
        let mut serializer = Serializer::new(io::stdout());
        library.serialize(&mut serializer)
    } else {
        let file = File::create(output).unwrap();
        let mut serializer = Serializer::new(file);
        library.serialize(&mut serializer)
    }
    .expect("Serde failed to serialize the library to a binary");
}
