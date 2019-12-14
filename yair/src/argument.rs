use crate::*;

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
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
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let int_ty = library.get_int_type(8);
    /// # let function = module.create_function(&mut library).with_name("func").with_argument_types(&[ int_ty ]).build();
    /// # let arg = function.get_arg(&library, 0);
    /// let ty = arg.get_type(&library);
    /// # assert_eq!(int_ty, ty);
    /// ```
    fn get_type(&self, _: &Library) -> Type {
        self.ty
    }
}
