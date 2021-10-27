use crate::*;
use generational_arena::Arena;
use generational_arena::Index;
use radix_trie::Trie;
use std::collections::HashMap;

#[cfg_attr(feature = "io", derive(Serialize, Deserialize))]
pub struct Context {
    pub(crate) modules: Arena<ModulePayload>,
    pub(crate) types: Arena<TypePayload>,
    pub(crate) functions: Arena<FunctionPayload>,
    pub(crate) blocks: Arena<BlockPayload>,
    pub(crate) values: Arena<ValuePayload>,
    pub(crate) names: Arena<String>,
    name_lookups: Trie<String, Index>,
    void_ty: Option<Type>,
    bool_ty: Option<Type>,
    i8_ty: Option<Type>,
    i16_ty: Option<Type>,
    i32_ty: Option<Type>,
    i64_ty: Option<Type>,
    u8_ty: Option<Type>,
    u16_ty: Option<Type>,
    u32_ty: Option<Type>,
    u64_ty: Option<Type>,
    half_ty: Option<Type>,
    float_ty: Option<Type>,
    double_ty: Option<Type>,
    ptr_tys: HashMap<(Type, Domain), Type>,
    vec_tys: HashMap<(Type, u8), Type>,
    array_tys: HashMap<(Type, usize), Type>,
    struct_tys: HashMap<Vec<Type>, Type>,
    constants: HashMap<Constant, Value>,
    undefs: HashMap<Type, Value>,
}

impl Context {
    /// Create a new context.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// let context = Context::new();
    /// ```
    pub fn new() -> Context {
        Context {
            modules: Arena::new(),
            types: Arena::new(),
            functions: Arena::new(),
            blocks: Arena::new(),
            values: Arena::new(),
            names: Arena::new(),
            name_lookups: Trie::new(),
            void_ty: None,
            bool_ty: None,
            i8_ty: None,
            i16_ty: None,
            i32_ty: None,
            i64_ty: None,
            u8_ty: None,
            u16_ty: None,
            u32_ty: None,
            u64_ty: None,
            half_ty: None,
            float_ty: None,
            double_ty: None,
            ptr_tys: HashMap::new(),
            vec_tys: HashMap::new(),
            array_tys: HashMap::new(),
            struct_tys: HashMap::new(),
            constants: HashMap::new(),
            undefs: HashMap::new(),
        }
    }

    /// Create a new module.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// let module_builder = context.create_module();
    /// ```
    pub fn create_module(&mut self) -> ModuleBuilder {
        ModuleBuilder::with_context(self)
    }

    /// Get all the modules in the context.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// let module_a = context.create_module().with_name("a").build();
    /// let module_b = context.create_module().with_name("b").build();
    /// let mut modules = context.get_modules();
    /// assert_eq!(modules.nth(0).unwrap().get_name(&context).as_str(&context), "a");
    /// assert_eq!(modules.nth(0).unwrap().get_name(&context).as_str(&context), "b");
    /// ```
    pub fn get_modules(&self) -> ModuleIterator {
        ModuleIterator::new(&self.modules)
    }

    /// Get the void type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// let ty = context.get_void_type();
    /// ```
    pub fn get_void_type(&mut self) -> Type {
        match self.void_ty {
            Some(ty) => ty,
            None => {
                self.void_ty = Some(Type(self.types.insert(TypePayload::Void)));
                self.void_ty.unwrap()
            }
        }
    }

    /// Get the bool type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// let ty = context.get_bool_type();
    /// ```
    pub fn get_bool_type(&mut self) -> Type {
        match self.bool_ty {
            Some(ty) => ty,
            None => {
                self.bool_ty = Some(Type(self.types.insert(TypePayload::Bool)));
                self.bool_ty.unwrap()
            }
        }
    }

    /// Get a signed integer type.
    ///
    /// Restrictions:
    /// - `bits` must be 8, 16, 32, or 64.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// let i32_ty = context.get_int_type(32);
    /// ```
    pub fn get_int_type(&mut self, bits: u8) -> Type {
        let option = match bits {
            8 => &mut self.i8_ty,
            16 => &mut self.i16_ty,
            32 => &mut self.i32_ty,
            64 => &mut self.i64_ty,
            _ => panic!("Unsupported int type {}", bits),
        };

        match option {
            Some(ty) => *ty,
            None => {
                *option = Some(Type(self.types.insert(TypePayload::Int(bits))));
                option.unwrap()
            }
        }
    }

