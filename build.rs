extern crate envcache;

#[cfg(feature = "llvm")]
use std::{path::PathBuf, process::Command};

#[cfg(feature = "llvm")]
use envcache::EnvCache;

#[cfg(not(feature = "llvm"))]
fn link_in_llvm() {}

#[cfg(feature = "llvm")]
fn llvm_config_output(llvm_config: &PathBuf, args: &[&str]) -> String {
    let output = Command::new(llvm_config)
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("Failed to run {}: {error}", llvm_config.display()));

    if !output.status.success() {
        panic!(
            "{} {} failed with status {}: {}",
            llvm_config.display(),
            args.join(" "),
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    String::from_utf8(output.stdout)
        .expect("llvm-config output was not UTF-8")
        .trim()
        .to_string()
}

#[cfg(feature = "llvm")]
fn link_in_llvm() {
    let mut envcache = EnvCache::new();

    let llvm_dir = envcache
        .cache("YAIR_LLVM_INSTALL_DIR")
        .expect("Required environment variable 'YAIR_LLVM_INSTALL_DIR' was not set!");

    let mut llvm_config = PathBuf::new();
    llvm_config.push(llvm_dir);
    llvm_config.push("bin");
    llvm_config.push(if cfg!(target_os = "windows") {
        "llvm-config.exe"
    } else {
        "llvm-config"
    });

    let libdir = llvm_config_output(&llvm_config, &["--libdir"]);
    println!("cargo:rustc-link-search=native={libdir}");

    if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{libdir}");
    }

    for flag in llvm_config_output(&llvm_config, &["--libs", "--system-libs", "--link-shared"])
        .split_whitespace()
    {
        if let Some(lib) = flag.strip_prefix("-l") {
            println!("cargo:rustc-link-lib=dylib={lib}");
        } else if let Some(path) = flag.strip_prefix("-L") {
            println!("cargo:rustc-link-search=native={path}");
        } else {
            println!("cargo:rustc-link-arg={flag}");
        }
    }
}

#[cfg(not(feature = "lld"))]
fn link_in_lld() {}

#[cfg(feature = "lld")]
fn link_in_lld() {
    // Since LLD depends on the LLVM feature, we can assume all the correct LLVM
    // bits have been pulled in.
    println!("cargo:rustc-link-lib=static=LLVMOption");
    println!("cargo:rustc-link-lib=static=lldCOFF");
    println!("cargo:rustc-link-lib=static=lldCommon");
    println!("cargo:rustc-link-lib=static=lldCore");
    println!("cargo:rustc-link-lib=static=lldDriver");
    println!("cargo:rustc-link-lib=static=lldELF");
    println!("cargo:rustc-link-lib=static=lldMacho");
}

fn main() {
    link_in_llvm();
    link_in_lld();
}
