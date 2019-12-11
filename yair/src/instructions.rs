use crate::*;

#[derive(Clone, Debug)]
pub enum InstructionPayload {
    Return,
    ReturnValue(Value),
    Add(Value, Value),
    Sub(Value, Value),
    Mul(Value, Value),
    Div(Value, Value),
    Rem(Value, Value),
    Neg(Value),
    And(Value, Value),
    Or(Value, Value),
    Xor(Value, Value),
    Not(Value),
    Shl(Value, Value),
    Shr(Value, Value),
    Cast(Value, Type),
    BitCast(Value, Type),
    Load(Value),
    Store(Value, Value),
    Extract(Value, usize),
    Insert(Value, Value, usize),
    CmpEqual(Value, Value, Type),
    CmpNotEqual(Value, Value, Type),
    CmpLessThan(Value, Value, Type),
    CmpLessThanEqual(Value, Value, Type),
    CmpGreaterThan(Value, Value, Type),
    CmpGreaterThanEqual(Value, Value, Type),
    StackAlloc(Type),
    Call(Function, Vec<Value>),
    Branch(Block, Vec<Value>),
    ConditionalBranch(Value, Block, Vec<Value>),
    Select(Value, Value, Value),
    GetElementPtr(Value, Vec<Value>, Type),
    // select
}

#[derive(Clone, Debug)]
pub struct Instruction {
    pub(crate) location: Option<Location>,
    pub(crate) payload: InstructionPayload,
}

impl Typed for Instruction {
    /// Get the type of an instruction.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let function = module.create_function().with_name("func").build();
    /// # let _ = function.create_block(&mut module).build();
    /// # let block = function.create_block(&mut module).with_argument_types(&[ u32_ty ]).build();
    /// # let arg = block.get_arg(&module, 0);
    /// let ty = arg.get_type(&module);
    /// ```
    fn get_type(&self, module: &Module) -> Type {
        match self.payload {
            InstructionPayload::Return => panic!("Cannot get the type of a void return"),
            InstructionPayload::ReturnValue(val) => val.get_type(module),
            InstructionPayload::Add(x, _) => x.get_type(module),
            InstructionPayload::Sub(x, _) => x.get_type(module),
            InstructionPayload::Mul(x, _) => x.get_type(module),
            InstructionPayload::Div(x, _) => x.get_type(module),
            InstructionPayload::Rem(x, _) => x.get_type(module),
            InstructionPayload::Neg(x) => x.get_type(module),
            InstructionPayload::And(x, _) => x.get_type(module),
            InstructionPayload::Or(x, _) => x.get_type(module),
            InstructionPayload::Xor(x, _) => x.get_type(module),
            InstructionPayload::Not(x) => x.get_type(module),
            InstructionPayload::Cast(_, ty) => ty,
            InstructionPayload::BitCast(_, ty) => ty,
            InstructionPayload::Load(ptr) => ptr.get_type(module).get_pointee(module),
            InstructionPayload::Store(_, _) => panic!("Cannot get the type of a store"),
            InstructionPayload::Extract(val, index) => {
                val.get_type(module).get_element(module, index)
            }
            InstructionPayload::Insert(_, val, _) => val.get_type(module),
            InstructionPayload::Shl(x, _) => x.get_type(module),
            InstructionPayload::Shr(x, _) => x.get_type(module),
            InstructionPayload::CmpEqual(_, _, ty) => ty,
            InstructionPayload::CmpNotEqual(_, _, ty) => ty,
            InstructionPayload::CmpLessThan(_, _, ty) => ty,
            InstructionPayload::CmpLessThanEqual(_, _, ty) => ty,
            InstructionPayload::CmpGreaterThan(_, _, ty) => ty,
            InstructionPayload::CmpGreaterThanEqual(_, _, ty) => ty,
            InstructionPayload::StackAlloc(ty) => ty,
            InstructionPayload::Call(function, _) => function.get_return_type(module),
            InstructionPayload::Branch(_, _) => panic!("Cannot get the type of a branch"),
            InstructionPayload::ConditionalBranch(_, _, _) => {
                panic!("Cannot get the type of a conditional branch")
            }
            InstructionPayload::Select(_, x, _) => x.get_type(module),
            InstructionPayload::GetElementPtr(_, _, ty) => ty,
        }
    }
}

