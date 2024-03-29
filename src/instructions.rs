use crate::*;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "io", derive(Serialize, Deserialize))]
pub enum Cmp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl fmt::Display for Cmp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Cmp::Eq => write!(f, "eq"),
            Cmp::Ne => write!(f, "ne"),
            Cmp::Lt => write!(f, "lt"),
            Cmp::Le => write!(f, "le"),
            Cmp::Gt => write!(f, "gt"),
            Cmp::Ge => write!(f, "ge"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "io", derive(Serialize, Deserialize))]
pub enum Unary {
    Neg,
    Not,
}

impl fmt::Display for Unary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Unary::Neg => write!(f, "neg"),
            Unary::Not => write!(f, "not"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "io", derive(Serialize, Deserialize))]
pub enum Binary {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    And,
    Or,
    Xor,
    Shl,
    Shr,
}

impl fmt::Display for Binary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Binary::Add => write!(f, "add"),
            Binary::Sub => write!(f, "sub"),
            Binary::Mul => write!(f, "mul"),
            Binary::Div => write!(f, "div"),
            Binary::Rem => write!(f, "rem"),
            Binary::And => write!(f, "and"),
            Binary::Or => write!(f, "or"),
            Binary::Xor => write!(f, "xor"),
            Binary::Shl => write!(f, "shl"),
            Binary::Shr => write!(f, "shr"),
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "io", derive(Serialize, Deserialize))]
pub enum Instruction {
    Return(Option<Location>),
    ReturnValue(Type, Value, Option<Location>),
    Cmp(Type, Cmp, Value, Value, Option<Location>),
    Unary(Type, Unary, Value, Option<Location>),
    Binary(Type, Binary, Value, Value, Option<Location>),
    Cast(Type, Value, Option<Location>),
    BitCast(Type, Value, Option<Location>),
    Load(Value, Option<Location>),
    Store(Value, Value, Option<Location>),
    Extract(Value, usize, Option<Location>),
    Insert(Value, Value, usize, Option<Location>),
    StackAlloc(Name, Type, Option<Location>),
    Call(Function, Vec<Value>, Option<Location>),
    Branch(Block, Vec<Value>, Option<Location>),
    ConditionalBranch(
        Value,
        Block,
        Block,
        Vec<Value>,
        Vec<Value>,
        Option<Location>,
    ),
    Select(Type, Value, Value, Value, Option<Location>),
    IndexInto(Type, Value, Vec<Value>, Option<Location>),
}

pub struct InstructionDisplayer<'a> {
    pub(crate) value: Value,
    pub(crate) context: &'a Context,
}

