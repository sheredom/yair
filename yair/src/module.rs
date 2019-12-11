use crate::*;
use generational_arena::Arena;
use std::collections::HashMap;

pub struct Module {
    pub(crate) name: String,
    pub(crate) types: Arena<TypePayload>,
    pub(crate) functions: Arena<FunctionPayload>,
    pub(crate) blocks: Arena<BlockPayload>,
    pub(crate) values: Arena<ValuePayload>,
    filenames: Arena<String>,
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
}

impl Module {
    /// Create a new module.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// let module_builder = Module::create_module();
    /// ```
    pub fn create_module<'a>() -> ModuleBuilder<'a> {
        Default::default()
    }

    /// Get the name of the module.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let module = Module::create_module().with_name("foo").build();
    /// let name = module.get_name();
    /// # assert_eq!(name, "foo");
    /// ```
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Create a new function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// let function_builder = module.create_function();
    /// ```
    pub fn create_function(&mut self) -> FunctionBuilder {
        FunctionBuilder::with_module(self)
    }

    /// Create a new global variable.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// let global_builder = module.create_global();
    /// ```
    pub fn create_global(&mut self) -> GlobalBuilder {
        GlobalBuilder::with_module(self)
    }

    /// Get the void type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// let ty = module.get_void_type();
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
    /// # let mut module = Module::create_module().build();
    /// let ty = module.get_bool_ty();
    /// ```
    pub fn get_bool_ty(&mut self) -> Type {
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
    /// # let mut module = Module::create_module().build();
    /// let i32_ty = module.get_int_type(32);
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
    /// # let mut module = Module::create_module().build();
    /// let u32_ty = module.get_uint_type(32);
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
    /// # let mut module = Module::create_module().build();
    /// let u32_ty = module.get_uint_type(32);
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
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// let vec_ty = module.get_vec_type(u32_ty, 8);
    /// ```
    pub fn get_vec_type(&mut self, element: Type, width: u8) -> Type {
        match self.types[element.0] {
            TypePayload::Int(_) => (),
            TypePayload::UInt(_) => (),
            _ => panic!("Unhandled element type for vector"),
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
            None => Type(self.types.insert(TypePayload::Vector(element, width))),
        }
    }

    /// Get a pointer type in a given domain.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// let ptr_ty = module.get_ptr_type(u32_ty, Domain::CPU);
    /// ```
    pub fn get_ptr_type(&mut self, pointee: Type, domain: Domain) -> Type {
        match self.ptr_tys.get(&(pointee, domain)) {
            Some(ty) => *ty,
            None => {
                let ty = Type(self.types.insert(TypePayload::Pointer(pointee, domain)));
                self.ptr_tys.insert((pointee, domain), ty);
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
    /// # let mut module = Module::create_module().build();
    /// # let void_ty = module.get_void_type();
    /// # let i8_ty = module.get_int_type(8);
    /// # let u16_ty = module.get_uint_type(16);
    /// let func_ty = module.get_fn_type(void_ty, &[i8_ty, u16_ty]);
    /// ```
    pub fn get_fn_type(&mut self, return_type: Type, argument_types: &[Type]) -> Type {
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
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// let array_ty = module.get_array_ty(u32_ty, 42);
    /// ```
    pub fn get_array_ty(&mut self, element: Type, len: usize) -> Type {
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
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let array_ty = module.get_array_ty(u32_ty, 42);
    /// # let bool_ty = module.get_bool_ty();
    /// let struct_ty = module.get_struct_ty(&[ u32_ty, bool_ty, array_ty ]);
    /// # assert_eq!(struct_ty, module.get_struct_ty(&[ u32_ty, bool_ty, array_ty ]));
    /// ```
    pub fn get_struct_ty(&mut self, elements: &[Type]) -> Type {
        match self.struct_tys.get(&elements.to_vec()) {
            Some(ty) => *ty,
            None => {
                let ty = Type(self.types.insert(TypePayload::Struct(elements.to_vec())));
                self.struct_tys.insert(elements.to_vec(), ty);
                ty
            }
        }
    }

    /// Get a location. A location consists of a filename, the start position
    /// (line, column), and the end position (line, column).
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// let location = module.get_location("foo.ya", (0, 0), (0, 13));
    /// ```
    pub fn get_location(
        &mut self,
        filename: &str,
        start: (usize, usize),
        end: (usize, usize),
    ) -> Location {
        for (index, string) in self.filenames.iter() {
            if filename == string {
                return Location {
                    filename: Filename(index),
                    start,
                    end,
                };
            }
        }

        let index = self.filenames.insert(filename.to_string());

        Location {
            filename: Filename(index),
            start,
            end,
        }
    }

    /// Get a boolean constant.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// let constant = module.get_bool_constant(true);
    /// ```
    pub fn get_bool_constant(&mut self, b: bool) -> Value {
        let constant = Constant::Bool(b, self.get_bool_ty());
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
    /// # let mut module = Module::create_module().build();
    /// let constant = module.get_int_constant(8, 42);
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
    /// # let mut module = Module::create_module().build();
    /// let constant = module.get_uint_constant(16, 42);
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
    /// # let mut module = Module::create_module().build();
    /// let constant = module.get_float_constant(64, 42.0);
    /// ```
    pub fn get_float_constant(&mut self, bits: u8, cnst: f64) -> Value {
        let constant = Constant::Float(cnst, self.get_float_type(bits));
        match self.constants.get(&constant) {
            Some(value) => *value,
            None => Value(self.values.insert(ValuePayload::Constant(constant))),
        }
    }
}

#[derive(Default)]
pub struct ModuleBuilder<'a> {
    name: &'a str,
}

impl<'a> ModuleBuilder<'a> {
    /// Add a name for the module to the builder.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let module_builder = Module::create_module();
    /// module_builder.with_name("my module");
    /// ```
    pub fn with_name(mut self, name: &'a str) -> Self {
        self.name = name;
        self
    }

    /// Finalize and build the module.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let module_builder = Module::create_module();
    /// let module = module_builder.build();
    /// ```
    pub fn build(self) -> Module {
        Module {
            name: self.name.to_string(),
            types: Arena::new(),
            functions: Arena::new(),
            blocks: Arena::new(),
            values: Arena::new(),
            filenames: Arena::new(),
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
        }
    }
}
