extern crate test;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llvm::Llvm;
    use crate::*;
    use std::io::Cursor;
    use test::Bencher;

    fn splat_adds() -> Library {
        let mut context = Context::new();
        let module = context.create_module().build();
        let u32_ty = context.get_uint_type(32);
        let function = module
            .create_function(&mut context)
            .with_name("func")
            .with_return_type(u32_ty)
            .with_arg("a", u32_ty)
            .with_arg("b", u32_ty)
            .build();
        let block = function.create_block(&mut context).build();
        let x = function.get_arg(&context, 0);
        let y = function.get_arg(&context, 1);
        let mut instruction_builder = block.create_instructions(&mut context);
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

        let target_triple = "aarch64-apple-darwin";
        let code_gen_output = CodeGenOutput::Object;

        b.iter(|| {
            Llvm::generate(&context, target_triple, code_gen_output, &mut cursor)
                .expect("Could not write data")
        });
    }
}
