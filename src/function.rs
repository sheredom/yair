use crate::*;

#[derive(EnumSetType, Debug)]
#[cfg_attr(feature = "io", derive(Serialize, Deserialize))]
pub enum FunctionAttribute {
    Export,
    Job,
}

impl<'a> std::fmt::Display for FunctionAttribute {
    fn fmt(
        &self,
        writer: &mut std::fmt::Formatter<'_>,
    ) -> std::result::Result<(), std::fmt::Error> {
        match self {
            FunctionAttribute::Export => write!(writer, "export"),
            FunctionAttribute::Job => write!(writer, "job"),
        }
    }
}

pub type FunctionAttributes = EnumSet<FunctionAttribute>;

#[cfg_attr(feature = "io", derive(Serialize, Deserialize))]
pub(crate) struct FunctionPayload {
    pub(crate) module: Module,
    pub(crate) name: Name,
    pub(crate) function_type: Type,
    pub(crate) arguments: Vec<Value>,
    pub(crate) blocks: Vec<Block>,
    pub(crate) attributes: FunctionAttributes,
    pub(crate) location: Option<Location>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "io", derive(Serialize, Deserialize))]
pub struct Function(pub(crate) generational_arena::Index);

pub struct FunctionDisplayer<'a> {
    pub(crate) function: Function,
    pub(crate) context: &'a Context,
}

impl<'a> std::fmt::Display for FunctionDisplayer<'a> {
    fn fmt(
        &self,
        writer: &mut std::fmt::Formatter<'_>,
    ) -> std::result::Result<(), std::fmt::Error> {
        write!(writer, "fn ")?;

        let attributes = self.function.get_attributes(self.context);

        if !attributes.is_empty() {
            write!(writer, "[")?;

            let mut first = true;

            for attribute in attributes.iter() {
                if first {
                    first = false;
                } else {
                    write!(writer, ", ")?;
                }

                write!(writer, "{}", attribute)?;
            }

            write!(writer, "] ")?;
        }

        write!(
            writer,
            "{}(",
            self.function
                .get_name(self.context)
                .get_displayer(self.context)
        )?;

        for i in 0..self.function.get_num_args(self.context) {
            if i > 0 {
                writer.write_fmt(format_args!(", "))?;
            }

            let arg = self.function.get_arg(self.context, i);

            let arg_name = arg.get_name(self.context).get_displayer(self.context);
            let ty_name = arg.get_type(self.context).get_displayer(self.context);

            writer.write_fmt(format_args!("{} : {}", arg_name, ty_name))?;
        }

        let ret_ty_name = self
            .function
            .get_return_type(self.context)
            .get_displayer(self.context);
        let location = self.function.get_location(self.context);

        writer.write_fmt(format_args!(") : {}", ret_ty_name))?;

        if let Some(location) = location {
            write!(writer, "{}", location.get_displayer(self.context))?;
        }

        Ok(())
    }
}

impl Function {
    /// Get the module a function belongs to.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let original_module = context.create_module().build();
    /// # let function = original_module.create_function(&mut context).with_name("foo").build();
    /// let module = function.get_module(&context);
    /// # assert_eq!(original_module, module);
    /// ```
    pub fn get_module(&self, context: &Context) -> Module {
        let function = &context.functions[self.0];
        function.module
    }

    /// Get the name of the function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let function = module.create_function(&mut context).with_name("foo").build();
    /// let name = function.get_name(&context);
    /// # assert_eq!(name.as_str(&context), "foo");
    /// ```
    pub fn get_name(&self, context: &Context) -> Name {
        let function = &context.functions[self.0];
        function.name
    }

    /// Get the return type of the function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let void_ty = context.get_void_type();
    /// # let function = module.create_function(&mut context).with_name("foo").build();
    /// let return_type = function.get_return_type(&context);
    /// # assert_eq!(return_type, void_ty);
    /// ```
    pub fn get_return_type(&self, context: &Context) -> Type {
        let function = &context.functions[self.0];

        match context.types[function.function_type.0] {
            TypePayload::Function(return_type, _) => return_type,
            _ => panic!("Function type was wrong"),
        }
    }

