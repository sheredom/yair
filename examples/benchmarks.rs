use std::env;
use std::path::PathBuf;

fn bin_dir() -> PathBuf {
    env::current_exe()
        .ok()
        .map(|mut path| {
            path.pop();
            path.pop();
            path
        })
        .unwrap()
}

fn get_file_size(file: &str) -> u64 {
    let path = bin_dir().join(format!("{}{}", file, env::consts::EXE_SUFFIX));
    std::fs::metadata(path.to_str().unwrap()).unwrap().len()
}

fn main() {
    let mut first = true;

    println!("[");

    for file in &[
        "bootstrap",
        "yair-as",
        "yair-dis",
        "yair-llvm",
        "yair-verify",
    ] {
        let size = get_file_size(*file);

        if first {
            first = false;
        } else {
            println!(",");
        }

        print!(
            "   {{ \"name\" : \"{}\", \"unit\" : \"bytes\", \"value\" : {} }}",
            file, size
        );
    }

    println!("");
    println!("]");
}
