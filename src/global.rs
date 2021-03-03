use crate::*;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
#[cfg_attr(feature = "io", derive(Serialize, Deserialize))]
pub(crate) struct Global {
    name: Name,
    pub(crate) ty: Type,
    pub(crate) ptr_ty: Type,
    pub(crate) export: bool,
    pub(crate) location: Option<Location>,
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
    /// # let void_ty = library.get_void_type();
    /// # assert_eq!(ty, library.get_pointer_type(Domain::CrossDevice));
    /// ```
    fn get_name(&self, _: &Library) -> Name {
       self.name
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
    /// # let void_ty = library.get_void_type();
    /// # assert_eq!(ty, library.get_pointer_type(Domain::CrossDevice));
    /// ```
    fn get_type(&self, _: &Library) -> Type {
        self.ptr_ty
    }
}

pub struct GlobalBuilder<'a> {
    library: &'a mut Library,
    module: Module,
    name: &'a str,
    ty: Type,
    domain: Domain,
    export: bool,
    location: Option<Location>,
}

impl<'a> GlobalBuilder<'a> {
    pub(crate) fn with_library_and_module(library: &'a mut Library, module: Module) -> Self {
        let void_ty = library.get_void_type();
        GlobalBuilder {
            library,
            module,
            name: "",
            ty: void_ty,
            domain: Domain::CrossDevice,
            export: false,
            location: None,
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
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let u32_ty = library.get_uint_type(32);
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

    /// Sets the location of the variable.
    ///
    /// By default variables have no location.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let location = library.get_location("foo.ya", 0, 13);
    /// # let module = library.create_module().build();
    /// # let builder = module.create_global(&mut library);
    /// builder.with_location(location);
    /// ```
    pub fn with_location(mut self, loc: Location) -> Self {
        self.location = Some(loc);
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

        let global_type = self.library.get_pointer_type(self.domain);

        let global = Global {
            name: self.library.get_name(self.name),
            ty: self.ty,
            ptr_ty: global_type,
            export: self.export,
            location: self.location,
        };

        let index = Value(self.library.values.insert(ValuePayload::Global(global)));

        self.library.modules[self.module.0].globals.push(index);

        index
    }
}