impl<'a> std::fmt::Display for InstructionDisplayer<'a> {
    fn fmt(
        &self,
        writer: &mut std::fmt::Formatter<'_>,
    ) -> std::result::Result<(), std::fmt::Error> {
        let inst = self.value.get_inst(self.context);
        match inst {
            Instruction::Return(loc) => {
                write!(writer, "ret",)?;
                if let Some(loc) = loc {
                    write!(writer, "{}", loc.get_displayer(self.context))?;
                }
            }
            Instruction::ReturnValue(_, val, loc) => {
                write!(writer, "ret {}", val.get_displayer(self.context),)?;
                if let Some(loc) = loc {
                    write!(writer, "{}", loc.get_displayer(self.context))?;
                }
            }
            Instruction::Cmp(_, cmp, a, b, loc) => {
                write!(
                    writer,
                    "{} = cmp {} {}, {}",
                    self.value.get_displayer(self.context),
                    cmp,
                    a.get_displayer(self.context),
                    b.get_displayer(self.context),
                )?;
                if let Some(loc) = loc {
                    write!(writer, "{}", loc.get_displayer(self.context))?;
                }
            }
            Instruction::Unary(_, unary, a, loc) => {
                write!(
                    writer,
                    "{} = {} {}",
                    self.value.get_displayer(self.context),
                    unary,
                    a.get_displayer(self.context),
                )?;
                if let Some(loc) = loc {
                    write!(writer, "{}", loc.get_displayer(self.context))?;
                }
            }
            Instruction::Binary(_, binary, a, b, loc) => {
                write!(
                    writer,
                    "{} = {} {}, {}",
                    self.value.get_displayer(self.context),
                    binary,
                    a.get_displayer(self.context),
                    b.get_displayer(self.context),
                )?;
                if let Some(loc) = loc {
                    write!(writer, "{}", loc.get_displayer(self.context))?;
                }
            }
            Instruction::Cast(ty, val, loc) => {
                write!(
                    writer,
                    "{} = cast {} to {}",
                    self.value.get_displayer(self.context),
                    val.get_displayer(self.context),
                    ty.get_displayer(self.context),
                )?;
                if let Some(loc) = loc {
                    write!(writer, "{}", loc.get_displayer(self.context))?;
                }
            }
            Instruction::BitCast(ty, val, loc) => {
                write!(
                    writer,
                    "{} = bitcast {} to {}",
                    self.value.get_displayer(self.context),
                    val.get_displayer(self.context),
                    ty.get_displayer(self.context),
                )?;
                if let Some(loc) = loc {
                    write!(writer, "{}", loc.get_displayer(self.context))?;
                }
            }
            Instruction::Load(ptr, loc) => {
                write!(
                    writer,
                    "{} = load {}",
                    self.value.get_displayer(self.context),
                    ptr.get_displayer(self.context),
                )?;
                if let Some(loc) = loc {
                    write!(writer, "{}", loc.get_displayer(self.context))?;
                }
            }
            Instruction::Store(ptr, val, loc) => {
                write!(
                    writer,
                    "store {}, {}",
                    ptr.get_displayer(self.context),
                    val.get_displayer(self.context),
                )?;
                if let Some(loc) = loc {
                    write!(writer, "{}", loc.get_displayer(self.context))?;
                }
            }
            Instruction::Extract(agg, index, loc) => {
                write!(
                    writer,
                    "{} = extract {}, {}",
                    self.value.get_displayer(self.context),
                    agg.get_displayer(self.context),
                    index,
                )?;
                if let Some(loc) = loc {
                    write!(writer, "{}", loc.get_displayer(self.context))?;
                }
            }
            Instruction::Insert(agg, elem, index, loc) => {
                write!(
                    writer,
                    "{} = insert {}, {}, {}",
                    self.value.get_displayer(self.context),
                    agg.get_displayer(self.context),
                    elem.get_displayer(self.context),
                    index,
                )?;
                if let Some(loc) = loc {
                    write!(writer, "{}", loc.get_displayer(self.context))?;
                }
            }
            Instruction::StackAlloc(name, ptr_ty, loc) => {
                write!(
                    writer,
                    "{} = stackalloc {}, {}",
                    self.value.get_displayer(self.context),
                    name.get_displayer(self.context),
                    ptr_ty
                        .get_element(self.context, 0)
                        .get_displayer(self.context),
                )?;

                if let Some(loc) = loc {
                    write!(writer, "{}", loc.get_displayer(self.context))?;
                }
            }
            Instruction::Call(func, args, loc) => {
                write!(
                    writer,
                    "{} = call {} from {} (",
                    self.value.get_displayer(self.context),
                    func.get_name(self.context).get_displayer(self.context),
                    func.get_module(self.context)
                        .get_name(self.context)
                        .get_displayer(self.context)
                )?;

                for arg in args.iter().take(1) {
                    write!(writer, "{}", arg.get_displayer(self.context))?;
                }

                for arg in args.iter().skip(1) {
                    write!(writer, ", {}", arg.get_displayer(self.context))?;
                }

                write!(writer, ")")?;

                if let Some(loc) = loc {
                    write!(writer, "{}", loc.get_displayer(self.context))?;
                }
            }
            Instruction::Branch(block, args, loc) => {
                write!(writer, "br b{}(", block.get_unique_index())?;

                for arg in args.iter().take(1) {
                    write!(writer, "{}", arg.get_displayer(self.context))?;
                }

                for arg in args.iter().skip(1) {
                    write!(writer, ", {}", arg.get_displayer(self.context))?;
                }

                write!(writer, ")")?;

                if let Some(loc) = loc {
                    write!(writer, "{}", loc.get_displayer(self.context))?;
                }
            }
            Instruction::ConditionalBranch(
                cond,
                true_block,
                false_block,
                true_args,
                false_args,
                loc,
            ) => {
                write!(
                    writer,
                    "cbr {}, b{}(",
                    cond.get_displayer(self.context),
                    true_block.get_unique_index()
                )?;

                for arg in true_args.iter().take(1) {
                    write!(writer, "{}", arg.get_displayer(self.context))?;
                }

                for arg in true_args.iter().skip(1) {
                    write!(writer, ", {}", arg.get_displayer(self.context))?;
                }

                write!(writer, "), b{}(", false_block.get_unique_index())?;

                for arg in false_args.iter().take(1) {
                    write!(writer, "{}", arg.get_displayer(self.context))?;
                }

                for arg in false_args.iter().skip(1) {
                    write!(writer, ", {}", arg.get_displayer(self.context))?;
                }

                write!(writer, ")")?;

                if let Some(loc) = loc {
                    write!(writer, "{}", loc.get_displayer(self.context))?;
                }
            }
            Instruction::Select(_, cond, true_val, false_val, loc) => {
                write!(
                    writer,
                    "{} = select {}, {}, {}",
                    self.value.get_displayer(self.context),
                    cond.get_displayer(self.context),
                    true_val.get_displayer(self.context),
                    false_val.get_displayer(self.context),
                )?;

                if let Some(loc) = loc {
                    write!(writer, "{}", loc.get_displayer(self.context))?;
                }
            }
            Instruction::IndexInto(_, ptr, args, loc) => {
                write!(
                    writer,
                    "{} = indexinto {}, ",
                    self.value.get_displayer(self.context),
                    ptr.get_displayer(self.context),
                )?;

                for arg in args.iter().take(1) {
                    write!(writer, "{}", arg.get_displayer(self.context))?;
                }

                for arg in args.iter().skip(1) {
                    write!(writer, ", {}", arg.get_displayer(self.context))?;
                }

                if let Some(loc) = loc {
                    write!(writer, "{}", loc.get_displayer(self.context))?;
                }
            }
        }

        Ok(())
    }
}

