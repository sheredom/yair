extern crate test;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    use test::Bencher;

    #[bench]
    fn create_library(b: &mut Bencher) {
        b.iter(|| {
            let _ = context::new();
        });
    }

    #[bench]
    fn create_module(b: &mut Bencher) {
        let mut context = Context::new();

        b.iter(|| {
            let _ = context.create_module().build();
        });
    }

    #[bench]
    fn create_function(b: &mut Bencher) {
        let mut context = Context::new();
        let module = context.create_module().build();

        b.iter(|| {
            let _ = module
                .create_function(&mut context)
                .with_name("func")
                .build();
        });
    }

    #[bench]
    fn create_global(b: &mut Bencher) {
        let mut context = Context::new();
        let module = context.create_module().build();

        b.iter(|| {
            let _ = module
                .create_global(&mut context)
                .with_name("global")
                .build();
        });
    }

    #[bench]
    fn create_block(b: &mut Bencher) {
        let mut context = Context::new();
        let module = context.create_module().build();
        let function = module
            .create_function(&mut context)
            .with_name("func")
            .build();

        b.iter(|| {
            let _ = function.create_block(&mut context).build();
        });
    }

    #[bench]
    fn create_instruction(b: &mut Bencher) {
        let mut context = Context::new();
        let module = context.create_module().build();
        let function = module
            .create_function(&mut context)
            .with_name("func")
            .build();
        let block = function.create_block(&mut context).build();

        b.iter(|| {
            let instruction_builder = block.create_instructions(&mut context);
            instruction_builder.ret(None);
        });
    }
}
