extern crate duct;

use clap::{Arg, Command};

fn main() {
    let matches = Command::new("test-lit-shell")
        .version("0.0.1")
        .author("Neil Henning <sheredom@gmail.com>")
        .about("Run a lit shell command.")
        .arg(
            Arg::new("command")
                .short('c')
                .value_name("command")
                .help("The command to run.")
                .required(true),
        )
        .get_matches();

    let command = matches.get_one::<String>("command").unwrap();

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
