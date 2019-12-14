extern crate lit;

#[cfg(test)]
mod tests {
    use std::env;
    use std::env::consts;
    use std::path::PathBuf;

    fn yair_as_dir() -> PathBuf {
        env::current_exe()
            .ok()
            .map(|mut path| {
                path.pop();
                path.pop();
                path
            })
            .unwrap()
    }

    fn yair_as_exe() -> String {
        yair_as_dir()
            .join(format!("yair-as{}", env::consts::EXE_SUFFIX))
            .to_str()
            .unwrap()
            .to_string()
    }

    #[test]
    fn lit() {
        lit::run::tests(|config| {
            config.add_search_path("tests/lit");
            config.add_extension("ya");

            config.constants.insert("yair_as".to_owned(), yair_as_exe());
            config
                .constants
                .insert("arch".to_owned(), consts::ARCH.to_owned());
            config
                .constants
                .insert("os".to_owned(), consts::OS.to_owned());
        })
        .expect("Lit tests failed");
    }
}
