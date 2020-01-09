use crate::*;

#[derive(Serialize, Deserialize)]
pub struct FunctionPayload {
    pub(crate) name: String,
    pub(crate) function_type: Type,
    pub(crate) arguments: Vec<Value>,
    pub(crate) blocks: Vec<Block>,
    pub(crate) export: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
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
    /// # let void_ty = library.get_void_ty();
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
    /// # let ty = library.get_int_ty(8);
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
}

pub struct FunctionBuilder<'a> {
    library: &'a mut Library,
    module: Module,
    name: &'a str,
    return_type: Type,
    argument_names: Vec<&'a str>,
    argument_types: Vec<Type>,
    export: bool,
}

impl<'a> FunctionBuilder<'a> {
    pub(crate) fn with_library_and_module(library: &'a mut Library, module: Module) -> Self {
        let void_ty = library.get_void_ty();

        FunctionBuilder {
            library,
            module,
            name: "",
            return_type: void_ty,
            argument_names: Vec::new(),
            argument_types: Vec::new(),
            export: false,
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
    /// # let u32_ty = library.get_uint_ty(32);
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
    /// # let i8_ty = library.get_int_ty(8);
    /// # let u32_ty = library.get_uint_ty(32);
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
            .get_fn_type(self.return_type, &self.argument_types);

        let mut function = FunctionPayload {
            name: self.name.to_string(),
            function_type,
            arguments: Vec::new(),
            blocks: Vec::new(),
            export: self.export,
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
