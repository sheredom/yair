use crate::Library;

pub trait JitFn<Args, Output> {
    fn run(&self, a: Args) -> Output;
}

pub trait JitGen {
    type Error;

    fn build_jit_fn<'a, Args: 'static, Output: 'static>(
        &'a self,
        library: &'a Library,
        entry_point: &str,
    ) -> Result<Box<dyn JitFn<Args, Output>>, Self::Error>;
}