impl Typed for Instruction {
    /// Get the type of an instruction.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(u32_ty).build();
    /// # let _ = function.create_block(&mut context).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let constant = context.get_uint_constant(32, 42);
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let instruction = instruction_builder.ret_val(constant, None);
    /// let ty = instruction.get_type(&context);
    /// # assert_eq!(ty, u32_ty);
    ///
    /// ```
    fn get_type(&self, context: &Context) -> Type {
        match self {
            Instruction::Return(_) => panic!("Cannot get the type of a void return"),
            Instruction::ReturnValue(ty, _, _) => *ty,
            Instruction::Cmp(ty, _, _, _, _) => *ty,
            Instruction::Unary(ty, _, _, _) => *ty,
            Instruction::Binary(ty, _, _, _, _) => *ty,
            Instruction::Cast(ty, _, _) => *ty,
            Instruction::BitCast(ty, _, _) => *ty,
            Instruction::Load(value, _) => value.get_type(context).get_element(context, 0),
            Instruction::Store(_, _, _) => panic!("Cannot get the type of a store"),
            Instruction::Extract(val, index, _) => {
                val.get_type(context).get_element(context, *index)
            }
            Instruction::Insert(val, _, _, _) => val.get_type(context),
            Instruction::StackAlloc(_, ty, _) => *ty,
            Instruction::Call(function, _, _) => function.get_return_type(context),
            Instruction::Branch(_, _, _) => panic!("Cannot get the type of a branch"),
            Instruction::ConditionalBranch(_, _, _, _, _, _) => {
                panic!("Cannot get the type of a conditional branch")
            }
            Instruction::Select(ty, _, _, _, _) => *ty,
            Instruction::IndexInto(ty, _, _, _) => *ty,
        }
    }
}

impl Named for Instruction {
    /// Get the name of an instruction.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let function = module.create_function(&mut context).with_name("func").build();
    /// # let _ = function.create_block(&mut context).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let constant = context.get_uint_constant(32, 42);
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let instruction = instruction_builder.stack_alloc("😀", u32_ty, None);
    /// # instruction_builder.ret(None);
    /// let name = instruction.get_name(&context);
    /// # assert_eq!(name.as_str(&context), "😀");
    /// ```
    fn get_name(&self, _: &Context) -> Name {
        match self {
            Instruction::StackAlloc(name, _, _) => *name,
            _ => panic!("Cannot get the name of instruction"),
        }
    }
}

