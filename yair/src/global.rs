use crate::*;

#[derive(Clone, Debug)]
pub struct Global {
    name: String,
    ty: Type,
}

impl Typed for Global {
    /// Get the type of a global.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let builder = module.create_global().with_name("var");
    /// # let global = builder.build();
    /// let ty = global.get_type(&module);
    /// # let void_ty = module.get_void_type();
    /// # assert_eq!(ty, module.get_ptr_type(void_ty, Domain::CPU_AND_GPU));
    /// ```
    fn get_type(&self, _: &Module) -> Type {
        self.ty
    }
}

pub struct GlobalBuilder<'a> {
    module: &'a mut Module,
    name: &'a str,
    ty: Type,
    domain: Domain,
}

impl<'a> GlobalBuilder<'a> {
    pub(crate) fn with_module(module: &'a mut Module) -> Self {
        let void_ty = module.get_void_type();
        GlobalBuilder {
            module,
            name: "",
            ty: void_ty,
            domain: Domain::CPU_AND_GPU,
        }
    }

    /// Add a name for the global to the builder.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let builder = module.create_global();
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
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let builder = module.create_global();
    /// builder.with_type(u32_ty);
    /// ```
    pub fn with_type(mut self, ty: Type) -> Self {
        self.ty = ty;
        self
    }

    /// Add a domain for the global.
    ///
    /// The default domain is `CPU_AND_GPU` if none is specified.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let builder = module.create_global();
    /// builder.with_domain(Domain::CPU);
    /// ```
    pub fn with_domain(mut self, domain: Domain) -> Self {
        self.domain = domain;
        self
    }

    /// Finalize and build the global.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let builder = module.create_global().with_name("var");
    /// let global = builder.build();
    /// ```
    pub fn build(self) -> Value {
        debug_assert!(!self.name.is_empty(), "name must be non-0 in length");

        let global_type = self.module.get_ptr_type(self.ty, self.domain);

        let global = Global {
            name: self.name.to_string(),
            ty: global_type,
        };

        Value(self.module.values.insert(ValuePayload::Global(global)))
    }
}
