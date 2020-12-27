use crate::*;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum ValuePayload {
    Undef(Type),
    Argument(Argument),
    Instruction(Instruction),
    Constant(Constant),
    Global(Global),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct Value(pub(crate) generational_arena::Index);

impl UniqueIndex for Value {
    fn get_unique_index(&self) -> usize {
        self.0.into_raw_parts().0
    }
}

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
        matches!(library.values[self.0], ValuePayload::Constant(_))
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

    /// Return true if a value is an instruction.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let u32_ty = library.get_uint_ty(32);
    /// # let function = module.create_function(&mut library).with_name("func").with_return_type(u32_ty).build();
    /// # let _ = function.create_block(&mut library).build();
    /// # let block = function.create_block(&mut library).build();
    /// # let constant = library.get_uint_constant(32, 42);
    /// # let mut instruction_builder = block.create_instructions(&mut library);
    /// # let instruction = instruction_builder.ret_val(constant, None);
    /// let is_inst = instruction.is_inst(&library);
    /// # assert!(is_inst);
    /// ```
    pub fn is_inst(&self, library: &Library) -> bool {
        matches!(library.values[self.0], ValuePayload::Instruction(_))
    }

    /// If the value is an instruction, get the instruction, otherwise panic.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let u32_ty = library.get_uint_ty(32);
    /// # let function = module.create_function(&mut library).with_name("func").with_return_type(u32_ty).build();
    /// # let _ = function.create_block(&mut library).build();
    /// # let block = function.create_block(&mut library).build();
    /// # let constant = library.get_uint_constant(32, 42);
    /// # let mut instruction_builder = block.create_instructions(&mut library);
    /// # let value = instruction_builder.ret_val(constant, None);
    /// let instruction = value.get_inst(&library);
    /// ```
    pub fn get_inst<'a>(&self, library: &'a Library) -> &'a Instruction {
        match &library.values[self.0] {
            ValuePayload::Instruction(i) => &i,
            _ => panic!("Cannot get the instruction from a non-instruction value"),
        }
    }

    /// Return true if the value is a global export.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().with_name("module").build();
    /// # let u32_ty = library.get_uint_ty(32);
    /// let global = module.create_global(&mut library).with_export(true).with_name("global").with_type(u32_ty).build();
    /// let is_export = global.is_export(&library);
    /// # assert!(is_export);
    /// ```
    pub fn is_export(&self, library: &Library) -> bool {
        match &library.values[self.0] {
            ValuePayload::Global(g) => g.export,
            _ => false,
        }
    }
}

impl Named for Value {
    /// Get the name of a value.
    fn get_name<'a>(&self, library: &'a Library) -> &'a str {
        match &library.values[self.0] {
            ValuePayload::Undef(_) => panic!("Undef values cannot have names"),
            ValuePayload::Argument(arg) => arg.get_name(library),
            ValuePayload::Instruction(inst) => inst.get_name(library),
            ValuePayload::Constant(_) => panic!("Constants cannot have names"),
            ValuePayload::Global(glbl) => glbl.get_name(library),
        }
    }
}

impl Typed for Value {
    /// Get the type of a value.
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