pub struct InstructionBuilder<'a> {
    module: &'a mut Module,
    block: Block,
}

impl<'a> InstructionBuilder<'a> {
    pub(crate) fn with_module_and_block(module: &'a mut Module, block: Block) -> Self {
        InstructionBuilder { module, block }
    }

    fn make_value(&mut self, payload: InstructionPayload, location: Option<Location>) -> Value {
        let instruction = Instruction { location, payload };
        let index = self
            .module
            .values
            .insert(ValuePayload::Instruction(instruction));
        self.module.blocks[self.block.0]
            .instructions
            .push(Value(index));
        Value(index)
    }

    /// Record a return from the function which closes the block.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let function = module.create_function().with_name("func").build();
    /// # let block = function.create_block(&mut module).build();
    /// # let mut instruction_builder = block.create_instructions(&mut module);
    /// # let location = None;
    /// instruction_builder.ret(location);
    /// ```
    pub fn ret(mut self, location: Option<Location>) {
        self.make_value(InstructionPayload::Return, location);
    }

    /// Record a return from the function which closes the block.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let function = module.create_function().with_name("func").with_return_type(u32_ty).with_argument_types(&[ u32_ty ]).build();
    /// # let block = function.create_block(&mut module).build();
    /// # let return_value = function.get_arg(&module, 0);
    /// # let mut instruction_builder = block.create_instructions(&mut module);
    /// # let location = None;
    /// instruction_builder.ret_val(return_value, location);
    /// ```
    pub fn ret_val(mut self, value: Value, location: Option<Location>) {
        self.make_value(InstructionPayload::ReturnValue(value), location);
    }

    /// Record an addition instruction in the block.
    ///
    /// Restrictions:
    /// - `x` and `y` must have the same type, which matches the type of the newly returned value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let function = module.create_function().with_name("func").with_return_type(u32_ty).with_argument_types(&[ u32_ty, u32_ty ]).build();
    /// # let block = function.create_block(&mut module).build();
    /// # let x = function.get_arg(&module, 0);
    /// # let y = function.get_arg(&module, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut module);
    /// # let location = None;
    /// let add = instruction_builder.add(x, y, location);
    /// ```
    pub fn add(&mut self, x: Value, y: Value, location: Option<Location>) -> Value {
        assert_eq!(x.get_type(self.module), y.get_type(self.module));
        self.make_value(InstructionPayload::Add(x, y), location)
    }

    /// Record a subtract instruction in the block.
    ///
    /// Restrictions:
    /// - `x` and `y` must have the same type, which matches the type of the newly returned value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let function = module.create_function().with_name("func").with_return_type(u32_ty).with_argument_types(&[ u32_ty, u32_ty ]).build();
    /// # let block = function.create_block(&mut module).build();
    /// # let x = function.get_arg(&module, 0);
    /// # let y = function.get_arg(&module, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut module);
    /// # let location = None;
    /// let sub = instruction_builder.sub(x, y, location);
    /// ```
    pub fn sub(&mut self, x: Value, y: Value, location: Option<Location>) -> Value {
        assert_eq!(x.get_type(self.module), y.get_type(self.module));
        self.make_value(InstructionPayload::Sub(x, y), location)
    }

    /// Record a multiply instruction in the block.
    ///
    /// Restrictions:
    /// - `x` and `y` must have the same type, which matches the type of the newly returned value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let function = module.create_function().with_name("func").with_return_type(u32_ty).with_argument_types(&[ u32_ty, u32_ty ]).build();
    /// # let block = function.create_block(&mut module).build();
    /// # let x = function.get_arg(&module, 0);
    /// # let y = function.get_arg(&module, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut module);
    /// # let location = None;
    /// let mul = instruction_builder.mul(x, y, location);
    /// ```
    pub fn mul(&mut self, x: Value, y: Value, location: Option<Location>) -> Value {
        assert_eq!(x.get_type(self.module), y.get_type(self.module));
        self.make_value(InstructionPayload::Mul(x, y), location)
    }