    /// Get an argument from a function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let ty = context.get_int_type(8);
    /// # let function = module.create_function(&mut context).with_name("func").with_arg("arg", ty).build();
    /// let arg = function.get_arg(&context, 0);
    /// # assert_eq!(arg.get_type(&context), ty);
    /// ```
    pub fn get_arg(&self, context: &Context, index: usize) -> Value {
        let function = &context.functions[self.0];

        assert!(
            index < function.arguments.len(),
            "Argument index {} is invalid {}",
            index,
            function.arguments.len()
        );

        function.arguments[index]
    }

    /// Get the number of arguments a function has.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let ty = context.get_int_type(8);
    /// # let function = module.create_function(&mut context).with_name("func").with_arg("arg", ty).build();
    /// let num_args = function.get_num_args(&context);
    /// # assert_eq!(1, num_args);
    /// ```
    pub fn get_num_args(&self, context: &Context) -> usize {
        let function = &context.functions[self.0];

        function.arguments.len()
    }

    /// Check if a block is the entry block of a function.
    pub fn is_entry_block(&self, context: &Context, block: Block) -> bool {
        let function = &context.functions[self.0];

        if function.blocks.is_empty() {
            false
        } else {
            function.blocks[0] == block
        }
    }

    /// Create a new block in a function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let function = module.create_function(&mut context).with_name("func").build();;
    /// let block_builder = function.create_block(&mut context);
    /// ```
    pub fn create_block<'a>(&self, context: &'a mut Context) -> BlockBuilder<'a> {
        BlockBuilder::with_context_and_function(context, *self)
    }

    /// Get the attributes on the function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().with_name("module").build();
    /// # let function = module.create_function(&mut context).with_name("func").with_attributes(FunctionAttributes::only(FunctionAttribute::Export)).build();
    /// let attributes = function.get_attributes(&context);
    /// # assert!(attributes.contains(FunctionAttribute::Export));
    /// ```
    pub fn get_attributes(&self, context: &Context) -> FunctionAttributes {
        let function = &context.functions[self.0];

        function.attributes
    }

    /// Get all the blocks in a function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let function = module.create_function(&mut context).with_name("func").build();;
    /// let block_a = function.create_block(&mut context).build();
    /// let block_b = function.create_block(&mut context).build();
    /// let blocks = function.get_blocks(&context);
    /// assert_eq!(blocks.count(), 2);
    /// ```
    pub fn get_blocks(&self, context: &Context) -> BlockIterator {
        let function = &context.functions[self.0];
        BlockIterator::new(&function.blocks)
    }

    /// Get all the arguments in a function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let function = module.create_function(&mut context).with_name("func").with_arg("a", u32_ty).build();
    /// let mut args = function.get_args(&context);
    /// assert_eq!(args.nth(0).unwrap().get_type(&context), u32_ty);
    /// ```
    pub fn get_args(&self, context: &Context) -> ValueIterator {
        let function = &context.functions[self.0];
        ValueIterator::new(&function.arguments)
    }

    /// Get the location of a function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let function = module.create_function(&mut context).with_name("func").build();;
    /// let location = function.get_location(&context);
    /// # assert_eq!(None, location);
    /// # let location = context.get_location("foo.ya", 0, 13);
    /// # let function = module.create_function(&mut context).with_name("func").with_location(location).build();;
    /// # let location = function.get_location(&context);
    /// # assert!(location.is_some());
    /// ```
    pub fn get_location(&self, context: &Context) -> Option<Location> {
        let function = &context.functions[self.0];
        function.location
    }

    pub fn get_displayer<'a>(&self, context: &'a Context) -> FunctionDisplayer<'a> {
        FunctionDisplayer {
            function: *self,
            context,
        }
    }
}

impl Typed for Function {
    fn get_type(&self, context: &Context) -> Type {
        let function = &context.functions[self.0];
        function.function_type
    }
}

pub struct FunctionBuilder<'a> {
    context: &'a mut Context,
    module: Module,
    name: &'a str,
    return_type: Type,
    argument_names: Vec<&'a str>,
    argument_types: Vec<Type>,
    attributes: FunctionAttributes,
    location: Option<Location>,
}

