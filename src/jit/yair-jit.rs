extern crate embedded_triple;
extern crate rmp_serde;
extern crate serde;
extern crate yair;

use clap::{Arg, Command};
use std::fs::File;
use std::io;

#[cfg(feature = "llvm")]
use yair::llvm::Llvm;

#[cfg(feature = "llvm")]
use yair::JitGen;

use yair::Context;

#[cfg(feature = "llvm")]
fn run_with_llvm(context: &Context, job: &str) {
    let codegen = Llvm::new(embedded_triple::get()).unwrap();

    let jit_fn = codegen.build_jit_fn(context, job).unwrap();

    jit_fn.run(())
}

#[cfg(not(feature = "llvm"))]
fn run_with_llvm(_: &Context, _: &str) {}

fn main() {
    let matches = Command::new("yair-jit")
        .version("0.0.1")
        .author("Neil Henning <sheredom@gmail.com>")
        .about("JIT execute a YAIR binary file (.yair).")
        .arg(
            Arg::new("input")
                .help("The input .yair file (or - for stdin)")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::new("backend")
                .long("backend")
                .value_name("BACKEND")
                .help("What backend to use to JIT with")
                .default_value("LLVM"),
        )
        .arg(
            Arg::new("job")
                .short('f')
                .long("function")
                .value_name("FUNCTION")
                .help("What function to JIT execute")
                .required(true),
        )
        .arg(
            Arg::new("gold")
                .short('g')
                .long("gold")
                .value_name("GOLD")
                .help("The golden value that the result of the function must match"),
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

    let backend = matches.get_one::<String>("backend").unwrap();

    if !cfg!(feature = "llvm") && backend == "LLVM" {
        panic!("LLVM backend requested but YAIR was not built with LLVM support");
    }

    let job = matches.get_one::<String>("job").unwrap();

    if backend == "LLVM" {
        run_with_llvm(&context, job);
    }
}
