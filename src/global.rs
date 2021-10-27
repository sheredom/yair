use crate::*;

#[derive(EnumSetType, Debug)]
#[cfg_attr(feature = "io", derive(Serialize, Deserialize))]
pub enum GlobalAttribute {
    Export,
}

pub type GlobalAttributes = EnumSet<GlobalAttribute>;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
#[cfg_attr(feature = "io", derive(Serialize, Deserialize))]
pub(crate) struct Global {
    name: Name,
    pub(crate) ty: Type,
    pub(crate) ptr_ty: Type,
    pub(crate) attributes: GlobalAttributes,
    pub(crate) location: Option<Location>,
}

impl Named for Global {
    /// Get the name of a global.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let global = module.create_global(&mut context).with_name("var").build();
    /// let name = global.get_name(&context);
    /// # assert_eq!(name.as_str(&context), "var");
    /// ```
    fn get_name(&self, _: &Context) -> Name {
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
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let global = module.create_global(&mut context).with_name("var").build();
    /// let ty = global.get_type(&context);
    /// # let void_ty = context.get_void_type();
    /// # assert_eq!(ty, context.get_pointer_type(void_ty, Domain::CrossDevice));
    /// ```
    fn get_type(&self, _: &Context) -> Type {
        self.ptr_ty
    }
}

pub struct GlobalBuilder<'a> {
    context: &'a mut Context,
    module: Module,
    name: &'a str,
    ty: Type,
    domain: Domain,
    attributes: GlobalAttributes,
    location: Option<Location>,
}

impl<'a> GlobalBuilder<'a> {
    pub(crate) fn with_context_and_module(context: &'a mut Context, module: Module) -> Self {
        let void_ty = context.get_void_type();
        GlobalBuilder {
            context,
            module,
            name: "",
            ty: void_ty,
            domain: Domain::CrossDevice,
            attributes: Default::default(),
            location: None,
        }
    }

    /// Add a name for the global to the builder.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let builder = module.create_global(&mut context);
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
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let builder = module.create_global(&mut context);
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
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let builder = module.create_global(&mut context);
    /// builder.with_domain(Domain::Cpu);
    /// ```
    pub fn with_domain(mut self, domain: Domain) -> Self {
        self.domain = domain;
        self
    }

    /// Sets the attributes for the global. This unions in the attributes with
    /// any previously set attributes (allowing multiple calls to `with_attributes`)
    /// to add attributes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let builder = module.create_global(&mut context);
    /// builder.with_attributes(GlobalAttributes::only(GlobalAttribute::Export));
    /// ```
    pub fn with_attributes(mut self, attributes: GlobalAttributes) -> Self {
        self.attributes = self.attributes.union(attributes);
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
    /// # let mut context = Context::new();
    /// # let location = context.get_location("foo.ya", 0, 13);
    /// # let module = context.create_module().build();
    /// # let builder = module.create_global(&mut context);
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
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let builder = module.create_global(&mut context).with_name("var");
    /// let global = builder.build();
    /// ```
    pub fn build(self) -> Value {
        debug_assert!(!self.name.is_empty(), "name must be non-0 in length");

        let global_type = self.context.get_pointer_type(self.ty, self.domain);

        let global = Global {
            name: self.context.get_name(self.name),
            ty: self.ty,
            ptr_ty: global_type,
            attributes: self.attributes,
            location: self.location,
        };

        let index = Value(self.context.values.insert(ValuePayload::Global(global)));

        self.context.modules[self.module.0].globals.push(index);

        index
    }
}
