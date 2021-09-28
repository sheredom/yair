use crate::*;

#[cfg_attr(feature = "io", derive(Serialize, Deserialize))]
pub(crate) struct ModulePayload {
    pub(crate) name: Name,
    pub(crate) functions: Vec<Function>,
    pub(crate) globals: Vec<Value>,
    pub(crate) named_structs: Vec<Type>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "io", derive(Serialize, Deserialize))]
pub struct Module(pub(crate) generational_arena::Index);

impl Named for Module {
    fn get_name(&self, context: &Context) -> Name {
        context.modules[self.0].name
    }
}

impl Module {
    /// Create a new function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// let function_builder = module.create_function(&mut context);
    /// ```
    pub fn create_function<'a>(&self, context: &'a mut Context) -> FunctionBuilder<'a> {
        FunctionBuilder::with_context_and_module(context, *self)
    }

    /// Create a new global variable.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// let global_builder = module.create_global(&mut context);
    /// ```
    pub fn create_global<'a>(&self, context: &'a mut Context) -> GlobalBuilder<'a> {
        GlobalBuilder::with_context_and_module(context, *self)
    }

    /// Get a named struct type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let module = context.create_module().build();
    /// # let elements = vec![("my_field", u32_ty, None)];
    /// # let location = None;
    /// let struct_ty = module.create_named_struct_type(&mut context, "my_struct", &elements, location);
    /// ```
    pub fn create_named_struct_type(
        &self,
        context: &mut Context,
        name: &str,
        elements: &[(&str, Type, Option<Location>)],
        location: Option<Location>,
    ) -> Type {
        let name = context.get_name(name);

        let vec = elements
            .iter()
            .map(|(n, t, l)| (context.get_name(n), *t, *l))
            .collect();

        let ty = Type(
            context
                .types
                .insert(TypePayload::NamedStruct(*self, name, vec, location)),
        );

        context.modules[self.0].named_structs.push(ty);

        ty
    }

    // Get all named structs in a module.
    pub fn get_named_structs(&self, context: &Context) -> StructIterator {
        let module = &context.modules[self.0];
        StructIterator::new(&module.named_structs)
    }

    /// Get all the globals in a module.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().with_name("module").build();
    /// # let u32_ty = context.get_uint_type(32);
    /// let global_a = module.create_global(&mut context).with_name("a").with_type(u32_ty).build();
    /// let global_b = module.create_global(&mut context).with_name("b").with_type(u32_ty).build();
    /// let mut globals = module.get_globals(&context);
    /// assert_eq!(globals.nth(0).unwrap().get_name(&context).as_str(&context), "a");
    /// assert_eq!(globals.nth(0).unwrap().get_name(&context).as_str(&context), "b");
    /// ```
    pub fn get_globals(&self, context: &Context) -> GlobalIterator {
        let module = &context.modules[self.0];
        GlobalIterator::new(&module.globals)
    }

    /// Get all the functions in a module.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().with_name("module").build();
    /// let function_a = module.create_function(&mut context).with_name("a").build();
    /// let function_b = module.create_function(&mut context).with_name("b").build();
    /// let mut functions = module.get_functions(&context);
    /// assert_eq!(functions.nth(0).unwrap().get_name(&context).as_str(&context), "a");
    /// assert_eq!(functions.nth(0).unwrap().get_name(&context).as_str(&context), "b");
    /// ```
    pub fn get_functions(&self, context: &Context) -> FunctionIterator {
        let module = &context.modules[self.0];
        FunctionIterator::new(&module.functions)
    }

    /// Verify a context of modules.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().with_name("module").build();
    /// let result = module.verify(&context);
    /// # assert!(result.is_ok());
    /// ```
    pub fn verify<'a>(&self, context: &'a Context) -> Result<(), VerifyError<'a>> {
        verify(context, *self)
    }
}

pub struct ModuleBuilder<'a> {
    context: &'a mut Context,
    name: &'a str,
}

impl<'a> ModuleBuilder<'a> {
    pub(crate) fn with_context(context: &'a mut Context) -> ModuleBuilder {
        ModuleBuilder { context, name: "" }
    }

    /// Add a name for the module to the builder.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module_builder = context.create_module();
    /// module_builder.with_name("my module");
    /// ```
    pub fn with_name(mut self, name: &'a str) -> Self {
        self.name = name;
        self
    }

    /// Finalize and build the module.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module_builder = context.create_module();
    /// let module = module_builder.build();
    /// ```
    pub fn build(self) -> Module {
        let module = ModulePayload {
            name: self.context.get_name(self.name),
            functions: Vec::new(),
            globals: Vec::new(),
            named_structs: Vec::new(),
        };

        Module(self.context.modules.insert(module))
    }
}

pub struct StructIterator {
    vec: Vec<Type>,
    next: usize,
}

impl StructIterator {
    fn new(iter: &[Type]) -> StructIterator {
        StructIterator {
            vec: iter.to_vec(),
            next: 0,
        }
    }
}

impl Iterator for StructIterator {
    type Item = Type;

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

pub struct GlobalIterator {
    vec: Vec<Value>,
    next: usize,
}

impl GlobalIterator {
    fn new(iter: &[Value]) -> GlobalIterator {
        GlobalIterator {
            vec: iter.to_vec(),
            next: 0,
        }
    }
}

impl Iterator for GlobalIterator {
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

pub struct FunctionIterator {
    vec: Vec<Function>,
    next: usize,
}

impl FunctionIterator {
    fn new(iter: &[Function]) -> FunctionIterator {
        FunctionIterator {
            vec: iter.to_vec(),
            next: 0,
        }
    }
}

impl Iterator for FunctionIterator {
    type Item = Function;

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
