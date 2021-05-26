extern crate lit;

#[cfg(feature = "io")]
#[cfg(test)]
mod tests {
    use std::env;
    use std::env::consts;
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

    fn yair_as_exe() -> String {
        bin_dir()
            .join(format!("yair-as{}", env::consts::EXE_SUFFIX))
            .to_str()
            .unwrap()
            .to_string()
    }

    fn yair_dis_exe() -> String {
        bin_dir()
            .join(format!("yair-dis{}", env::consts::EXE_SUFFIX))
            .to_str()
            .unwrap()
            .to_string()
    }

    fn yair_verify_exe() -> String {
        bin_dir()
            .join(format!("yair-verify{}", env::consts::EXE_SUFFIX))
            .to_str()
            .unwrap()
            .to_string()
    }

    fn lit_shell_exe() -> String {
        bin_dir()
            .join(format!("test-lit-shell{}", env::consts::EXE_SUFFIX))
            .to_str()
            .unwrap()
            .to_string()
    }

    #[cfg(not(feature = "llvm"))]
    fn add_yair_llvm(_: &mut lit::Config) {}

    #[cfg(feature = "llvm")]
    fn add_yair_llvm(config: &mut lit::Config) {
        let yair_llvm_exe = bin_dir()
            .join(format!("yair-llvm{}", env::consts::EXE_SUFFIX))
            .to_str()
            .unwrap()
            .to_string();

        config.add_search_path("tests/lit/llvm");

        config
            .constants
            .insert("yair_llvm".to_owned(), yair_llvm_exe);
    }

    #[test]
    fn lit() {
        lit::run::tests(lit::event_handler::Default::default(), |config| {
            config.add_search_path("tests/lit/all");
            config.add_extension("ya");

            config.constants.insert("yair_as".to_owned(), yair_as_exe());
            config
                .constants
                .insert("yair_dis".to_owned(), yair_dis_exe());
            config
                .constants
                .insert("yair_verify".to_owned(), yair_verify_exe());
            config
                .constants
                .insert("arch".to_owned(), consts::ARCH.to_owned());
            config
                .constants
                .insert("os".to_owned(), consts::OS.to_owned());

            add_yair_llvm(config);

            config.shell = lit_shell_exe();
        })
        .expect("Lit tests failed");
    }
}
