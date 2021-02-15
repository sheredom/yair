use crate::*;

#[cfg_attr(feature = "io", derive(Serialize, Deserialize))]
pub struct FunctionPayload {
    pub(crate) name: String,
    pub(crate) function_type: Type,
    pub(crate) arguments: Vec<Value>,
    pub(crate) blocks: Vec<Block>,
    pub(crate) export: bool,
    pub(crate) location: Option<Location>,
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "io", derive(Serialize, Deserialize))]
pub struct Function(pub(crate) generational_arena::Index);

impl Function {
    /// Get the name of the function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let function = module.create_function(&mut library).with_name("foo").build();
    /// let name = function.get_name(&library);
    /// # assert_eq!(name, "foo");
    /// ```
    pub fn get_name<'a>(&self, library: &'a Library) -> &'a str {
        let function = &library.functions[self.0];

        &function.name
    }

    /// Get the return type of the function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let void_ty = library.get_void_type();
    /// # let function = module.create_function(&mut library).with_name("foo").build();
    /// let return_type = function.get_return_type(&library);
    /// # assert_eq!(return_type, void_ty);
    /// ```
    pub fn get_return_type(&self, library: &Library) -> Type {
        let function = &library.functions[self.0];

        match library.types[function.function_type.0] {
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
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let ty = library.get_int_type(8);
    /// # let function = module.create_function(&mut library).with_name("func").with_argument("arg", ty).build();
    /// let arg = function.get_arg(&library, 0);
    /// # assert_eq!(arg.get_type(&library), ty);
    /// ```
    pub fn get_arg(&self, library: &Library, index: usize) -> Value {
        let function = &library.functions[self.0];

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
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let ty = library.get_int_type(8);
    /// # let function = module.create_function(&mut library).with_name("func").with_argument("arg", ty).build();
    /// let num_args = function.get_num_args(&library);
    /// # assert_eq!(1, num_args);
    /// ```
    pub fn get_num_args(&self, library: &Library) -> usize {
        let function = &library.functions[self.0];

        function.arguments.len()
    }

    /// Create a new block in a function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let function = module.create_function(&mut library).with_name("func").build();;
    /// let block_builder = function.create_block(&mut library);
    /// ```
    pub fn create_block<'a>(&self, library: &'a mut Library) -> BlockBuilder<'a> {
        BlockBuilder::with_library_and_function(library, *self)
    }

    /// Return true if the function is exported.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().with_name("module").build();
    /// # let function = module.create_function(&mut library).with_name("func").with_export(true).build();;
    /// let is_export = function.is_export(&library);
    /// # assert!(is_export);
    /// ```
    pub fn is_export(&self, library: &Library) -> bool {
        let function = &library.functions[self.0];

        function.export
    }

    /// Get all the blocks in a function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let function = module.create_function(&mut library).with_name("func").build();;
    /// let block_a = function.create_block(&mut library).build();
    /// let block_b = function.create_block(&mut library).build();
    /// let blocks = function.get_blocks(&library);
    /// assert_eq!(blocks.count(), 2);
    /// ```
    pub fn get_blocks(&self, library: &Library) -> BlockIterator {
        let function = &library.functions[self.0];
        BlockIterator::new(&function.blocks)
    }

    /// Get the location of a function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let function = module.create_function(&mut library).with_name("func").build();;
    /// let location = function.get_location(&library);
    /// # assert_eq!(None, location);
    /// # let location = library.get_location("foo.ya", 0, 13);
    /// # let function = module.create_function(&mut library).with_name("func").with_location(location).build();;
    /// # let location = function.get_location(&library);
    /// # assert!(location.is_some());
    /// ```
    pub fn get_location(&self, library: &Library) -> Option<Location> {
        let function = &library.functions[self.0];
        function.location
    }
}

pub struct FunctionBuilder<'a> {
    library: &'a mut Library,
    module: Module,
    name: &'a str,
    return_type: Type,
    argument_names: Vec<&'a str>,
    argument_types: Vec<Type>,
    export: bool,
    location: Option<Location>,
}

impl<'a> FunctionBuilder<'a> {
    pub(crate) fn with_library_and_module(library: &'a mut Library, module: Module) -> Self {
        let void_ty = library.get_void_type();

        FunctionBuilder {
            library,
            module,
            name: "",
            return_type: void_ty,
            argument_names: Vec::new(),
            argument_types: Vec::new(),
            export: false,
            location: None,
        }
    }

    /// Add a name for the function to the builder.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let function_builder = module.create_function(&mut library);
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
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let u32_ty = library.get_uint_type(32);
    /// # let function_builder = module.create_function(&mut library);
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
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let i8_ty = library.get_int_type(8);
    /// # let u32_ty = library.get_uint_type(32);
    /// # let function_builder = module.create_function(&mut library);
    /// function_builder.with_argument("a", i8_ty).with_argument("b", u32_ty);
    /// ```
    pub fn with_argument(mut self, argument_name: &'a str, argument_type: Type) -> Self {
        self.argument_names.push(argument_name);
        self.argument_types.push(argument_type);
        self
    }

    /// Sets whether the function is exported from the module or not.
    ///
    /// By default functions are not exported.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let builder = module.create_function(&mut library);
    /// builder.with_export(true);
    /// ```
    pub fn with_export(mut self, export: bool) -> Self {
        self.export = export;
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
    /// # let mut library = Library::new();
    /// # let location = library.get_location("foo.ya", 0, 13);
    /// # let module = library.create_module().build();
    /// # let builder = module.create_function(&mut library);
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
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let function_builder = module.create_function(&mut library).with_name("func");
    /// let function = function_builder.build();
    /// ```
    pub fn build(self) -> Function {
        debug_assert!(!self.name.is_empty(), "name must be non-0 in length");

        let function_type = self
            .library
            .get_function_type(self.return_type, &self.argument_types);

        let mut function = FunctionPayload {
            name: self.name.to_string(),
            function_type,
            arguments: Vec::new(),
            blocks: Vec::new(),
            export: self.export,
            location: self.location,
        };

        for (argument_name, argument_type) in self.argument_names.iter().zip(self.argument_types) {
            let name = self.library.get_name(argument_name);

            let argument = self.library.values.insert(ValuePayload::Argument(Argument {
                name,
                ty: argument_type,
            }));
            function.arguments.push(Value(argument));
        }

        let func = Function(self.library.functions.insert(function));

        self.library.modules[self.module.0].functions.push(func);

        func
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
        let _ = function.get_arg(&library, 0);
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
