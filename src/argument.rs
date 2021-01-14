use crate::{Deserialize, Library, Name, Named, Serialize, Type, Typed};

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Argument {
    pub(crate) name: Name,
    pub(crate) ty: Type,
}

impl Named for Argument {
    /// Get the name of an argument.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let int_ty = library.get_int_ty(8);
    /// # let function = module.create_function(&mut library).with_name("func").with_argument("😀", int_ty).build();
    /// # let arg = function.get_arg(&library, 0);
    /// let name = arg.get_name(&library);
    /// # assert_eq!(name, "😀");
    /// ```
    fn get_name<'a>(&self, library: &'a Library) -> &'a str {
        &library.names[self.name.0]
    }
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
    /// # let int_ty = library.get_int_ty(8);
    /// # let function = module.create_function(&mut library).with_name("func").with_argument("😀", int_ty).build();
    /// # let arg = function.get_arg(&library, 0);
    /// let ty = arg.get_type(&library);
    /// # assert_eq!(int_ty, ty);
    /// ```
    fn get_type(&self, _: &Library) -> Type {
        self.ty
    }
}