    /// Record a divide instruction in the block.
    ///
    /// Restrictions:
    /// - `x` and `y` must have the same type, which matches the type of the newly returned value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let function = module.create_function().with_name("func").with_return_type(u32_ty).with_argument_types(&[ u32_ty, u32_ty ]).build();
    /// # let block = function.create_block(&mut module).build();
    /// # let x = function.get_arg(&module, 0);
    /// # let y = function.get_arg(&module, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut module);
    /// # let location = None;
    /// let div = instruction_builder.div(x, y, location);
    /// ```
    pub fn div(&mut self, x: Value, y: Value, location: Option<Location>) -> Value {
        assert_eq!(x.get_type(self.module), y.get_type(self.module));
        self.make_value(InstructionPayload::Div(x, y), location)
    }

    /// Negate a value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let function = module.create_function().with_name("func").with_return_type(u32_ty).with_argument_types(&[ u32_ty ]).build();
    /// # let block = function.create_block(&mut module).build();
    /// # let x = function.get_arg(&module, 0);
    /// # let mut instruction_builder = block.create_instructions(&mut module);
    /// # let location = None;
    /// let div = instruction_builder.neg(x, location);
    /// ```
    pub fn neg(&mut self, x: Value, location: Option<Location>) -> Value {
        self.make_value(InstructionPayload::Neg(x), location)
    }

    /// Record a bitwise and instruction in the block.
    ///
    /// Restrictions:
    /// - `x` must be an integer or vector-of-integer type.
    /// - `x` and `y` must have the same type, which matches the type of the newly returned value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let function = module.create_function().with_name("func").with_return_type(u32_ty).with_argument_types(&[ u32_ty, u32_ty ]).build();
    /// # let block = function.create_block(&mut module).build();
    /// # let x = function.get_arg(&module, 0);
    /// # let y = function.get_arg(&module, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut module);
    /// # let location = None;
    /// let and = instruction_builder.and(x, y, location);
    /// ```
    pub fn and(&mut self, x: Value, y: Value, location: Option<Location>) -> Value {
        assert!(x
            .get_type(self.module)
            .is_integral_or_integral_vector(self.module));
        assert_eq!(x.get_type(self.module), y.get_type(self.module));
        self.make_value(InstructionPayload::And(x, y), location)
    }

    /// Record a bitwise or instruction in the block.
    ///
    /// Restrictions:
    /// - `x` must be an integer or vector-of-integer type.
    /// - `x` and `y` must have the same type, which matches the type of the newly returned value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let function = module.create_function().with_name("func").with_return_type(u32_ty).with_argument_types(&[ u32_ty, u32_ty ]).build();
    /// # let block = function.create_block(&mut module).build();
    /// # let x = function.get_arg(&module, 0);
    /// # let y = function.get_arg(&module, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut module);
    /// # let location = None;
    /// let or = instruction_builder.or(x, y, location);
    /// ```
    pub fn or(&mut self, x: Value, y: Value, location: Option<Location>) -> Value {
        assert!(x
            .get_type(self.module)
            .is_integral_or_integral_vector(self.module));
        assert_eq!(x.get_type(self.module), y.get_type(self.module));
        self.make_value(InstructionPayload::Or(x, y), location)
    }

    /// Record a bitwise xor instruction in the block.
    ///
    /// Restrictions:
    /// - `x` must be an integer or vector-of-integer type.
    /// - `x` and `y` must have the same type, which matches the type of the newly returned value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let function = module.create_function().with_name("func").with_return_type(u32_ty).with_argument_types(&[ u32_ty, u32_ty ]).build();
    /// # let block = function.create_block(&mut module).build();
    /// # let x = function.get_arg(&module, 0);
    /// # let y = function.get_arg(&module, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut module);
    /// # let location = None;
    /// let xor = instruction_builder.xor(x, y, location);
    /// ```
    pub fn xor(&mut self, x: Value, y: Value, location: Option<Location>) -> Value {
        assert!(x
            .get_type(self.module)
            .is_integral_or_integral_vector(self.module));
        assert_eq!(x.get_type(self.module), y.get_type(self.module));
        self.make_value(InstructionPayload::Xor(x, y), location)
    }

