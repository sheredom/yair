use crate::*;

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct Global {
    name: Name,
    ty: Type,
    export: bool,
}

impl Named for Global {
    /// Get the type of a global.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let global = module.create_global(&mut library).with_name("var").build();
    /// let ty = global.get_type(&library);
    /// # let void_ty = library.get_void_ty();
    /// # assert_eq!(ty, library.get_ptr_type(void_ty, Domain::CrossDevice));
    /// ```
    fn get_name<'a>(&self, library: &'a Library) -> &'a str {
        &library.names[self.name.0]
    }
}

impl Typed for Global {
    /// Get the type of a global.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let global = module.create_global(&mut library).with_name("var").build();
    /// let ty = global.get_type(&library);
    /// # let void_ty = library.get_void_ty();
    /// # assert_eq!(ty, library.get_ptr_type(void_ty, Domain::CrossDevice));
    /// ```
    fn get_type(&self, _: &Library) -> Type {
        self.ty
    }
}

pub struct GlobalBuilder<'a> {
    library: &'a mut Library,
    module: Module,
    name: &'a str,
    ty: Type,
    domain: Domain,
    export: bool,
}

impl<'a> GlobalBuilder<'a> {
    pub(crate) fn with_library_and_module(library: &'a mut Library, module: Module) -> Self {
        let void_ty = library.get_void_ty();
        GlobalBuilder {
            library,
            module,
            name: "",
            ty: void_ty,
            domain: Domain::CrossDevice,
            export: false,
        }
    }

    /// Add a name for the global to the builder.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let builder = module.create_global(&mut library);
    /// builder.with_name("var");
    /// ```
    pub fn with_name(mut self, name: &'a str) -> Self {
        self.name = name;
        self
    }

    /// Add a type for the global.
    ///
    /// The default type is void if none is specified.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let u32_ty = library.get_uint_ty(32);
    /// # let builder = module.create_global(&mut library);
    /// builder.with_type(u32_ty);
    /// ```
    pub fn with_type(mut self, ty: Type) -> Self {
        self.ty = ty;
        self
    }

    /// Add a domain for the global.
    ///
    /// The default domain is `CrossDevice` if none is specified.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let builder = module.create_global(&mut library);
    /// builder.with_domain(Domain::CPU);
    /// ```
    pub fn with_domain(mut self, domain: Domain) -> Self {
        self.domain = domain;
        self
    }

    /// Sets whether the variable is exported from the module or not.
    ///
    /// By default variables are not exported.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let builder = module.create_global(&mut library);
    /// builder.with_export(true);
    /// ```
    pub fn with_export(mut self, export: bool) -> Self {
        self.export = export;
        self
    }

    /// Finalize and build the global.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let builder = module.create_global(&mut library).with_name("var");
    /// let global = builder.build();
    /// ```
    pub fn build(self) -> Value {
        debug_assert!(!self.name.is_empty(), "name must be non-0 in length");

        let global_type = self.library.get_ptr_type(self.ty, self.domain);

        let global = Global {
            name: self.library.get_name(self.name),
            ty: global_type,
            export: self.export,
        };

        let index = Value(self.library.values.insert(ValuePayload::Global(global)));

        self.library.modules[self.module.0].globals.push(index);

        index
    }
}
