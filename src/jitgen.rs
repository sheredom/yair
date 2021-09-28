use crate::Context;

pub trait JitFn<Args, Output> {
    fn run(&self, a: Args) -> Output;
}

pub trait JitGen {
    type Error;

    fn build_jit_fn<'a, Args: 'static, Output: 'static>(
        &'a self,
        context: &'a Context,
        entry_point: &str,
    ) -> Result<Box<dyn JitFn<Args, Output>>, Self::Error>;
}