    /// Record a bitwise not instruction in the block.
    ///
    /// Restrictions:
    /// - `x` must be an integer or vector-of-integer type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let function = module.create_function().with_name("func").with_return_type(u32_ty).with_argument_types(&[ u32_ty ]).build();
    /// # let block = function.create_block(&mut module).build();
    /// # let x = function.get_arg(&module, 0);
    /// # let mut instruction_builder = block.create_instructions(&mut module);
    /// # let location = None;
    /// let not = instruction_builder.not(x, location);
    /// ```
    pub fn not(&mut self, x: Value, location: Option<Location>) -> Value {
        assert!(x
            .get_type(self.module)
            .is_integral_or_integral_vector(self.module));
        self.make_value(InstructionPayload::Not(x), location)
    }

    /// Record a shift left instruction in the block.
    ///
    /// Restrictions:
    /// - `x` and `y` must have the same type, which matches the type of the newly returned value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let function = module.create_function().with_name("func").with_return_type(u32_ty).with_argument_types(&[ u32_ty, u32_ty ]).build();
    /// # let block = function.create_block(&mut module).build();
    /// # let x = function.get_arg(&module, 0);
    /// # let y = function.get_arg(&module, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut module);
    /// # let location = None;
    /// let shl = instruction_builder.shl(x, y, location);
    /// ```
    pub fn shl(&mut self, x: Value, y: Value, location: Option<Location>) -> Value {
        assert_eq!(x.get_type(self.module), y.get_type(self.module));
        self.make_value(InstructionPayload::Shl(x, y), location)
    }

    /// Record a shift right instruction in the block.
    ///
    /// Restrictions:
    /// - `x` and `y` must have the same type, which matches the type of the newly returned value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let function = module.create_function().with_name("func").with_return_type(u32_ty).with_argument_types(&[ u32_ty, u32_ty ]).build();
    /// # let block = function.create_block(&mut module).build();
    /// # let x = function.get_arg(&module, 0);
    /// # let y = function.get_arg(&module, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut module);
    /// # let location = None;
    /// let shr = instruction_builder.shr(x, y, location);
    /// ```
    pub fn shr(&mut self, x: Value, y: Value, location: Option<Location>) -> Value {
        assert_eq!(x.get_type(self.module), y.get_type(self.module));
        self.make_value(InstructionPayload::Shr(x, y), location)
    }

    /// Cast a value to another type.
    ///
    /// Restrictions:
    /// - You cannot cast a value to the same type as it already has.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let i8_ty = module.get_int_type(8);
    /// # let function = module.create_function().with_name("func").with_return_type(u32_ty).with_argument_types(&[ u32_ty ]).build();
    /// # let block = function.create_block(&mut module).build();
    /// # let x = function.get_arg(&module, 0);
    /// # let mut instruction_builder = block.create_instructions(&mut module);
    /// # let location = None;
    /// let cast = instruction_builder.cast(x, i8_ty, location);
    /// ```
    pub fn cast(&mut self, x: Value, ty: Type, location: Option<Location>) -> Value {
        assert_ne!(x.get_type(self.module), ty);
        self.make_value(InstructionPayload::Cast(x, ty), location)
    }

    /// Bitcast a value to another type.
    ///
    /// Restrictions:
    /// - You cannot cast a value to the same type as it already has.
    /// - The type you are casting to must have the same bit-width as the value being casted.
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let i32_ty = module.get_int_type(32);
    /// # let function = module.create_function().with_name("func").with_return_type(u32_ty).with_argument_types(&[ u32_ty ]).build();
    /// # let block = function.create_block(&mut module).build();
    /// # let x = function.get_arg(&module, 0);
    /// # let mut instruction_builder = block.create_instructions(&mut module);
    /// # let location = None;
    /// let cast = instruction_builder.bitcast(x, i32_ty, location);
    /// ```
    pub fn bitcast(&mut self, x: Value, ty: Type, location: Option<Location>) -> Value {
        let x_ty = x.get_type(self.module);
        assert_ne!(x_ty, ty);
        assert_eq!(x_ty.get_bits(self.module), ty.get_bits(self.module));
        self.make_value(InstructionPayload::BitCast(x, ty), location)
    }

