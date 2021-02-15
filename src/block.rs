use crate::*;

#[cfg_attr(feature = "io", derive(Serialize, Deserialize))]
pub struct BlockPayload {
    pub(crate) arguments: Vec<Value>,
    pub(crate) instructions: Vec<Value>,
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "io", derive(Serialize, Deserialize))]
pub struct Block(pub(crate) generational_arena::Index);

impl UniqueIndex for Block {
    fn get_unique_index(&self) -> usize {
        self.0.into_raw_parts().0
    }
}

impl Block {
    /// Get an argument from a block.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let function = module.create_function(&mut library).with_name("func").build();
    /// # let ty = library.get_uint_type(32);
    /// # let _ = function.create_block(&mut library).build();
    /// let block = function.create_block(&mut library).with_argument(ty).build();
    /// let arg = block.get_arg(&library, 0);
    /// assert_eq!(arg.get_type(&library), ty);
    /// ```
    pub fn get_arg(&self, library: &Library, index: usize) -> Value {
        let block = &library.blocks[self.0];

        assert!(
            index < block.arguments.len(),
            "Argument index {} is invalid {}",
            index,
            block.arguments.len()
        );

        block.arguments[index]
    }

    /// Get an argument from a block.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let function = module.create_function(&mut library).with_name("func").build();
    /// # let ty = library.get_uint_type(32);
    /// # let _ = function.create_block(&mut library).build();
    /// let block = function.create_block(&mut library).with_argument(ty).build();
    /// let num_args = block.get_num_args(&library);
    /// assert_eq!(num_args, 1);
    /// ```
    pub fn get_num_args(&self, library: &Library) -> usize {
        let block = &library.blocks[self.0];

        block.arguments.len()
    }

    /// Add instructions to the block.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let function = module.create_function(&mut library).with_name("func").build();
    /// # let block = function.create_block(&mut library).build();
    /// let instruction_builder = block.create_instructions(&mut library);
    /// ```
    pub fn create_instructions<'a>(&self, library: &'a mut Library) -> InstructionBuilder<'a> {
        InstructionBuilder::with_library_and_block(library, *self)
    }

    /// Get all the blocks in a function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let u32_ty = library.get_uint_type(32);
    /// # let function = module.create_function(&mut library).with_name("func").build();
    /// # let _ = function.create_block(&mut library).build();
    /// # let block = function.create_block(&mut library).build();
    /// # let constant = library.get_uint_constant(32, 42);
    /// let mut instruction_builder = block.create_instructions(&mut library);
    /// let instruction = instruction_builder.stack_alloc("😀", u32_ty, None);
    /// instruction_builder.ret(None);
    /// let mut instructions = block.get_insts(&library);
    /// assert_eq!(instructions.nth(0).unwrap(), instruction);
    /// ```
    pub fn get_insts(&self, library: &Library) -> InstructionIterator {
        let block = &library.blocks[self.0];
        InstructionIterator::new(&block.instructions)
    }
}

pub struct BlockBuilder<'a> {
    library: &'a mut Library,
    function: Function,
    argument_types: Vec<Type>,
}

impl<'a> BlockBuilder<'a> {
    pub(crate) fn with_library_and_function(library: &'a mut Library, function: Function) -> Self {
        BlockBuilder {
            library,
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
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let function = module.create_function(&mut library).with_name("func").build();
    /// # let i8_ty = library.get_int_type(8);
    /// # let u32_ty = library.get_uint_type(32);
    /// # let block_builder = function.create_block(&mut library);
    /// block_builder.with_argument(i8_ty).with_argument(u32_ty);
    /// ```
    pub fn with_argument(mut self, ty: Type) -> Self {
        self.argument_types.push(ty);
        self
    }

    /// Finalize and build the block.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let function = module.create_function(&mut library).with_name("func").build();
    /// # let block_builder = function.create_block(&mut library);
    /// let block = block_builder.build();
    /// ```
    pub fn build(self) -> Block {
        let mut block = BlockPayload {
            arguments: Vec::new(),
            instructions: Vec::new(),
        };

        let name = self.library.get_name("");

        let function = &mut self.library.functions[self.function.0];

        for argument_type in self.argument_types {
            let argument = self.library.values.insert(ValuePayload::Argument(Argument {
                name,
                ty: argument_type,
            }));
            block.arguments.push(Value(argument));
        }

        let block = Block(self.library.blocks.insert(block));

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
        let mut library = Library::new();
        let module = library.create_module().build();
        let function = module
            .create_function(&mut library)
            .with_name("func")
            .build();
        let block = function.create_block(&mut library).build();
        let _ = block.get_arg(&library, 0);
    }

    #[test]
    fn first_had_args() {
        let mut library = Library::new();
        let module = library.create_module().build();
        let u32_ty = library.get_uint_type(32);
        let function = module
            .create_function(&mut library)
            .with_argument("a", u32_ty)
            .with_name("func")
            .build();
        let _ = function
            .create_block(&mut library)
            .with_argument(u32_ty)
            .build();
    }
}

pub struct InstructionIterator {
    vec: Vec<Value>,
    next: usize,
}

impl InstructionIterator {
    fn new(iter: &[Value]) -> InstructionIterator {
        InstructionIterator {
            vec: iter.to_vec(),
            next: 0,
        }
    }
}

impl Iterator for InstructionIterator {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next < self.vec.len() {
            let next = self.next;
            self.next += 1;
            Some(self.vec[next])
        } else {
            None
        }
    }
}