use crate::*;

#[derive(Default)]
pub struct BlockPayload {
    pub(crate) arguments: Vec<Value>,
    pub(crate) instructions: Vec<Value>,
}

#[derive(Clone, Copy, Debug)]
pub struct Block(pub(crate) generational_arena::Index);

impl Block {
    /// Get an argument from a block.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let function = module.create_function().with_name("func").build();
    /// # let ty = module.get_uint_type(32);
    /// # let _ = function.create_block(&mut module).build();
    /// # let block = function.create_block(&mut module).with_argument_types(&[ ty ]).build();
    /// let arg = block.get_arg(&module, 0);
    /// # assert_eq!(arg.get_type(&module), ty);
    /// ```
    pub fn get_arg(&self, module: &Module, index: usize) -> Value {
        let block = &module.blocks[self.0];

        assert!(
            index < block.arguments.len(),
            "Argument index {} is invalid {}",
            index,
            block.arguments.len()
        );

        block.arguments[index]
    }

    /// Add instructions to the block.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let function = module.create_function().with_name("func").build();
    /// # let block = function.create_block(&mut module).build();
    /// let instruction_builder = block.create_instructions(&mut module);
    /// ```
    pub fn create_instructions<'a>(&self, module: &'a mut Module) -> InstructionBuilder<'a> {
        InstructionBuilder::with_module_and_block(module, *self)
    }
}

pub struct BlockBuilder<'a> {
    module: &'a mut Module,
    function: Function,
    argument_types: &'a [Type],
}

impl<'a> BlockBuilder<'a> {
    pub(crate) fn with_module_and_function(module: &'a mut Module, function: Function) -> Self {
        BlockBuilder {
            module,
            function,
            argument_types: Default::default(),
        }
    }

    /// Add argument types for the block.
    ///
    /// The default is no argument types.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let function = module.create_function().with_name("func").build();
    /// # let i8_ty = module.get_int_type(8);
    /// # let u32_ty = module.get_uint_type(32);
    /// # let block_builder = function.create_block(&mut module);
    /// block_builder.with_argument_types(&[i8_ty, u32_ty]);
    /// ```
    pub fn with_argument_types(mut self, argument_types: &'a [Type]) -> Self {
        self.argument_types = argument_types;
        self
    }

    /// Finalize and build the block.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let function = module.create_function().with_name("func").build();
    /// # let block_builder = function.create_block(&mut module);
    /// let block = block_builder.build();
    /// ```
    pub fn build(self) -> Block {
        let mut block = BlockPayload {
            arguments: Vec::new(),
            instructions: Vec::new(),
        };

        let function = &mut self.module.functions[self.function.0];

        if function.blocks.is_empty() {
            assert!(
                self.argument_types.is_empty(),
                "The first block in a function cannot have any argument types"
            );
        }

        for argument_type in self.argument_types {
            let argument = self
                .module
                .values
                .insert(ValuePayload::Argument(Argument { ty: *argument_type }));
            block.arguments.push(Value(argument));
        }

        let block = Block(self.module.blocks.insert(block));

        function.blocks.push(block);

        block
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn bad_arg_index() {
        let mut module = Module::create_module().build();
        let function = module.create_function().with_name("func").build();
        let block = function.create_block(&mut module).build();
        let _ = block.get_arg(&module, 0);
    }

    #[test]
    #[should_panic]
    fn first_had_args() {
        let mut module = Module::create_module().build();
        let u32_ty = module.get_uint_type(32);
        let function = module.create_function().with_name("func").build();
        let _ = function
            .create_block(&mut module)
            .with_argument_types(&[u32_ty])
            .build();
    }
}
