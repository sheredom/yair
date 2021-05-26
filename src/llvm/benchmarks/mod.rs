extern crate test;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llvm::Llvm;
    use crate::*;
    use std::io::Cursor;
    use test::Bencher;

    fn splat_adds() -> Library {
        let mut library = Library::new();
        let module = library.create_module().build();
        let u32_ty = library.get_uint_type(32);
        let function = module
            .create_function(&mut library)
            .with_name("func")
            .with_return_type(u32_ty)
            .with_arg("a", u32_ty)
            .with_arg("b", u32_ty)
            .build();
        let block = function.create_block(&mut library).build();
        let x = function.get_arg(&library, 0);
        let y = function.get_arg(&library, 1);
        let mut instruction_builder = block.create_instructions(&mut library);
        let location = None;

        let mut result = instruction_builder.add(x, y, location);
        for _ in 0..100 {
            result = instruction_builder.add(result, y, location);
        }

        instruction_builder.ret_val(result, location);

        library
    }

    #[bench]
    fn bench_splat_adds(b: &mut Bencher) {
        let library = splat_adds();

        let mut cursor = Cursor::new(Vec::new());

        let code_gen_platform = CodeGenPlatform::MacOsAppleSilicon;
        let code_gen_output = CodeGenOutput::Object;

        b.iter(|| {
            Llvm::generate(&library, code_gen_platform, code_gen_output, &mut cursor)
                .expect("Could not write data")
        });
    }
}