    /// Load a value from a pointer.
    ///
    /// Restrictions:
    /// - `ptr` must be of pointer type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let u32_ptr_ty = module.get_ptr_type(u32_ty, Domain::CPU);
    /// # let function = module.create_function().with_name("func").with_return_type(u32_ty).with_argument_types(&[ u32_ptr_ty ]).build();
    /// # let block = function.create_block(&mut module).build();
    /// # let ptr = function.get_arg(&module, 0);
    /// # let mut instruction_builder = block.create_instructions(&mut module);
    /// # let location = None;
    /// let load = instruction_builder.load(ptr, location);
    /// ```
    pub fn load(&mut self, ptr: Value, location: Option<Location>) -> Value {
        assert!(ptr.get_type(self.module).is_ptr(self.module));
        self.make_value(InstructionPayload::Load(ptr), location)
    }

    /// Store a value to a pointer.
    ///
    /// Restrictions:
    /// - `ptr` must be of pointer type.
    /// - The pointee type of `ptr` must be the same as the type of `val`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let void_ty = module.get_void_type();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let u32_ptr_ty = module.get_ptr_type(u32_ty, Domain::CPU);
    /// # let function = module.create_function().with_name("func").with_return_type(void_ty).with_argument_types(&[ u32_ty, u32_ptr_ty ]).build();
    /// # let block = function.create_block(&mut module).build();
    /// # let val = function.get_arg(&module, 0);
    /// # let ptr = function.get_arg(&module, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut module);
    /// # let location = None;
    /// let store = instruction_builder.store(ptr, val, location);
    /// ```
    pub fn store(&mut self, ptr: Value, val: Value, location: Option<Location>) -> Value {
        assert!(ptr.get_type(self.module).is_ptr(self.module));
        assert_eq!(
            ptr.get_type(self.module).get_pointee(self.module),
            val.get_type(self.module)
        );
        self.make_value(InstructionPayload::Store(ptr, val), location)
    }

    /// Extract an element from a vector or struct type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let vec_ty = module.get_vec_type(u32_ty, 4);
    /// # let function = module.create_function().with_name("func").with_return_type(u32_ty).with_argument_types(&[ vec_ty ]).build();
    /// # let block = function.create_block(&mut module).build();
    /// # let val = function.get_arg(&module, 0);
    /// # let mut instruction_builder = block.create_instructions(&mut module);
    /// # let location = None;
    /// let extract = instruction_builder.extract(val, 0, location);
    /// ```
    pub fn extract(&mut self, val: Value, idx: usize, location: Option<Location>) -> Value {
        let ty = val.get_type(self.module);
        assert!(ty.is_vector(self.module));
        self.make_value(InstructionPayload::Extract(val, idx), location)
    }

    /// Insert an element into a vector or struct type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let vec_ty = module.get_vec_type(u32_ty, 4);
    /// # let function = module.create_function().with_name("func").with_return_type(u32_ty).with_argument_types(&[ vec_ty, u32_ty ]).build();
    /// # let block = function.create_block(&mut module).build();
    /// # let val = function.get_arg(&module, 0);
    /// # let element = function.get_arg(&module, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut module);
    /// # let location = None;
    /// let insert = instruction_builder.insert(val, element, 1, location);
    /// ```
    pub fn insert(
        &mut self,
        val: Value,
        element: Value,
        idx: usize,
        location: Option<Location>,
    ) -> Value {
        assert_eq!(
            val.get_type(self.module).get_element(self.module, idx),
            element.get_type(self.module)
        );
        self.make_value(InstructionPayload::Insert(val, element, idx), location)
    }