    /// Get an unsigned integer type.
    ///
    /// Restrictions:
    /// - `bits` must be 8, 16, 32, or 64.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// let u32_ty = context.get_uint_type(32);
    /// ```
    pub fn get_uint_type(&mut self, bits: u8) -> Type {
        let option = match bits {
            8 => &mut self.u8_ty,
            16 => &mut self.u16_ty,
            32 => &mut self.u32_ty,
            64 => &mut self.u64_ty,
            _ => panic!("Unsupported uint type {}", bits),
        };

        match option {
            Some(ty) => *ty,
            None => {
                *option = Some(Type(self.types.insert(TypePayload::UInt(bits))));
                option.unwrap()
            }
        }
    }

    /// Get a floating-point type.
    ///
    /// Restrictions:
    /// - `bits` must be 16, 32, or 64.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// let u32_ty = context.get_uint_type(32);
    /// ```
    pub fn get_float_type(&mut self, bits: u8) -> Type {
        let option = match bits {
            16 => &mut self.half_ty,
            32 => &mut self.float_ty,
            64 => &mut self.double_ty,
            _ => panic!("Unsupported float type {}", bits),
        };

        match option {
            Some(ty) => *ty,
            None => {
                *option = Some(Type(self.types.insert(TypePayload::Float(bits))));
                option.unwrap()
            }
        }
    }

    /// Get a vector type.
    ///
    /// Restrictions:
    /// - Vectors must have an element type of int or uint.
    /// - Vectors must have a width of 2, 3, 4, 8, 16, 32, 64, or 128.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// let vec_ty = context.get_vector_type(u32_ty, 8);
    /// ```
    pub fn get_vector_type(&mut self, element: Type, width: u8) -> Type {
        match &self.types[element.0] {
            TypePayload::Bool => (),
            TypePayload::Int(_) => (),
            TypePayload::UInt(_) => (),
            TypePayload::Float(_) => (),
            TypePayload::Pointer(_, _) => (),
            t => panic!("Unhandled element type for vector {:?}", t),
        }

        match width {
            2 => (),
            3 => (),
            4 => (),
            8 => (),
            16 => (),
            32 => (),
            64 => (),
            128 => (),
            _ => panic!("Unhandled vector type width {}", width),
        }

        match self.vec_tys.get(&(element, width)) {
            Some(ty) => *ty,
            None => {
                let ty = Type(self.types.insert(TypePayload::Vector(element, width)));
                self.vec_tys.insert((element, width), ty);
                ty
            }
        }
    }

    /// Get a pointer type in a given domain.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let ty = context.get_uint_type(32);
    /// let ptr_ty = context.get_pointer_type(ty, Domain::Cpu);
    /// # assert!(ptr_ty.is_pointer(&context));
    /// ```
    pub fn get_pointer_type(&mut self, element_ty: Type, domain: Domain) -> Type {
        match self.ptr_tys.get(&(element_ty, domain)) {
            Some(ty) => *ty,
            None => {
                let ty = Type(self.types.insert(TypePayload::Pointer(element_ty, domain)));
                self.ptr_tys.insert((element_ty, domain), ty);
                ty
            }
        }
    }

    /// Get a function type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let void_ty = context.get_void_type();
    /// # let i8_ty = context.get_int_type(8);
    /// # let u16_ty = context.get_uint_type(16);
    /// let func_ty = context.get_function_type(void_ty, &[i8_ty, u16_ty]);
    /// ```
    pub fn get_function_type(&mut self, return_type: Type, argument_types: &[Type]) -> Type {
        Type(
            self.types
                .insert(TypePayload::Function(return_type, argument_types.to_vec())),
        )
    }

    /// Get an array type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// let array_ty = context.get_array_type(u32_ty, 42);
    /// ```
    pub fn get_array_type(&mut self, element: Type, len: usize) -> Type {
        match self.array_tys.get(&(element, len)) {
            Some(ty) => *ty,
            None => {
                let ty = Type(self.types.insert(TypePayload::Array(element, len)));
                self.array_tys.insert((element, len), ty);
                ty
            }
        }
    }

    /// Get a struct type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let array_ty = context.get_array_type(u32_ty, 42);
    /// # let bool_ty = context.get_bool_type();
    /// let struct_ty = context.get_struct_type(&[ u32_ty, bool_ty, array_ty ]);
    /// # assert_eq!(struct_ty, context.get_struct_type(&[ u32_ty, bool_ty, array_ty ]));
    /// ```
    pub fn get_struct_type(&mut self, elements: &[Type]) -> Type {
        match self.struct_tys.get(&elements.to_vec()) {
            Some(ty) => *ty,
            None => {
                let ty = Type(self.types.insert(TypePayload::Struct(elements.to_vec())));
                self.struct_tys.insert(elements.to_vec(), ty);
                ty
            }
        }
    }

