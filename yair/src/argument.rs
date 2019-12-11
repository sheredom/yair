use crate::*;

#[derive(Clone, Debug)]
pub struct Argument {
    pub(crate) ty: Type,
}

impl Typed for Argument {
    /// Get the type of an argument.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let int_ty = module.get_int_type(8);
    /// # let function = module.create_function().with_name("func").with_argument_types(&[ int_ty ]).build();
    /// # let arg = function.get_arg(&module, 0);
    /// let ty = arg.get_type(&module);
    /// # assert_eq!(int_ty, ty);
    /// ```
    fn get_type(&self, _: &Module) -> Type {
        self.ty
    }
}
