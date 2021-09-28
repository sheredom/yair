use crate::*;

#[derive(Debug, Eq, PartialEq)]
#[cfg_attr(feature = "io", derive(Serialize, Deserialize))]
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
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let int_ty = context.get_int_type(8);
    /// # let function = module.create_function(&mut context).with_name("func").with_arg("😀", int_ty).build();
    /// # let arg = function.get_arg(&context, 0);
    /// let name = arg.get_name(&context);
    /// # assert_eq!(name.as_str(&context), "😀");
    /// ```
    fn get_name(&self, _: &Context) -> Name {
        self.name
    }
}

impl Typed for Argument {
    /// Get the type of an argument.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let int_ty = context.get_int_type(8);
    /// # let function = module.create_function(&mut context).with_name("func").with_arg("😀", int_ty).build();
    /// # let arg = function.get_arg(&context, 0);
    /// let ty = arg.get_type(&context);
    /// # assert_eq!(int_ty, ty);
    /// ```
    fn get_type(&self, _: &Context) -> Type {
        self.ty
    }
}

pub struct ArgumentIterator<'a> {
    context: &'a mut Context,
    vec: Vec<Value>,
    next: usize,
}

impl<'a> Extend<(&'a str, Type)> for ArgumentIterator<'a> {
    fn extend<T: IntoIterator<Item = (&'a str, Type)>>(&mut self, iter: T) {
        for elem in iter {
            let name = self.context.get_name(elem.0);

            let argument = self
                .context
                .values
                .insert(ValuePayload::Argument(Argument { name, ty: elem.1 }));

            self.vec.push(Value(argument));
        }
    }
}

impl<'a> Iterator for ArgumentIterator<'a> {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next < self.vec.len() {
            let next = self.next;
            self.next += 1;
            Some(self.vec[next])
        } else {
            None
        }
    }
}

pub struct BlockArguments<'a> {
    context: &'a mut Context,
    block: Block,
}

impl<'a> BlockArguments<'a> {
    pub(crate) fn new(context: &'a mut Context, block: Block) -> Self {
        BlockArguments { context, block }
    }

    /// Push a new argument to the end of the argument list.
    pub fn push(&mut self, ty: Type) {
        let name = self.context.get_name("");

        let argument = self
            .context
            .values
            .insert(ValuePayload::Argument(Argument { name, ty }));

        self.context.blocks[self.block.0]
            .arguments
            .push(Value(argument));
    }
}
