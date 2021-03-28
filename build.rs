extern crate envcache;

#[cfg(feature = "llvm")]
extern crate cc;

#[cfg(feature = "llvm")]
use std::path::PathBuf;

#[cfg(feature = "llvm")]
use std::env;

#[cfg(feature = "llvm")]
use envcache::EnvCache;

#[cfg(not(feature = "llvm"))]
fn link_in_llvm() {}

#[cfg(feature = "llvm")]
fn link_in_llvm() {
    let mut envcache = EnvCache::new();
    if let Some(llvm_dir) = envcache.cache("YAIR_LLVM_INSTALL_DIR") {
        let mut llvm_path = PathBuf::new();

        llvm_path.push(llvm_dir);
        llvm_path.push("lib");

        println!(
            "cargo:rustc-link-search=native={}",
            llvm_path.to_str().unwrap()
        );
    }

    cc::Build::new()
        .file("src/llvm/extras.cpp")
        .include(&env::var("CARGO_MANIFEST_DIR").unwrap())
        .cpp(true)
        .compile("yair-llvm-cpp-link");

    if cfg!(not(target_os = "windows")) {
        println!("cargo:rustc-link-lib=dylib=ncurses");
    }

    println!("cargo:rustc-link-lib=static=LLVMASMPrinter");
    println!("cargo:rustc-link-lib=static=LLVMAnalysis");
    println!("cargo:rustc-link-lib=static=LLVMBinaryFormat");
    println!("cargo:rustc-link-lib=static=LLVMBitWriter");
    println!("cargo:rustc-link-lib=static=LLVMCFGuard");
    println!("cargo:rustc-link-lib=static=LLVMCodegen");
    println!("cargo:rustc-link-lib=static=LLVMCore");
    println!("cargo:rustc-link-lib=static=LLVMDebugInfoCodeView");
    println!("cargo:rustc-link-lib=static=LLVMDebugInfoDWARF");
    println!("cargo:rustc-link-lib=static=LLVMDebugInfoPDB");
    println!("cargo:rustc-link-lib=static=LLVMDemangle");
    println!("cargo:rustc-link-lib=static=LLVMGlobalISel");
    println!("cargo:rustc-link-lib=static=LLVMMC");
    println!("cargo:rustc-link-lib=static=LLVMMCDisassembler");
    println!("cargo:rustc-link-lib=static=LLVMMCParser");
    println!("cargo:rustc-link-lib=static=LLVMObject");
    println!("cargo:rustc-link-lib=static=LLVMProfileData");
    println!("cargo:rustc-link-lib=static=LLVMRemarks");
    println!("cargo:rustc-link-lib=static=LLVMScalarOpts");
    println!("cargo:rustc-link-lib=static=LLVMSelectionDAG");
    println!("cargo:rustc-link-lib=static=LLVMSupport");
    println!("cargo:rustc-link-lib=static=LLVMTarget");
    println!("cargo:rustc-link-lib=static=LLVMTransformUtils");
    println!("cargo:rustc-link-lib=static=LLVMX86AsmParser");
    println!("cargo:rustc-link-lib=static=LLVMX86CodeGen");
    println!("cargo:rustc-link-lib=static=LLVMX86Desc");
    println!("cargo:rustc-link-lib=static=LLVMX86Disassembler");
    println!("cargo:rustc-link-lib=static=LLVMX86Info");
    println!("cargo:rustc-link-lib=static=LLVMAArch64AsmParser");
    println!("cargo:rustc-link-lib=static=LLVMAArch64CodeGen");
    println!("cargo:rustc-link-lib=static=LLVMAArch64Desc");
    println!("cargo:rustc-link-lib=static=LLVMAArch64Disassembler");
    println!("cargo:rustc-link-lib=static=LLVMAArch64Info");
}

fn main() {
    link_in_llvm();
}
