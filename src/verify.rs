use crate::*;
use std::collections::HashSet;

pub enum VerifyError<'a> {
    FunctionAndFirstBlockMustHaveMatchingArguments(&'a Library, Function, Block),
}

impl<'a> std::fmt::Display for VerifyError<'a> {
    fn fmt(
        &self,
        formatter: &mut std::fmt::Formatter<'_>,
    ) -> std::result::Result<(), std::fmt::Error> {
        match self {
            VerifyError::FunctionAndFirstBlockMustHaveMatchingArguments(l, f, b) => {
                writeln!(formatter, "A function and the first block contained within it must have matching arguments:")?;
                writeln!(formatter, "  {}", f.get_displayer(l))?;
                writeln!(formatter, "  {}", b.get_displayer(l))
            }
        }
    }
}

struct Verifier<'a> {
    library: &'a Library,
    live_values: HashSet<Value>,
}

impl<'a> Verifier<'a> {
    fn new(library: &'a Library) -> Self {
        Self {
            library,
            live_values: HashSet::new(),
        }
    }

    fn verify_function(&mut self, function: Function) -> Result<(), VerifyError<'a>> {
        for (index, block) in function.get_blocks(self.library).enumerate() {
            // The first block has to have the same arguments as the function.
            if index == 0 {
                if function.get_num_args(self.library) != block.get_num_args(self.library) {
                    return Err(VerifyError::FunctionAndFirstBlockMustHaveMatchingArguments(
                        self.library,
                        function,
                        block,
                    ));
                }

                for (farg, barg) in function
                    .get_args(self.library)
                    .zip(block.get_args(self.library))
                {
                    if farg.get_type(self.library) != barg.get_type(self.library) {
                        return Err(VerifyError::FunctionAndFirstBlockMustHaveMatchingArguments(
                            self.library,
                            function,
                            block,
                        ));
                    }
                }
            }

            // Insert all the arguments as being live.
            block.get_args(self.library).for_each(|a| {
                self.live_values.insert(a);
            });

            // Remove all the arguments which are no longer live.
            block.get_args(self.library).for_each(|a| {
                self.live_values.remove(&a);
            });
        }

        Ok(())
    }

    fn verify_module(&mut self, module: Module) -> Result<(), VerifyError<'a>> {
        for function in module.get_functions(self.library) {
            self.verify_function(function)?;
        }

        Ok(())
    }
}

pub(crate) fn verify(library: &Library, module: Module) -> Result<(), VerifyError> {
    let mut verifier = Verifier::new(library);
    verifier.verify_module(module)
}