    pub(crate) fn get_name(&mut self, name: &str) -> Name {
        Name(match self.name_lookups.get(name) {
            Some(x) => *x,
            None => {
                let string = name.to_string();
                let index = self.names.insert(string.clone());
                self.name_lookups.insert(string, index);
                index
            }
        })
    }

    /// Get a location. A location consists of a filename, the line
    /// number, and the column number.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// let location = context.get_location("foo.ya", 0, 13);
    /// ```
    pub fn get_location(&mut self, filename: &str, line: usize, column: usize) -> Location {
        Location {
            filename: self.get_name(filename),
            line,
            column,
        }
    }

    /// Get an undef.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// let undef = context.get_undef(u32_ty);
    /// ```
    pub fn get_undef(&mut self, ty: Type) -> Value {
        match self.undefs.get(&ty) {
            Some(value) => *value,
            None => Value(self.values.insert(ValuePayload::Undef(ty))),
        }
    }

    /// Get a boolean constant.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// let constant = context.get_bool_constant(true);
    /// ```
    pub fn get_bool_constant(&mut self, b: bool) -> Value {
        let constant = Constant::Bool(b, self.get_bool_type());
        match self.constants.get(&constant) {
            Some(value) => *value,
            None => Value(self.values.insert(ValuePayload::Constant(constant))),
        }
    }

    /// Get a signed integer `bits` bit-width constant.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// let constant = context.get_int_constant(8, 42);
    /// ```
    pub fn get_int_constant(&mut self, bits: u8, cnst: i64) -> Value {
        let constant = Constant::Int(cnst, self.get_int_type(bits));
        match self.constants.get(&constant) {
            Some(value) => *value,
            None => Value(self.values.insert(ValuePayload::Constant(constant))),
        }
    }

    /// Get an unsigned integer `bits` bit-width constant.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// let constant = context.get_uint_constant(16, 42);
    /// ```
    pub fn get_uint_constant(&mut self, bits: u8, cnst: u64) -> Value {
        let constant = Constant::UInt(cnst, self.get_uint_type(bits));
        match self.constants.get(&constant) {
            Some(value) => *value,
            None => Value(self.values.insert(ValuePayload::Constant(constant))),
        }
    }

    /// Get a floating-point `bits` bit-width constant.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// let constant = context.get_float_constant(64, 42.0);
    /// ```
    pub fn get_float_constant(&mut self, bits: u8, cnst: f64) -> Value {
        let constant = Constant::Float(cnst, self.get_float_type(bits));
        match self.constants.get(&constant) {
            Some(value) => *value,
            None => Value(self.values.insert(ValuePayload::Constant(constant))),
        }
    }

    /// Get a null pointer constant.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let void_ty = context.get_void_type();
    /// # let ty = context.get_pointer_type(void_ty, Domain::Cpu);
    /// let constant = context.get_pointer_constant_null(ty);
    /// ```
    pub fn get_pointer_constant_null(&mut self, ty: Type) -> Value {
        let constant = Constant::Pointer(ty);
        match self.constants.get(&constant) {
            Some(value) => *value,
            None => Value(self.values.insert(ValuePayload::Constant(constant))),
        }
    }

    /// Get a composite constant (array, vector, or struct typed).
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let a = context.get_uint_constant(32, 13);
    /// # let b = context.get_uint_constant(32, 42);
    /// # let u32_ty = context.get_uint_type(32);
    /// # let ty = context.get_array_type(u32_ty, 2);
    /// let constant = context.get_composite_constant(ty, &[a, b]);
    /// ```
    pub fn get_composite_constant(&mut self, ty: Type, elems: &[Value]) -> Value {
        let constant = Constant::Composite(elems.to_vec(), ty);
        match self.constants.get(&constant) {
            Some(value) => *value,
            None => Value(self.values.insert(ValuePayload::Constant(constant))),
        }
    }

    /// Verify a library of modules.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// let result = context.verify();
    /// # assert!(result.is_ok());
    /// ```
    pub fn verify(&self) -> Result<(), VerifyError> {
        for module in self.get_modules() {
            module.verify(self)?;
        }

        Ok(())
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ModuleIterator {
    vec: Vec<Module>,
    next: usize,
}

impl ModuleIterator {
    fn new(iter: &generational_arena::Arena<ModulePayload>) -> ModuleIterator {
        let mut vec = Vec::new();

        for (index, _) in iter.iter() {
            vec.push(Module(index));
        }

        ModuleIterator { vec, next: 0 }
    }
}

impl Iterator for ModuleIterator {
    type Item = Module;

    fn next(&mut self) -> Option<Module> {
        if self.next < self.vec.len() {
            let next = self.next;
            self.next += 1;
            Some(self.vec[next])
        } else {
            None
        }
    }
}
