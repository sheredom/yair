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

        let command = split.split(' ').next().unwrap();
        let args: Vec<_> = split.split(' ').skip(1).collect();

        let command = duct::cmd(command, args)
            .stderr_to_stdout()
            .stdin_bytes(stdin)
            .stdout_capture()
            .unchecked()
            .run()
            .unwrap();

        stdin = command.stdout;
    }

    print!("{}", String::from_utf8(stdin).unwrap());
}