    /// Record a compare equal instruction in the block.
    ///
    /// Restrictions:
    /// - `x` and `y` must have the same type, which matches the type of the newly returned value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let function = module.create_function().with_name("func").with_return_type(u32_ty).with_argument_types(&[ u32_ty, u32_ty ]).build();
    /// # let block = function.create_block(&mut module).build();
    /// # let x = function.get_arg(&module, 0);
    /// # let y = function.get_arg(&module, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut module);
    /// # let location = None;
    /// let cmp_eq = instruction_builder.cmp_eq(x, y, location);
    /// ```
    pub fn cmp_eq(&mut self, x: Value, y: Value, location: Option<Location>) -> Value {
        assert_eq!(x.get_type(self.module), y.get_type(self.module));
        let bool_ty = self.module.get_bool_ty();
        self.make_value(InstructionPayload::CmpEqual(x, y, bool_ty), location)
    }

    /// Record a compare not-equal instruction in the block.
    ///
    /// Restrictions:
    /// - `x` and `y` must have the same type, which matches the type of the newly returned value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let function = module.create_function().with_name("func").with_return_type(u32_ty).with_argument_types(&[ u32_ty, u32_ty ]).build();
    /// # let block = function.create_block(&mut module).build();
    /// # let x = function.get_arg(&module, 0);
    /// # let y = function.get_arg(&module, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut module);
    /// # let location = None;
    /// let cmp_ne = instruction_builder.cmp_ne(x, y, location);
    /// ```
    pub fn cmp_ne(&mut self, x: Value, y: Value, location: Option<Location>) -> Value {
        assert_eq!(x.get_type(self.module), y.get_type(self.module));
        let bool_ty = self.module.get_bool_ty();
        self.make_value(InstructionPayload::CmpNotEqual(x, y, bool_ty), location)
    }

    /// Record a compare less-than instruction in the block.
    ///
    /// Restrictions:
    /// - `x` and `y` must have the same type, which matches the type of the newly returned value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let function = module.create_function().with_name("func").with_return_type(u32_ty).with_argument_types(&[ u32_ty, u32_ty ]).build();
    /// # let block = function.create_block(&mut module).build();
    /// # let x = function.get_arg(&module, 0);
    /// # let y = function.get_arg(&module, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut module);
    /// # let location = None;
    /// let cmp_lt = instruction_builder.cmp_lt(x, y, location);
    /// ```
    pub fn cmp_lt(&mut self, x: Value, y: Value, location: Option<Location>) -> Value {
        assert_eq!(x.get_type(self.module), y.get_type(self.module));
        let bool_ty = self.module.get_bool_ty();
        self.make_value(InstructionPayload::CmpLessThan(x, y, bool_ty), location)
    }

    /// Record a compare less-than-equal instruction in the block.
    ///
    /// Restrictions:
    /// - `x` and `y` must have the same type, which matches the type of the newly returned value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let function = module.create_function().with_name("func").with_return_type(u32_ty).with_argument_types(&[ u32_ty, u32_ty ]).build();
    /// # let block = function.create_block(&mut module).build();
    /// # let x = function.get_arg(&module, 0);
    /// # let y = function.get_arg(&module, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut module);
    /// # let location = None;
    /// let cmp_le = instruction_builder.cmp_le(x, y, location);
    /// ```
    pub fn cmp_le(&mut self, x: Value, y: Value, location: Option<Location>) -> Value {
        assert_eq!(x.get_type(self.module), y.get_type(self.module));
        let bool_ty = self.module.get_bool_ty();
        self.make_value(
            InstructionPayload::CmpLessThanEqual(x, y, bool_ty),
            location,
        )
    }

