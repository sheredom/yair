extern crate test;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    use test::Bencher;

    #[bench]
    fn create_library(b: &mut Bencher) {
        b.iter(|| {
            let _ = Library::new();
        });
    }

    #[bench]
    fn create_module(b: &mut Bencher) {
        let mut library = Library::new();

        b.iter(|| {
            let _ = library.create_module().build();
        });
    }

    #[bench]
    fn create_function(b: &mut Bencher) {
        let mut library = Library::new();
        let module = library.create_module().build();

        b.iter(|| {
            let _ = module
                .create_function(&mut library)
                .with_name("func")
                .build();
        });
    }

    #[bench]
    fn create_global(b: &mut Bencher) {
        let mut library = Library::new();
        let module = library.create_module().build();

        b.iter(|| {
            let _ = module
                .create_global(&mut library)
                .with_name("global")
                .build();
        });
    }

    #[bench]
    fn create_block(b: &mut Bencher) {
        let mut library = Library::new();
        let module = library.create_module().build();
        let function = module
            .create_function(&mut library)
            .with_name("func")
            .build();

        b.iter(|| {
            let _ = function.create_block(&mut library).build();
        });
    }

    #[bench]
    fn create_instruction(b: &mut Bencher) {
        let mut library = Library::new();
        let module = library.create_module().build();
        let function = module
            .create_function(&mut library)
            .with_name("func")
            .build();
        let block = function.create_block(&mut library).build();

        b.iter(|| {
            let instruction_builder = block.create_instructions(&mut library);
            instruction_builder.ret(None);
        });
    }
}
