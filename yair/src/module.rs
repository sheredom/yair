use crate::*;

#[derive(Serialize, Deserialize)]
pub(crate) struct ModulePayload {
    pub(crate) name: String,
    pub(crate) functions: Vec<Function>,
    pub(crate) globals: Vec<Value>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Module(pub(crate) generational_arena::Index);

impl Module {
    /// Get the name of the module.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().with_name("foo").build();
    /// let name = module.get_name(&library);
    /// # assert_eq!(name, "foo");
    /// ```
    pub fn get_name<'a>(&self, library: &'a Library) -> &'a str {
        &library.modules[self.0].name
    }

    /// Create a new function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// let function_builder = module.create_function(&mut library);
    /// ```
    pub fn create_function<'a>(&self, library: &'a mut Library) -> FunctionBuilder<'a> {
        FunctionBuilder::with_library_and_module(library, *self)
    }

    /// Create a new global variable.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// let global_builder = module.create_global(&mut library);
    /// ```
    pub fn create_global<'a>(&self, library: &'a mut Library) -> GlobalBuilder<'a> {
        GlobalBuilder::with_library_and_module(library, *self)
    }
}

pub struct ModuleBuilder<'a> {
    library: &'a mut Library,
    name: &'a str,
}

impl<'a> ModuleBuilder<'a> {
    pub(crate) fn with_library(library: &'a mut Library) -> ModuleBuilder {
        ModuleBuilder { library, name: "" }
    }

    /// Add a name for the module to the builder.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module_builder = library.create_module();
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
    /// # let mut library = Library::new();
    /// # let module_builder = library.create_module();
    /// let module = module_builder.build();
    /// ```
    pub fn build(self) -> Module {
        let module = ModulePayload {
            name: self.name.to_string(),
            functions: Vec::new(),
            globals: Vec::new(),
        };

        Module(self.library.modules.insert(module))
    }
}
