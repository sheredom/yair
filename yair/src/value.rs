use crate::*;

#[derive(Debug, Serialize, Deserialize)]
pub enum ValuePayload {
    Undef(Type),
    Argument(Argument),
    Instruction(Instruction),
    Constant(Constant),
    Global(Global),
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Serialize, Deserialize)]
pub struct Value(pub(crate) generational_arena::Index);

impl Value {
    /// Return true if a value is a constant.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let value = library.get_bool_constant(true);
    /// let is_constant = value.is_constant(&library);
    /// # assert!(is_constant);
    /// ```
    pub fn is_constant(&self, library: &Library) -> bool {
        match library.values[self.0] {
            ValuePayload::Constant(_) => true,
            _ => false,
        }
    }

    /// If the value is a constant, get the constant, otherwise panic.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let value = library.get_bool_constant(true);
    /// let constant = value.get_constant(&library);
    /// # match constant {
    /// #     Constant::Bool(c, _) => assert!(c),
    /// #     _ => panic!("Bad constant"),
    /// # }
    /// ```
    pub fn get_constant<'a>(&self, library: &'a Library) -> &'a Constant {
        match &library.values[self.0] {
            ValuePayload::Constant(c) => &c,
            _ => panic!("Cannot get the constant from a non-constant value"),
        }
    }
}

impl Typed for Value {
    /// Get the type of a value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let function_builder = module.create_function(&mut library).with_name("func");
    /// # let function = function_builder.build();
    /// ```
    fn get_type(&self, library: &Library) -> Type {
        match &library.values[self.0] {
            ValuePayload::Undef(ty) => *ty,
            ValuePayload::Argument(arg) => arg.get_type(library),
            ValuePayload::Instruction(inst) => inst.get_type(library),
            ValuePayload::Constant(cnst) => cnst.get_type(library),
            ValuePayload::Global(glbl) => glbl.get_type(library),
        }
    }
}
