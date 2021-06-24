#[macro_use]
extern crate clap;
extern crate rmp_serde;
extern crate serde;
extern crate yair;

use clap::App;
use std::fs::File;
use std::io::{self, Cursor, Write};
use std::str::FromStr;

#[cfg(feature = "llvm")]
use yair::llvm::Llvm;

use yair::FunctionAttribute;
use yair::{CodeGen, CodeGenOutput, Function, JitGen, Library};

#[cfg(feature = "llvm")]
fn run_with_llvm(library: &Library, job: &str) {
    let codegen = Llvm::new("aarch64-apple-darwin").unwrap();

    codegen.build_jit_fn(library, job).unwrap().run(())
}

fn main() {
    let yaml = load_yaml!("yair-jit.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let input = matches.value_of("input").unwrap();

    let library: Library = if input == "-" {
        rmp_serde::from_read(io::stdin())
    } else {
        let file = File::open(input).unwrap();
        rmp_serde::from_read(file)
    }
    .expect("Could not open malformed YAIR binary object");

    let backend = matches.value_of("backend").unwrap();

    if !cfg!(feature = "llvm") && backend == "LLVM" {
        panic!("LLVM backend requested but YAIR was not built with LLVM support");
    }

    let job = matches.value_of("job").unwrap();

    if backend == "LLVM" {
        run_with_llvm(&library, job);
    }
}