    /// Record a compare greater-than instruction in the block.
    ///
    /// Restrictions:
    /// - `x` and `y` must have the same type, which matches the type of the newly returned value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let function = module.create_function().with_name("func").with_return_type(u32_ty).with_argument_types(&[ u32_ty, u32_ty ]).build();
    /// # let block = function.create_block(&mut module).build();
    /// # let x = function.get_arg(&module, 0);
    /// # let y = function.get_arg(&module, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut module);
    /// # let location = None;
    /// let cmp_gt = instruction_builder.cmp_gt(x, y, location);
    /// ```
    pub fn cmp_gt(&mut self, x: Value, y: Value, location: Option<Location>) -> Value {
        assert_eq!(x.get_type(self.module), y.get_type(self.module));
        let bool_ty = self.module.get_bool_ty();
        self.make_value(InstructionPayload::CmpGreaterThan(x, y, bool_ty), location)
    }

    /// Record a compare greater-than-equal instruction in the block.
    ///
    /// Restrictions:
    /// - `x` and `y` must have the same type, which matches the type of the newly returned value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let function = module.create_function().with_name("func").with_return_type(u32_ty).with_argument_types(&[ u32_ty, u32_ty ]).build();
    /// # let block = function.create_block(&mut module).build();
    /// # let x = function.get_arg(&module, 0);
    /// # let y = function.get_arg(&module, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut module);
    /// # let location = None;
    /// let cmp_ge = instruction_builder.cmp_ge(x, y, location);
    /// ```
    pub fn cmp_ge(&mut self, x: Value, y: Value, location: Option<Location>) -> Value {
        assert_eq!(x.get_type(self.module), y.get_type(self.module));
        let bool_ty = self.module.get_bool_ty();
        self.make_value(
            InstructionPayload::CmpGreaterThanEqual(x, y, bool_ty),
            location,
        )
    }

    /// Stack allocate a variable.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let u32_stack_ptr_ty = module.get_ptr_type(u32_ty, Domain::STACK);
    /// # let function = module.create_function().with_name("func").with_return_type(u32_ty).with_argument_types(&[]).build();
    /// # let block = function.create_block(&mut module).build();
    /// # let mut instruction_builder = block.create_instructions(&mut module);
    /// # let location = None;
    /// let stack_alloc = instruction_builder.stack_alloc(u32_ty, location);
    /// # assert_eq!(stack_alloc.get_type(&module), u32_stack_ptr_ty);
    /// ```
    pub fn stack_alloc(&mut self, ty: Type, location: Option<Location>) -> Value {
        let ptr_ty = self.module.get_ptr_type(ty, Domain::STACK);
        self.make_value(InstructionPayload::StackAlloc(ptr_ty), location)
    }

    /// Call a function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let called_function = module.create_function().with_name("called_function").with_return_type(u32_ty).with_argument_types(&[ u32_ty, u32_ty ]).build();
    /// # let function = module.create_function().with_name("func").with_return_type(u32_ty).with_argument_types(&[ u32_ty, u32_ty ]).build();
    /// # let block = function.create_block(&mut module).build();
    /// # let x = function.get_arg(&module, 0);
    /// # let y = function.get_arg(&module, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut module);
    /// # let location = None;
    /// let call = instruction_builder.call(called_function, &[ x, y ], location);
    /// ```
    pub fn call(
        &mut self,
        function: Function,
        args: &[Value],
        location: Option<Location>,
    ) -> Value {
        self.make_value(InstructionPayload::Call(function, args.to_vec()), location)
    }

    /// Record an unconditional branch between blocks.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let function = module.create_function().with_name("func").with_return_type(u32_ty).with_argument_types(&[ u32_ty, u32_ty ]).build();
    /// # let block = function.create_block(&mut module).build();
    /// # let called_block = function.create_block(&mut module).with_argument_types(&[ u32_ty, u32_ty ]).build();
    /// # let x = function.get_arg(&module, 0);
    /// # let y = function.get_arg(&module, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut module);
    /// # let location = None;
    /// instruction_builder.branch(called_block, &[ x, y ], location);
    /// ```
    pub fn branch(&mut self, block: Block, args: &[Value], location: Option<Location>) {
        self.make_value(InstructionPayload::Branch(block, args.to_vec()), location);
    }

