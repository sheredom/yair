use crate::*;

#[derive(Clone, Debug)]
pub enum ValuePayload {
    Undef(Type),
    Argument(Argument),
    Instruction(Instruction),
    Constant(Constant),
    Global(Global),
}

#[derive(Clone, Copy, Debug, Hash, PartialEq)]
pub struct Value(pub(crate) generational_arena::Index);

impl Value {
    /// Return true if a value is a constant.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let value = module.get_bool_constant(true);
    /// let is_constant = value.is_constant(&module);
    /// # assert!(is_constant);
    /// ```
    pub fn is_constant(&self, module: &Module) -> bool {
        match module.values[self.0] {
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
    /// # let mut module = Module::create_module().build();
    /// # let value = module.get_bool_constant(true);
    /// let constant = value.get_constant(&module);
    /// # match constant {
    /// #     Constant::Bool(c, _) => assert!(c),
    /// #     _ => panic!("Bad constant"),
    /// # }
    /// ```
    pub fn get_constant<'a>(&self, module: &'a Module) -> &'a Constant {
        match &module.values[self.0] {
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
    /// # let mut module = Module::create_module().build();
    /// # let function_builder = module.create_function().with_name("func");
    /// # let function = function_builder.build();
    /// ```
    fn get_type(&self, module: &Module) -> Type {
        match &module.values[self.0] {
            ValuePayload::Undef(ty) => *ty,
            ValuePayload::Argument(arg) => arg.get_type(module),
            ValuePayload::Instruction(inst) => inst.get_type(module),
            ValuePayload::Constant(cnst) => cnst.get_type(module),
            ValuePayload::Global(glbl) => glbl.get_type(module),
        }
    }
}
