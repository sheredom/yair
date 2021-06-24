extern crate envcache;

#[cfg(feature = "llvm")]
use std::path::PathBuf;

#[cfg(feature = "llvm")]
use envcache::EnvCache;

#[cfg(not(feature = "llvm"))]
fn link_in_llvm() {}

#[cfg(feature = "llvm")]
fn link_in_llvm() {
    let mut envcache = EnvCache::new();

    let llvm_dir = envcache
        .cache("YAIR_LLVM_INSTALL_DIR")
        .expect("Required environment variable 'YAIR_LLVM_INSTALL_DIR' was not set!");

    let mut llvm_path = PathBuf::new();

    llvm_path.push(llvm_dir);
    llvm_path.push("lib");

    println!(
        "cargo:rustc-link-search=native={}",
        llvm_path.to_str().unwrap()
    );

    if cfg!(not(target_os = "windows")) {
        println!("cargo:rustc-link-lib=dylib=ncurses");
        println!("cargo:rustc-link-lib=dylib=z");
    }

    println!("cargo:rustc-link-lib=static=LLVMAArch64AsmParser");
    println!("cargo:rustc-link-lib=static=LLVMAArch64CodeGen");
    println!("cargo:rustc-link-lib=static=LLVMAArch64Desc");
    println!("cargo:rustc-link-lib=static=LLVMAArch64Disassembler");
    println!("cargo:rustc-link-lib=static=LLVMAArch64Info");
    println!("cargo:rustc-link-lib=static=LLVMAArch64Utils");
    println!("cargo:rustc-link-lib=static=LLVMAsmPrinter");
    println!("cargo:rustc-link-lib=static=LLVMAnalysis");
    println!("cargo:rustc-link-lib=static=LLVMBinaryFormat");
    println!("cargo:rustc-link-lib=static=LLVMBitReader");
    println!("cargo:rustc-link-lib=static=LLVMBitstreamReader");
    println!("cargo:rustc-link-lib=static=LLVMBitWriter");
    println!("cargo:rustc-link-lib=static=LLVMCFGuard");
    println!("cargo:rustc-link-lib=static=LLVMCodeGen");
    println!("cargo:rustc-link-lib=static=LLVMCore");
    println!("cargo:rustc-link-lib=static=LLVMDebugInfoCodeView");
    println!("cargo:rustc-link-lib=static=LLVMDebugInfoDWARF");
    println!("cargo:rustc-link-lib=static=LLVMDebugInfoGSYM");
    println!("cargo:rustc-link-lib=static=LLVMDebugInfoMSF");
    //println!("cargo:rustc-link-lib=static=LLVMDebugInfoPDB");
    println!("cargo:rustc-link-lib=static=LLVMDemangle");
    println!("cargo:rustc-link-lib=static=LLVMExecutionEngine");
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
    println!("cargo:rustc-link-lib=static=LLVMTextAPI");
    println!("cargo:rustc-link-lib=static=LLVMTransformUtils");
    println!("cargo:rustc-link-lib=static=LLVMX86AsmParser");
    println!("cargo:rustc-link-lib=static=LLVMX86CodeGen");
    println!("cargo:rustc-link-lib=static=LLVMX86Desc");
    println!("cargo:rustc-link-lib=static=LLVMX86Disassembler");
    println!("cargo:rustc-link-lib=static=LLVMX86Info");

    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=dylib=c++");
    } else if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-lib=dylib=stdc++");
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
