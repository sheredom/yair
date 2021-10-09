#[macro_use]
extern crate clap;
extern crate duct;

use clap::App;

fn main() {
    let yaml = load_yaml!("test-lit-shell.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let command = matches.value_of("command").unwrap();

    let splits = command.split('|');

    let mut stdin = Vec::new();

    for split in splits {
        let split = split.trim();

        let command_str = split.split(' ').next().unwrap();
        let args: Vec<_> = split.split(' ').skip(1).collect();

        let command = duct::cmd(command_str, args)
            .stderr_to_stdout()
            .stdin_bytes(stdin)
            .stdout_capture()
            .unchecked()
            .run();

        if let Err(e) = command {
            eprintln!("Command '{}' failed with error '{}'", command_str, e);
            std::process::exit(1);
        }

        stdin = command.unwrap().stdout;
    }

    print!("{}", String::from_utf8(stdin).unwrap());
}