pub struct PausedInstructionBuilder {
    block: Block,
}

impl PausedInstructionBuilder {
    pub fn get_block(&self) -> Block {
        self.block
    }
}

pub struct InstructionBuilder<'a> {
    context: &'a mut Context,
    block: Block,
}

impl<'a> InstructionBuilder<'a> {
    pub(crate) fn with_context_and_block(context: &'a mut Context, block: Block) -> Self {
        InstructionBuilder { context, block }
    }

    fn make_value(&mut self, instruction: Instruction) -> Value {
        let index = self
            .context
            .values
            .insert(ValuePayload::Instruction(instruction));
        self.context.blocks[self.block.0]
            .instructions
            .push(Value(index));
        Value(index)
    }

    /// Pause building an instruction builder (used when you need to use the library during building).
    ///
    /// This is useful when you need to create types during instruction building, so need to pause building.
    pub fn pause_building(self) -> PausedInstructionBuilder {
        PausedInstructionBuilder { block: self.block }
    }

    /// Resume building an instruction builder.
    pub fn resume_building(context: &'a mut Context, paused: PausedInstructionBuilder) -> Self {
        InstructionBuilder {
            context,
            block: paused.block,
        }
    }

    /// Borrow the context from the builder.
    ///
    /// This is useful if you need to build a type or constant during instruction building.
    pub fn borrow_context(&mut self) -> &mut Context {
        &mut self.context
    }

    /// Record a return from the function which closes the block.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let function = module.create_function(&mut context).with_name("func").build();
    /// # let block = function.create_block(&mut context).build();
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let location = None;
    /// instruction_builder.ret(location);
    /// ```
    pub fn ret(mut self, location: Option<Location>) -> Value {
        self.make_value(Instruction::Return(location))
    }