    /// Record a conditional branch between blocks.
    ///
    /// Restrictions:
    /// - The `condition` must be a boolean.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let function = module.create_function().with_name("func").with_return_type(u32_ty).with_argument_types(&[ u32_ty, u32_ty ]).build();
    /// # let block = function.create_block(&mut module).build();
    /// # let called_block = function.create_block(&mut module).with_argument_types(&[ u32_ty, u32_ty ]).build();
    /// # let x = function.get_arg(&module, 0);
    /// # let y = function.get_arg(&module, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut module);
    /// # let location = None;
    /// # let condition = instruction_builder.cmp_ge(x, y, location);
    /// instruction_builder.conditional_branch(condition, called_block, &[ x, y ], location);
    /// ```
    pub fn conditional_branch(
        &mut self,
        condition: Value,
        block: Block,
        args: &[Value],
        location: Option<Location>,
    ) {
        assert!(condition.get_type(self.module).is_boolean(self.module));
        self.make_value(
            InstructionPayload::ConditionalBranch(condition, block, args.to_vec()),
            location,
        );
    }

    /// Record a select between two values in the block.
    ///
    /// Restrictions:
    /// - The `condition` must be a boolean.
    /// - `x` and `y` must have the same type, which matches the type of the newly returned value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let function = module.create_function().with_name("func").with_return_type(u32_ty).with_argument_types(&[ u32_ty, u32_ty ]).build();
    /// # let block = function.create_block(&mut module).build();
    /// # let x = function.get_arg(&module, 0);
    /// # let y = function.get_arg(&module, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut module);
    /// # let location = None;
    /// # let condition = instruction_builder.cmp_ge(x, y, location);
    /// let select = instruction_builder.select(condition, x, y, location);
    /// ```
    pub fn select(
        &mut self,
        condition: Value,
        x: Value,
        y: Value,
        location: Option<Location>,
    ) -> Value {
        assert!(condition.get_type(self.module).is_boolean(self.module));
        assert_eq!(x.get_type(self.module), y.get_type(self.module));
        self.make_value(InstructionPayload::Select(condition, x, y), location)
    }

    /// Get a pointer to an element from within another pointer.
    ///
    /// Restrictions:
    /// - `ptr` must be a pointer type.
    /// - `indices` must be non-empty.
    /// - Constants must be used when indexing into a struct.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let u32_ptr_ty = module.get_ptr_type(u32_ty, Domain::CPU);
    /// # let u32_array_ty = module.get_array_ty(u32_ty, 42);
    /// # let struct_ty = module.get_struct_ty(&[ u32_ptr_ty, u32_array_ty, u32_ty ]);
    /// # let ptr_ty = module.get_ptr_type(struct_ty, Domain::CPU);
    /// # let function = module.create_function().with_name("func").with_return_type(u32_ptr_ty).with_argument_types(&[ ptr_ty ]).build();
    /// # let block = function.create_block(&mut module).build();
    /// # let ptr = function.get_arg(&module, 0);
    /// # let i0 = module.get_int_constant(8, 0);
    /// # let i1 = module.get_uint_constant(32, 1);
    /// # let i2 = i0;
    /// # let mut instruction_builder = block.create_instructions(&mut module);
    /// # let location = None;
    /// let get_element_ptr = instruction_builder.get_element_ptr(ptr, &[ i0, i1, i2 ], location);
    /// # assert_eq!(get_element_ptr.get_type(&module), u32_ptr_ty);
    /// ```
    pub fn get_element_ptr(
        &mut self,
        ptr: Value,
        indices: &[Value],
        location: Option<Location>,
    ) -> Value {
        let ptr_ty = ptr.get_type(self.module);

        assert!(ptr_ty.is_ptr(self.module));
        assert!(!indices.is_empty());

        let mut ty = ptr_ty.get_pointee(self.module);

        // Skip the first index since that produces a type the same as the pointer.
        for index in &indices[1..] {
            ty = ty.get_indexed(self.module, *index);
        }

        // Lastly turn the indexed type back into a pointer.
        ty = self.module.get_ptr_type(ty, ptr_ty.get_domain(self.module));

        self.make_value(
            InstructionPayload::GetElementPtr(ptr, indices.to_vec(), ty),
            location,
        )
    }
}
