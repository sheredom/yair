use crate::*;

#[cfg_attr(feature = "io", derive(Serialize, Deserialize))]
pub struct BlockPayload {
    pub(crate) arguments: Vec<Value>,
    pub(crate) instructions: Vec<Value>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "io", derive(Serialize, Deserialize))]
pub struct Block(pub(crate) generational_arena::Index);

pub struct BlockDisplayer<'a> {
    pub(crate) block: Block,
    pub(crate) context: &'a Context,
}

impl<'a> std::fmt::Display for BlockDisplayer<'a> {
    fn fmt(
        &self,
        writer: &mut std::fmt::Formatter<'_>,
    ) -> std::result::Result<(), std::fmt::Error> {
        write!(writer, "b{}(", self.block.get_unique_index())?;

        for i in 0..self.block.get_num_args(self.context) {
            if i > 0 {
                write!(writer, ", ")?;
            }

            let arg = self.block.get_arg(self.context, i);

            write!(
                writer,
                "{} : {}",
                arg.get_displayer(self.context),
                arg.get_type(self.context).get_displayer(self.context)
            )?;
        }

        write!(writer, ")")
    }
}

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
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let function = module.create_function(&mut context).with_name("func").build();
    /// # let ty = context.get_uint_type(32);
    /// # let _ = function.create_block(&mut context).build();
    /// let block = function.create_block(&mut context).with_arg(ty).build();
    /// let arg = block.get_arg(&context, 0);
    /// assert_eq!(arg.get_type(&context), ty);
    /// ```
    pub fn get_arg(&self, context: &Context, index: usize) -> Value {
        let block = &context.blocks[self.0];

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
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let function = module.create_function(&mut context).with_name("func").build();
    /// # let ty = context.get_uint_type(32);
    /// # let _ = function.create_block(&mut context).build();
    /// let block = function.create_block(&mut context).with_arg(ty).build();
    /// let num_args = block.get_num_args(&context);
    /// assert_eq!(num_args, 1);
    /// ```
    pub fn get_num_args(&self, context: &Context) -> usize {
        let block = &context.blocks[self.0];

        block.arguments.len()
    }

    /// Add instructions to the block.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let function = module.create_function(&mut context).with_name("func").build();
    /// # let block = function.create_block(&mut context).build();
    /// let instruction_builder = block.create_instructions(&mut context);
    /// ```
    pub fn create_instructions<'a>(&self, context: &'a mut Context) -> InstructionBuilder<'a> {
        InstructionBuilder::with_context_and_block(context, *self)
    }

    /// Get all the instructions in a block.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let function = module.create_function(&mut context).with_name("func").build();
    /// # let _ = function.create_block(&mut context).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let constant = context.get_uint_constant(32, 42);
    /// let mut instruction_builder = block.create_instructions(&mut context);
    /// let instruction = instruction_builder.stack_alloc("😀", u32_ty, None);
    /// instruction_builder.ret(None);
    /// let mut instructions = block.get_insts(&context);
    /// assert_eq!(instructions.nth(0).unwrap(), instruction);
    /// ```
    pub fn get_insts(&self, context: &Context) -> ValueIterator {
        let block = &context.blocks[self.0];
        ValueIterator::new(&block.instructions)
    }

    /// Get all the arguments in a block.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let function = module.create_function(&mut context).with_name("func").with_arg("a", u32_ty).build();
    /// # let block = function.create_block(&mut context).with_arg(u32_ty).build();
    /// let mut args = block.get_args(&context);
    /// assert_eq!(args.nth(0).unwrap().get_type(&context), u32_ty);
    /// ```
    pub fn get_args(&self, context: &Context) -> ValueIterator {
        let block = &context.blocks[self.0];
        ValueIterator::new(&block.arguments)
    }

    /// Get a mutable reference to all the arguments in a block.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let function = module.create_function(&mut context).with_name("func").with_arg("a", u32_ty).build();
    /// # let block = function.create_block(&mut context).with_arg(u32_ty).build();
    /// let mut args = block.get_args_mut(&mut context);
    /// # args.push(u32_ty);
    /// ```
    pub fn get_args_mut<'a>(&self, context: &'a mut Context) -> BlockArguments<'a> {
        BlockArguments::new(context, *self)
    }

    pub fn get_displayer<'a>(&self, context: &'a Context) -> BlockDisplayer<'a> {
        BlockDisplayer {
            block: *self,
            context,
        }
    }
}

pub struct BlockBuilder<'a> {
    context: &'a mut Context,
    function: Function,
    argument_types: Vec<Type>,
}

impl<'a> BlockBuilder<'a> {
    pub(crate) fn with_context_and_function(context: &'a mut Context, function: Function) -> Self {
        BlockBuilder {
            context,
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
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let function = module.create_function(&mut context).with_name("func").build();
    /// # let i8_ty = context.get_int_type(8);
    /// # let u32_ty = context.get_uint_type(32);
    /// # let block_builder = function.create_block(&mut context);
    /// block_builder.with_arg(i8_ty).with_arg(u32_ty);
    /// ```
    pub fn with_arg(mut self, ty: Type) -> Self {
        self.argument_types.push(ty);
        self
    }

    /// Add many arguments to a block.
    ///
    /// The default is no argument types.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let function = module.create_function(&mut context).with_name("func").build();
    /// # let i8_ty = context.get_int_type(8);
    /// # let u32_ty = context.get_uint_type(32);
    /// # let block_builder = function.create_block(&mut context);
    /// block_builder.with_args(&[i8_ty, u32_ty]);
    /// ```
    pub fn with_args(mut self, arguments: &[Type]) -> Self {
        for argument in arguments {
            self.argument_types.push(*argument); 
        }
        self
    }

    /// Finalize and build the block.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let function = module.create_function(&mut context).with_name("func").build();
    /// # let block_builder = function.create_block(&mut context);
    /// let block = block_builder.build();
    /// ```
    pub fn build(self) -> Block {
        let mut block = BlockPayload {
            arguments: Vec::new(),
            instructions: Vec::new(),
        };

        let name = self.context.get_name("");

        let function = &mut self.context.functions[self.function.0];

        for argument_type in self.argument_types {
            let argument = self.context.values.insert(ValuePayload::Argument(Argument {
                name,
                ty: argument_type,
            }));
            block.arguments.push(Value(argument));
        }

        let block = Block(self.context.blocks.insert(block));

        function.blocks.push(block);

        block
    }
}