    /// Record a return from the function which closes the block.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(u32_ty).with_arg("a", u32_ty).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let return_value = function.get_arg(&context, 0);
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let location = None;
    /// instruction_builder.ret_val(return_value, location);
    /// ```
    pub fn ret_val(mut self, value: Value, location: Option<Location>) -> Value {
        self.make_value(Instruction::ReturnValue(
            value.get_type(self.context),
            value,
            location,
        ))
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
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(u32_ty).with_arg("a", u32_ty).with_arg("b", u32_ty).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let x = function.get_arg(&context, 0);
    /// # let y = function.get_arg(&context, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let location = None;
    /// let add = instruction_builder.add(x, y, location);
    /// ```
    pub fn add(&mut self, x: Value, y: Value, location: Option<Location>) -> Value {
        self.make_value(Instruction::Binary(
            x.get_type(self.context),
            Binary::Add,
            x,
            y,
            location,
        ))
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
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(u32_ty).with_arg("a", u32_ty).with_arg("b", u32_ty).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let x = function.get_arg(&context, 0);
    /// # let y = function.get_arg(&context, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let location = None;
    /// let sub = instruction_builder.sub(x, y, location);
    /// ```
    pub fn sub(&mut self, x: Value, y: Value, location: Option<Location>) -> Value {
        self.make_value(Instruction::Binary(
            x.get_type(self.context),
            Binary::Sub,
            x,
            y,
            location,
        ))
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
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(u32_ty).with_arg("a", u32_ty).with_arg("b", u32_ty).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let x = function.get_arg(&context, 0);
    /// # let y = function.get_arg(&context, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let location = None;
    /// let mul = instruction_builder.mul(x, y, location);
    /// ```
    pub fn mul(&mut self, x: Value, y: Value, location: Option<Location>) -> Value {
        self.make_value(Instruction::Binary(
            x.get_type(self.context),
            Binary::Mul,
            x,
            y,
            location,
        ))
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
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(u32_ty).with_arg("a", u32_ty).with_arg("b", u32_ty).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let x = function.get_arg(&context, 0);
    /// # let y = function.get_arg(&context, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let location = None;
    /// let div = instruction_builder.div(x, y, location);
    /// ```
    pub fn div(&mut self, x: Value, y: Value, location: Option<Location>) -> Value {
        self.make_value(Instruction::Binary(
            x.get_type(self.context),
            Binary::Div,
            x,
            y,
            location,
        ))
    }

    /// Record a remainder instruction in the block.
    ///
    /// Restrictions:
    /// - `x` and `y` must have the same type, which matches the type of the newly returned value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(u32_ty).with_arg("a", u32_ty).with_arg("b", u32_ty).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let x = function.get_arg(&context, 0);
    /// # let y = function.get_arg(&context, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let location = None;
    /// let rem = instruction_builder.rem(x, y, location);
    /// ```
    pub fn rem(&mut self, x: Value, y: Value, location: Option<Location>) -> Value {
        self.make_value(Instruction::Binary(
            x.get_type(self.context),
            Binary::Rem,
            x,
            y,
            location,
        ))
    }

    /// Negate a value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(u32_ty).with_arg("a", u32_ty).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let x = function.get_arg(&context, 0);
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let location = None;
    /// let div = instruction_builder.neg(x, location);
    /// ```
    pub fn neg(&mut self, x: Value, location: Option<Location>) -> Value {
        self.make_value(Instruction::Unary(
            x.get_type(self.context),
            Unary::Neg,
            x,
            location,
        ))
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
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(u32_ty).with_arg("a", u32_ty).with_arg("b", u32_ty).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let x = function.get_arg(&context, 0);
    /// # let y = function.get_arg(&context, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let location = None;
    /// let and = instruction_builder.and(x, y, location);
    /// ```
    pub fn and(&mut self, x: Value, y: Value, location: Option<Location>) -> Value {
        self.make_value(Instruction::Binary(
            x.get_type(self.context),
            Binary::And,
            x,
            y,
            location,
        ))
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
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(u32_ty).with_arg("a", u32_ty).with_arg("b", u32_ty).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let x = function.get_arg(&context, 0);
    /// # let y = function.get_arg(&context, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let location = None;
    /// let or = instruction_builder.or(x, y, location);
    /// ```
    pub fn or(&mut self, x: Value, y: Value, location: Option<Location>) -> Value {
        self.make_value(Instruction::Binary(
            x.get_type(self.context),
            Binary::Or,
            x,
            y,
            location,
        ))
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
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(u32_ty).with_arg("a", u32_ty).with_arg("b", u32_ty).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let x = function.get_arg(&context, 0);
    /// # let y = function.get_arg(&context, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let location = None;
    /// let xor = instruction_builder.xor(x, y, location);
    /// ```
    pub fn xor(&mut self, x: Value, y: Value, location: Option<Location>) -> Value {
        self.make_value(Instruction::Binary(
            x.get_type(self.context),
            Binary::Xor,
            x,
            y,
            location,
        ))
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
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(u32_ty).with_arg("a", u32_ty).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let x = function.get_arg(&context, 0);
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let location = None;
    /// let not = instruction_builder.not(x, location);
    /// ```
    pub fn not(&mut self, x: Value, location: Option<Location>) -> Value {
        self.make_value(Instruction::Unary(
            x.get_type(self.context),
            Unary::Not,
            x,
            location,
        ))
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
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(u32_ty).with_arg("a", u32_ty).with_arg("b", u32_ty).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let x = function.get_arg(&context, 0);
    /// # let y = function.get_arg(&context, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let location = None;
    /// let shl = instruction_builder.shl(x, y, location);
    /// ```
    pub fn shl(&mut self, x: Value, y: Value, location: Option<Location>) -> Value {
        self.make_value(Instruction::Binary(
            x.get_type(self.context),
            Binary::Shl,
            x,
            y,
            location,
        ))
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
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(u32_ty).with_arg("a", u32_ty).with_arg("b", u32_ty).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let x = function.get_arg(&context, 0);
    /// # let y = function.get_arg(&context, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let location = None;
    /// let shr = instruction_builder.shr(x, y, location);
    /// ```
    pub fn shr(&mut self, x: Value, y: Value, location: Option<Location>) -> Value {
        self.make_value(Instruction::Binary(
            x.get_type(self.context),
            Binary::Shr,
            x,
            y,
            location,
        ))
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
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let i8_ty = context.get_int_type(8);
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(u32_ty).with_arg("a", u32_ty).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let x = function.get_arg(&context, 0);
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let location = None;
    /// let cast = instruction_builder.cast(x, i8_ty, location);
    /// ```
    pub fn cast(&mut self, x: Value, ty: Type, location: Option<Location>) -> Value {
        self.make_value(Instruction::Cast(ty, x, location))
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
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let i32_ty = context.get_int_type(32);
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(u32_ty).with_arg("a", u32_ty).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let x = function.get_arg(&context, 0);
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let location = None;
    /// let cast = instruction_builder.bitcast(x, i32_ty, location);
    /// ```
    pub fn bitcast(&mut self, x: Value, ty: Type, location: Option<Location>) -> Value {
        let x_ty = x.get_type(self.context);
        assert_ne!(x_ty, ty);
        assert_eq!(x_ty.get_bits(self.context), ty.get_bits(self.context));
        self.make_value(Instruction::BitCast(ty, x, location))
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
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let u32_ptr_ty = context.get_pointer_type(u32_ty, Domain::Cpu);
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(u32_ty).with_arg("a", u32_ptr_ty).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let ptr = function.get_arg(&context, 0);
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let location = None;
    /// let load = instruction_builder.load(ptr, location);
    /// ```
    pub fn load(&mut self, ptr: Value, location: Option<Location>) -> Value {
        assert!(ptr.get_type(self.context).is_pointer(self.context));
        self.make_value(Instruction::Load(ptr, location))
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
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let void_ty = context.get_void_type();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let u32_ptr_ty = context.get_pointer_type(u32_ty, Domain::Cpu);
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(void_ty).with_arg("a", u32_ty).with_arg("b", u32_ptr_ty).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let val = function.get_arg(&context, 0);
    /// # let ptr = function.get_arg(&context, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let location = None;
    /// let store = instruction_builder.store(ptr, val, location);
    /// ```
    pub fn store(&mut self, ptr: Value, val: Value, location: Option<Location>) -> Value {
        assert!(ptr.get_type(self.context).is_pointer(self.context));
        self.make_value(Instruction::Store(ptr, val, location))
    }

    /// Extract an element from an array, vector, or struct type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let vec_ty = context.get_vector_type(u32_ty, 4);
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(u32_ty).with_arg("", vec_ty).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let val = function.get_arg(&context, 0);
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let location = None;
    /// let extract = instruction_builder.extract(val, 0, location);
    /// ```
    pub fn extract(&mut self, val: Value, idx: usize, location: Option<Location>) -> Value {
        self.make_value(Instruction::Extract(val, idx, location))
    }

    /// Insert an element into a vector or struct type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let vec_ty = context.get_vector_type(u32_ty, 4);
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(u32_ty).with_arg("a", vec_ty).with_arg("b", u32_ty).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let val = function.get_arg(&context, 0);
    /// # let element = function.get_arg(&context, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut context);
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
            val.get_type(self.context).get_element(self.context, idx),
            element.get_type(self.context)
        );
        self.make_value(Instruction::Insert(val, element, idx, location))
    }

    /// Record a compare instruction in the block.
    ///
    /// Restrictions:
    /// - `x` and `y` must have the same type, which matches the type of the newly returned value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(u32_ty).with_arg("a", u32_ty).with_arg("b", u32_ty).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let x = function.get_arg(&context, 0);
    /// # let y = function.get_arg(&context, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let location = None;
    /// let cmp_eq = instruction_builder.cmp(Cmp::Eq, x, y, location);
    /// ```
    pub fn cmp(&mut self, cmp: Cmp, x: Value, y: Value, location: Option<Location>) -> Value {
        let bool_ty = self.context.get_bool_type();
        self.make_value(Instruction::Cmp(bool_ty, cmp, x, y, location))
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
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(u32_ty).with_arg("a", u32_ty).with_arg("b", u32_ty).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let x = function.get_arg(&context, 0);
    /// # let y = function.get_arg(&context, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let location = None;
    /// let cmp_eq = instruction_builder.cmp_eq(x, y, location);
    /// ```
    pub fn cmp_eq(&mut self, x: Value, y: Value, location: Option<Location>) -> Value {
        let bool_ty = self.context.get_bool_type();
        self.make_value(Instruction::Cmp(bool_ty, Cmp::Eq, x, y, location))
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
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(u32_ty).with_arg("a", u32_ty).with_arg("b", u32_ty).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let x = function.get_arg(&context, 0);
    /// # let y = function.get_arg(&context, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let location = None;
    /// let cmp_ne = instruction_builder.cmp_ne(x, y, location);
    /// ```
    pub fn cmp_ne(&mut self, x: Value, y: Value, location: Option<Location>) -> Value {
        let bool_ty = self.context.get_bool_type();
        self.make_value(Instruction::Cmp(bool_ty, Cmp::Ne, x, y, location))
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
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(u32_ty).with_arg("a", u32_ty).with_arg("b", u32_ty).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let x = function.get_arg(&context, 0);
    /// # let y = function.get_arg(&context, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let location = None;
    /// let cmp_lt = instruction_builder.cmp_lt(x, y, location);
    /// ```
    pub fn cmp_lt(&mut self, x: Value, y: Value, location: Option<Location>) -> Value {
        let bool_ty = self.context.get_bool_type();
        self.make_value(Instruction::Cmp(bool_ty, Cmp::Lt, x, y, location))
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
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(u32_ty).with_arg("a", u32_ty).with_arg("b", u32_ty).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let x = function.get_arg(&context, 0);
    /// # let y = function.get_arg(&context, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let location = None;
    /// let cmp_le = instruction_builder.cmp_le(x, y, location);
    /// ```
    pub fn cmp_le(&mut self, x: Value, y: Value, location: Option<Location>) -> Value {
        let bool_ty = self.context.get_bool_type();
        self.make_value(Instruction::Cmp(bool_ty, Cmp::Le, x, y, location))
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
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(u32_ty).with_arg("a", u32_ty).with_arg("b", u32_ty).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let x = function.get_arg(&context, 0);
    /// # let y = function.get_arg(&context, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let location = None;
    /// let cmp_gt = instruction_builder.cmp_gt(x, y, location);
    /// ```
    pub fn cmp_gt(&mut self, x: Value, y: Value, location: Option<Location>) -> Value {
        let bool_ty = self.context.get_bool_type();
        self.make_value(Instruction::Cmp(bool_ty, Cmp::Gt, x, y, location))
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
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(u32_ty).with_arg("a", u32_ty).with_arg("b", u32_ty).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let x = function.get_arg(&context, 0);
    /// # let y = function.get_arg(&context, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let location = None;
    /// let cmp_ge = instruction_builder.cmp_ge(x, y, location);
    /// ```
    pub fn cmp_ge(&mut self, x: Value, y: Value, location: Option<Location>) -> Value {
        let bool_ty = self.context.get_bool_type();
        self.make_value(Instruction::Cmp(bool_ty, Cmp::Ge, x, y, location))
    }

    /// Stack allocate a variable.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let stack_ptr_ty = context.get_pointer_type(u32_ty, Domain::Stack);
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(u32_ty).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let location = None;
    /// let stack_alloc = instruction_builder.stack_alloc("a", u32_ty, location);
    /// # assert_eq!(stack_alloc.get_type(&context), stack_ptr_ty);
    /// ```
    pub fn stack_alloc(&mut self, name: &str, ty: Type, location: Option<Location>) -> Value {
        let ptr_ty = self.context.get_pointer_type(ty, Domain::Stack);

        let name_index = self.context.get_name(name);
        self.make_value(Instruction::StackAlloc(name_index, ptr_ty, location))
    }

    /// Call a function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let called_function = module.create_function(&mut context).with_name("called_function").with_return_type(u32_ty).with_arg("a", u32_ty).with_arg("b", u32_ty).build();
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(u32_ty).with_arg("a", u32_ty).with_arg("b", u32_ty).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let x = function.get_arg(&context, 0);
    /// # let y = function.get_arg(&context, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let location = None;
    /// let call = instruction_builder.call(called_function, &[ x, y ], location);
    /// ```
    pub fn call(
        &mut self,
        function: Function,
        args: &[Value],
        location: Option<Location>,
    ) -> Value {
        self.make_value(Instruction::Call(function, args.to_vec(), location))
    }

    /// Record an unconditional branch between blocks.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(u32_ty).with_arg("a", u32_ty).with_arg("b", u32_ty).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let called_block = function.create_block(&mut context).with_arg(u32_ty).with_arg(u32_ty).build();
    /// # let x = function.get_arg(&context, 0);
    /// # let y = function.get_arg(&context, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let location = None;
    /// instruction_builder.branch(called_block, &[ x, y ], location);
    /// ```
    pub fn branch(mut self, block: Block, args: &[Value], location: Option<Location>) {
        self.make_value(Instruction::Branch(block, args.to_vec(), location));
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
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(u32_ty).with_arg("a", u32_ty).with_arg("b", u32_ty).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let true_block = function.create_block(&mut context).with_arg(u32_ty).with_arg(u32_ty).build();
    /// # let false_block = function.create_block(&mut context).with_arg(u32_ty).with_arg(u32_ty).build();
    /// # let x = function.get_arg(&context, 0);
    /// # let y = function.get_arg(&context, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let location = None;
    /// # let condition = instruction_builder.cmp_ge(x, y, location);
    /// instruction_builder.conditional_branch(condition, true_block, false_block, &[ x, y ], &[ x, y ], location);
    /// ```
    pub fn conditional_branch(
        mut self,
        condition: Value,
        true_block: Block,
        false_block: Block,
        true_args: &[Value],
        false_args: &[Value],
        location: Option<Location>,
    ) {
        self.make_value(Instruction::ConditionalBranch(
            condition,
            true_block,
            false_block,
            true_args.to_vec(),
            false_args.to_vec(),
            location,
        ));
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
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(u32_ty).with_arg("a", u32_ty).with_arg("b", u32_ty).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let x = function.get_arg(&context, 0);
    /// # let y = function.get_arg(&context, 1);
    /// # let mut instruction_builder = block.create_instructions(&mut context);
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
        assert!(condition.get_type(self.context).is_boolean(self.context));
        assert_eq!(x.get_type(self.context), y.get_type(self.context));
        self.make_value(Instruction::Select(
            x.get_type(self.context),
            condition,
            x,
            y,
            location,
        ))
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
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let u32_ptr_ty = context.get_pointer_type(u32_ty, Domain::Cpu);
    /// # let u32_array_ty = context.get_array_type(u32_ty, 42);
    /// # let struct_ty = context.get_struct_type(&[ u32_ptr_ty, u32_array_ty, u32_ty ]);
    /// # let ptr_ty = context.get_pointer_type(struct_ty, Domain::Cpu);
    /// # let function = module.create_function(&mut context).with_name("func").with_return_type(u32_ptr_ty).with_arg("a", ptr_ty).build();
    /// # let block = function.create_block(&mut context).build();
    /// # let ptr = function.get_arg(&context, 0);
    /// # let i0 = context.get_int_constant(8, 0);
    /// # let i1 = context.get_uint_constant(32, 1);
    /// # let i2 = i0;
    /// # let mut instruction_builder = block.create_instructions(&mut context);
    /// # let location = None;
    /// let index_into = instruction_builder.index_into(ptr, &[ i0, i1, i2 ], location);
    /// # assert_eq!(index_into.get_type(&context), u32_ptr_ty);
    /// ```
    pub fn index_into(
        &mut self,
        ptr: Value,
        indices: &[Value],
        location: Option<Location>,
    ) -> Value {
        let ptr_ty = ptr.get_type(self.context);

        assert!(ptr_ty.is_pointer(self.context));
        assert!(!indices.is_empty());

        let domain = ptr_ty.get_domain(self.context);
        let mut ty = ptr_ty;

        for index in indices {
            let element_ty = ty.get_element(self.context, 0);

            let new_element_ty = if element_ty.is_struct(self.context) {
                assert!(index.is_constant(self.context));

                let constant = index.get_constant(self.context);

                match constant {
                    Constant::Int(i, _) => element_ty.get_element(self.context, *i as usize),
                    Constant::UInt(u, _) => element_ty.get_element(self.context, *u as usize),
                    _ => panic!("Index into struct is not constant"),
                }
            } else if element_ty.is_array(self.context) | element_ty.is_pointer(self.context) {
                element_ty.get_element(self.context, 0)
            } else {
                element_ty
            };

            // Any other index would yield the same type as getting the index from the 0th element.
            ty = self.context.get_pointer_type(new_element_ty, domain);
        }

        self.make_value(Instruction::IndexInto(ty, ptr, indices.to_vec(), location))
    }
}