impl<'a> FunctionBuilder<'a> {
    pub(crate) fn with_context_and_module(context: &'a mut Context, module: Module) -> Self {
        let void_ty = context.get_void_type();

        FunctionBuilder {
            context,
            module,
            name: "",
            return_type: void_ty,
            argument_names: Vec::new(),
            argument_types: Vec::new(),
            attributes: Default::default(),
            location: None,
        }
    }

    /// Add a name for the function to the builder.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let function_builder = module.create_function(&mut context);
    /// function_builder.with_name("func");
    /// ```
    pub fn with_name(mut self, name: &'a str) -> Self {
        self.name = name;
        self
    }

    /// Add a return type for the function.
    ///
    /// The default return type is void if none is specified.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let function_builder = module.create_function(&mut context);
    /// function_builder.with_return_type(u32_ty);
    /// ```
    pub fn with_return_type(mut self, return_type: Type) -> Self {
        self.return_type = return_type;
        self
    }

    /// Add the argument types to a function.
    ///
    /// The default is no argument types.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let i8_ty = context.get_int_type(8);
    /// # let u32_ty = context.get_uint_type(32);
    /// # let function_builder = module.create_function(&mut context);
    /// function_builder.with_arg("a", i8_ty).with_arg("b", u32_ty);
    /// ```
    pub fn with_arg(mut self, argument_name: &'a str, argument_type: Type) -> Self {
        self.argument_names.push(argument_name);
        self.argument_types.push(argument_type);
        self
    }

    /// Add many arguments to a function.
    ///
    /// The default is no argument types.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let i8_ty = context.get_int_type(8);
    /// # let u32_ty = context.get_uint_type(32);
    /// # let function_builder = module.create_function(&mut context);
    /// function_builder.with_args(&[("a", i8_ty), ("b", u32_ty)]);
    /// ```
    pub fn with_args(mut self, arguments: &[(&'a str, Type)]) -> Self {
        for argument in arguments {
            self.argument_names.push(argument.0);
            self.argument_types.push(argument.1);
        }
        self
    }

    /// Sets the attributes for the function. This unions in the attributes with
    /// any previously set attributes (allowing multiple calls to `with_attributes`)
    /// to add attributes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let builder = module.create_function(&mut context);
    /// builder.with_attributes(FunctionAttributes::only(FunctionAttribute::Export));
    /// ```
    pub fn with_attributes(mut self, attributes: FunctionAttributes) -> Self {
        self.attributes = self.attributes.union(attributes);
        self
    }

    /// Sets the location of the function.
    ///
    /// By default functions have no location.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let location = context.get_location("foo.ya", 0, 13);
    /// # let module = context.create_module().build();
    /// # let builder = module.create_function(&mut context);
    /// builder.with_location(location);
    /// ```
    pub fn with_location(mut self, loc: Location) -> Self {
        self.location = Some(loc);
        self
    }

    /// Finalize and build the function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let function_builder = module.create_function(&mut context).with_name("func");
    /// let function = function_builder.build();
    /// ```
    pub fn build(self) -> Function {
        debug_assert!(!self.name.is_empty(), "name must be non-0 in length");

        let function_type = self
            .context
            .get_function_type(self.return_type, &self.argument_types);

        let mut function = FunctionPayload {
            module: self.module,
            name: self.context.get_name(self.name),
            function_type,
            arguments: Vec::new(),
            blocks: Vec::new(),
            attributes: self.attributes,
            location: self.location,
        };

        for (argument_name, argument_type) in self.argument_names.iter().zip(self.argument_types) {
            let name = self.context.get_name(argument_name);

            let argument = self.context.values.insert(ValuePayload::Argument(Argument {
                name,
                ty: argument_type,
            }));
            function.arguments.push(Value(argument));
        }

        let func = Function(self.context.functions.insert(function));

        self.context.modules[self.module.0].functions.push(func);

        func
    }
}

pub struct BlockIterator {
    vec: Vec<Block>,
    next: usize,
}

impl BlockIterator {
    fn new(iter: &[Block]) -> BlockIterator {
        BlockIterator {
            vec: iter.to_vec(),
            next: 0,
        }
    }
}

impl Iterator for BlockIterator {
    type Item = Block;

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
