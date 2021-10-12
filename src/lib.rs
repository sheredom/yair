#![cfg_attr(feature = "nightly", feature(test))]

extern crate enumset;
extern crate generational_arena;
extern crate radix_trie;

#[cfg(feature = "llvm")]
extern crate llvm_sys;

#[cfg(feature = "llvm")]
extern crate libc;

use enumset::*;

mod argument;
mod block;
mod codegen;
mod constant;
mod context;
mod function;
mod global;
mod instructions;
mod jitgen;
mod linkgen;
mod location;
mod module;
mod value;
mod verify;

#[cfg(feature = "io")]
pub mod io;

#[cfg(feature = "llvm")]
pub mod llvm;

#[cfg(feature = "lld")]
pub mod lld;

pub use argument::*;
pub use block::*;
pub use codegen::*;
pub use constant::*;
pub use context::*;
pub use function::*;
pub use global::*;
pub use instructions::*;
pub use jitgen::*;
pub use location::*;
pub use module::*;
pub use value::*;
pub use verify::*;

#[cfg(feature = "io")]
use serde::{Deserialize, Serialize};

/// The domain that a memory location inhabits. Used by cross-function variables
/// and pointer types to encode where the memory resides.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "io", derive(Serialize, Deserialize))]
pub enum Domain {
    CrossDevice,
    Cpu,
    Gpu,
    Stack,
}

impl std::fmt::Display for Domain {
    fn fmt(
        &self,
        writer: &mut std::fmt::Formatter<'_>,
    ) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Domain::CrossDevice => write!(writer, "any"),
            Domain::Cpu => write!(writer, "cpu"),
            Domain::Gpu => write!(writer, "gpu"),
            Domain::Stack => write!(writer, "stack"),
        }
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "io", derive(Serialize, Deserialize))]
enum TypePayload {
    Void,
    Bool,
    Int(u8),
    UInt(u8),
    Float(u8),
    Vector(Type, u8),
    Pointer(Domain),
    Struct(Vec<Type>),
    Function(Type, Vec<Type>),
    Array(Type, usize),
    NamedStruct(
        Module,
        Name,
        Vec<(Name, Type, Option<Location>)>,
        Option<Location>,
    ),
}

impl Default for TypePayload {
    fn default() -> Self {
        TypePayload::Void
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "io", derive(Serialize, Deserialize))]
pub struct Name(pub(crate) generational_arena::Index);

impl Name {
    pub fn as_str<'a>(&self, context: &'a Context) -> &'a str {
        &context.names[self.0]
    }

    pub fn get_displayer<'a>(&self, context: &'a Context) -> NameDisplayer<'a> {
        NameDisplayer {
            name: *self,
            context,
        }
    }
}

pub struct NameDisplayer<'a> {
    pub(crate) name: Name,
    pub(crate) context: &'a Context,
}

impl<'a> std::fmt::Display for NameDisplayer<'a> {
    fn fmt(
        &self,
        writer: &mut std::fmt::Formatter<'_>,
    ) -> std::result::Result<(), std::fmt::Error> {
        let name = self.name.as_str(self.context);

        if name.is_empty() {
            write!(writer, "\"\"")
        } else if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            write!(writer, "{}", name)
        } else {
            write!(writer, "\"{}\"", name)
        }
    }
}

pub trait Named {
    fn get_name(&self, context: &Context) -> Name;
}

pub trait Typed {
    fn get_type(&self, context: &Context) -> Type;
}

pub trait UniqueIndex {
    fn get_unique_index(&self) -> usize;
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "io", derive(Serialize, Deserialize))]
pub struct Type(pub(crate) generational_arena::Index);

pub struct TypeDisplayer<'a> {
    pub(crate) ty: Type,
    pub(crate) context: &'a Context,
}

impl<'a> std::fmt::Display for TypeDisplayer<'a> {
    fn fmt(
        &self,
        writer: &mut std::fmt::Formatter<'_>,
    ) -> std::result::Result<(), std::fmt::Error> {
        if self.ty.is_named_struct(self.context) {
            write!(
                writer,
                "%{}",
                self.ty.get_name(self.context).get_displayer(self.context)
            )
        } else if self.ty.is_void(self.context) {
            write!(writer, "void")
        } else if self.ty.is_boolean(self.context) {
            write!(writer, "bool")
        } else if self.ty.is_vector(self.context) {
            write!(
                writer,
                "<{}, {}>",
                self.ty
                    .get_element(self.context, 0)
                    .get_displayer(self.context),
                self.ty.get_len(self.context)
            )
        } else if self.ty.is_array(self.context) {
            write!(
                writer,
                "[{}, {}]",
                self.ty
                    .get_element(self.context, 0)
                    .get_displayer(self.context),
                self.ty.get_len(self.context)
            )
        } else if self.ty.is_struct(self.context) {
            write!(writer, "{{")?;

            for i in 0..self.ty.get_len(self.context) {
                if i != 0 {
                    write!(writer, ", ")?;
                }

                write!(
                    writer,
                    "{}",
                    self.ty
                        .get_element(self.context, i)
                        .get_displayer(self.context)
                )?;
            }

            write!(writer, "}}")
        } else if self.ty.is_int(self.context) {
            write!(writer, "i{}", self.ty.get_bits(self.context))
        } else if self.ty.is_uint(self.context) {
            write!(writer, "u{}", self.ty.get_bits(self.context))
        } else if self.ty.is_float(self.context) {
            write!(writer, "f{}", self.ty.get_bits(self.context))
        } else if self.ty.is_pointer(self.context) {
            write!(writer, "*{}", self.ty.get_domain(self.context))
        } else {
            std::unreachable!();
        }
    }
}

impl Type {
    /// Get the number of bits required to represent the type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// let bits = u32_ty.get_bits(&context);
    /// # assert_eq!(bits, 32);
    /// ```
    pub fn get_bits(&self, context: &Context) -> usize {
        match context.types[self.0] {
            TypePayload::Void => 0,
            TypePayload::Int(x) => x as usize,
            TypePayload::UInt(x) => x as usize,
            TypePayload::Float(x) => x as usize,
            TypePayload::Vector(ty, width) => ty.get_bits(context) * (width as usize),
            _ => panic!("Cannot get the bit-width of type"),
        }
    }

    /// Get the length of an aggregate type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// let vec_ty = context.get_vector_type(u32_ty, 4);
    /// assert_eq!(vec_ty.get_len(&context), 4);
    /// ```
    pub fn get_len(&self, context: &Context) -> usize {
        match &context.types[self.0] {
            TypePayload::Vector(_, width) => *width as usize,
            TypePayload::Array(_, width) => *width,
            TypePayload::Struct(vec) => vec.len(),
            TypePayload::NamedStruct(_, _, vec, _) => vec.len(),
            _ => panic!("Cannot get the length of a non-aggregate type"),
        }
    }

    /// Get the domain from a pointer.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let u32_ptr_ty = context.get_pointer_type(Domain::Cpu);
    /// let domain = u32_ptr_ty.get_domain(&context);
    /// # assert_eq!(domain, Domain::Cpu);
    /// ```
    pub fn get_domain(&self, context: &Context) -> Domain {
        match context.types[self.0] {
            TypePayload::Pointer(domain) => domain,
            _ => panic!("Cannot get the domain from a non pointer type"),
        }
    }

    /// Get the element from an array, vector or struct type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let vec_ty = context.get_vector_type(u32_ty, 4);
    /// let element = vec_ty.get_element(&context, 3);
    /// # assert_eq!(element, u32_ty);
    /// ```
    pub fn get_element(&self, context: &Context, index: usize) -> Type {
        match &context.types[self.0] {
            TypePayload::Vector(ty, size) => {
                assert!(
                    index < (*size as usize),
                    "Index is beyond the end of the vector"
                );
                *ty
            }
            TypePayload::Struct(tys) => tys[index],
            TypePayload::NamedStruct(_, _, tys, _) => tys[index].1,
            TypePayload::Array(ty, size) => {
                assert!(index < *size, "Index is beyond the end of the array");
                *ty
            }
            tp => panic!("Cannot get the element from type {:?}", tp),
        }
    }

    /// Checks whether a type is a array type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let array_ty = context.get_array_type(u32_ty, 4);
    /// let is_array = array_ty.is_array(&context);
    /// # assert!(is_array);
    /// ```
    pub fn is_array(&self, context: &Context) -> bool {
        matches!(context.types[self.0], TypePayload::Array(_, _))
    }

    /// Checks whether a type is a struct type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let struct_ty = context.get_struct_type(&[ u32_ty ]);
    /// let is_struct = struct_ty.is_struct(&context);
    /// # assert!(is_struct);
    /// ```
    pub fn is_struct(&self, context: &Context) -> bool {
        matches!(context.types[self.0], TypePayload::Struct(_))
            || matches!(context.types[self.0], TypePayload::NamedStruct(_, _, _, _))
    }

    /// Checks whether a type is a named struct type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let struct_ty = context.get_struct_type(&[ u32_ty ]);
    /// let is_named_struct = struct_ty.is_named_struct(&context);
    /// # assert!(!is_named_struct);
    /// ```
    pub fn is_named_struct(&self, context: &Context) -> bool {
        matches!(context.types[self.0], TypePayload::NamedStruct(_, _, _, _))
    }

    /// Checks whether a type is a vector type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let vec_ty = context.get_vector_type(u32_ty, 4);
    /// let is_vec = vec_ty.is_vector(&context);
    /// # assert!(is_vec);
    /// ```
    pub fn is_vector(&self, context: &Context) -> bool {
        matches!(context.types[self.0], TypePayload::Vector(_, _))
    }

    /// Checks whether a type is an int type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let i32_ty = context.get_int_type(32);
    /// assert!(i32_ty.is_int(&context));
    /// ```
    pub fn is_int(&self, context: &Context) -> bool {
        matches!(context.types[self.0], TypePayload::Int(_))
    }

    /// Checks whether a type is an uint type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// assert!(u32_ty.is_uint(&context));
    /// ```
    pub fn is_uint(&self, context: &Context) -> bool {
        matches!(context.types[self.0], TypePayload::UInt(_))
    }

    /// Checks whether a type is a float type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let f32_ty = context.get_float_type(32);
    /// assert!(f32_ty.is_float(&context));
    /// ```
    pub fn is_float(&self, context: &Context) -> bool {
        matches!(context.types[self.0], TypePayload::Float(_))
    }

    /// Checks whether a type is an integral (signed or unsigned) type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let vec_ty = context.get_vector_type(u32_ty, 4);
    /// let is_integral = u32_ty.is_integral(&context);
    /// # assert!(is_integral);
    /// # assert!(!vec_ty.is_integral(&context));
    /// ```
    pub fn is_integral(&self, context: &Context) -> bool {
        matches!(
            context.types[self.0],
            TypePayload::Int(_) | TypePayload::UInt(_)
        )
    }

    /// Checks whether a type is an unsigned integer type, or a vector of it.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let vec_ty = context.get_vector_type(u32_ty, 4);
    /// # let bool_ty = context.get_bool_type();
    /// let is_uint = u32_ty.is_uint_or_uint_vector(&context);
    /// let is_vector_uint = vec_ty.is_uint_or_uint_vector(&context);
    /// # assert!(is_uint);
    /// # assert!(is_vector_uint);
    /// # assert!(!bool_ty.is_uint_or_uint_vector(&context));
    /// ```
    pub fn is_uint_or_uint_vector(&self, context: &Context) -> bool {
        let mut ty = *self;

        if ty.is_vector(context) {
            ty = ty.get_element(context, 0);
        }

        ty.is_uint(context)
    }

    /// Checks whether a type is a signed integer type, or a vector of it.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let i32_ty = context.get_int_type(32);
    /// # let vec_ty = context.get_vector_type(i32_ty, 4);
    /// # let bool_ty = context.get_bool_type();
    /// let is_int = i32_ty.is_int_or_int_vector(&context);
    /// let is_vector_int = vec_ty.is_int_or_int_vector(&context);
    /// # assert!(is_int);
    /// # assert!(is_vector_int);
    /// # assert!(!bool_ty.is_int_or_int_vector(&context));
    /// ```
    pub fn is_int_or_int_vector(&self, context: &Context) -> bool {
        let mut ty = *self;

        if ty.is_vector(context) {
            ty = ty.get_element(context, 0);
        }

        ty.is_int(context)
    }

    /// Checks whether a type is an integral (signed or unsigned) type, or a vector of integral.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let vec_ty = context.get_vector_type(u32_ty, 4);
    /// # let bool_ty = context.get_bool_type();
    /// let is_integral = u32_ty.is_integral_or_integral_vector(&context);
    /// let is_vector_integral = vec_ty.is_integral_or_integral_vector(&context);
    /// # assert!(is_integral);
    /// # assert!(is_vector_integral);
    /// # assert!(!bool_ty.is_integral_or_integral_vector(&context));
    /// ```
    pub fn is_integral_or_integral_vector(&self, context: &Context) -> bool {
        let mut ty = *self;

        if ty.is_vector(context) {
            ty = ty.get_element(context, 0);
        }

        ty.is_integral(context)
    }

    /// Checks whether a type is a float type, or a vector of float.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let f32_ty = context.get_float_type(32);
    /// # let vec_ty = context.get_vector_type(f32_ty, 4);
    /// # let bool_ty = context.get_bool_type();
    /// let is_float = f32_ty.is_float_or_float_vector(&context);
    /// let is_vector_float = vec_ty.is_float_or_float_vector(&context);
    /// # assert!(is_float);
    /// # assert!(is_vector_float);
    /// # assert!(!bool_ty.is_float_or_float_vector(&context));
    /// ```
    pub fn is_float_or_float_vector(&self, context: &Context) -> bool {
        let mut ty = *self;

        if ty.is_vector(context) {
            ty = ty.get_element(context, 0);
        }

        ty.is_float(context)
    }

    /// Checks whether a type is a float type, or a vector of float.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let f32_ty = context.get_float_type(32);
    /// # let bool_ty = context.get_bool_type();
    /// # let vec_ty = context.get_vector_type(bool_ty, 4);
    /// let is_bool = bool_ty.is_bool_or_bool_vector(&context);
    /// let is_vector_bool = vec_ty.is_bool_or_bool_vector(&context);
    /// # assert!(is_bool);
    /// # assert!(is_vector_bool);
    /// # assert!(!f32_ty.is_bool_or_bool_vector(&context));
    /// ```
    pub fn is_bool_or_bool_vector(&self, context: &Context) -> bool {
        let mut ty = *self;

        if ty.is_vector(context) {
            ty = ty.get_element(context, 0);
        }

        ty.is_boolean(context)
    }

    /// Checks whether a type is a boolean type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let vec_ty = context.get_vector_type(u32_ty, 4);
    /// # let bool_ty = context.get_bool_type();
    /// let is_boolean = bool_ty.is_boolean(&context);
    /// # assert!(is_boolean);
    /// # assert!(!vec_ty.is_boolean(&context));
    /// ```
    pub fn is_boolean(&self, context: &Context) -> bool {
        matches!(context.types[self.0], TypePayload::Bool)
    }

    /// Checks whether a type is a void type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let vec_ty = context.get_vector_type(u32_ty, 4);
    /// # let void_ty = context.get_void_type();
    /// assert!(void_ty.is_void(&context));
    /// # assert!(!vec_ty.is_boolean(&context));
    /// ```
    pub fn is_void(&self, context: &Context) -> bool {
        matches!(context.types[self.0], TypePayload::Void)
    }

    /// Checks whether a type is a pointer type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let bool_ty = context.get_bool_type();
    /// # let ptr_ty = context.get_pointer_type(Domain::Cpu);
    /// let is_pointer = ptr_ty.is_pointer(&context);
    /// # assert!(is_pointer);
    /// # assert!(!bool_ty.is_pointer(&context));
    /// ```
    pub fn is_pointer(&self, context: &Context) -> bool {
        matches!(context.types[self.0], TypePayload::Pointer(_))
    }

    /// Get the type at the index into the type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let module = context.create_module().build();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let ty = context.get_array_type(u32_ty, 42);
    /// # let index = context.get_int_constant(8, 0);
    /// let indexed_type = ty.get_indexed(&context, index);
    /// # assert_eq!(indexed_type, u32_ty);
    /// ```
    pub fn get_indexed(&self, context: &Context, index: Value) -> Type {
        match &context.types[self.0] {
            TypePayload::Array(ty, _) => *ty,
            TypePayload::Struct(tys) => {
                assert!(
                    index.is_constant(context),
                    "Cannot index into a struct with a non-constant"
                );

                match index.get_constant(context) {
                    Constant::Int(c, _) => tys[*c as usize],
                    Constant::UInt(c, _) => tys[*c as usize],
                    _ => panic!("Cannot index into a struct with a non-integral constant"),
                }
            }
            TypePayload::NamedStruct(_, _, tys, _) => {
                assert!(
                    index.is_constant(context),
                    "Cannot index into a struct with a non-constant"
                );

                match index.get_constant(context) {
                    Constant::Int(c, _) => tys[*c as usize].1,
                    Constant::UInt(c, _) => tys[*c as usize].1,
                    _ => panic!("Cannot index into a struct with a non-integral constant"),
                }
            }
            _ => panic!("Unable to index into type!"),
        }
    }

    /// Get the name of a named-struct type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let module = context.create_module().build();
    /// # let elements = vec![("my_field", u32_ty, None)];
    /// # let location = None;
    /// # let struct_ty = module.create_named_struct_type(&mut context, "my_struct", &elements, location);
    /// let name = struct_ty.get_name(&context);
    /// ```
    pub fn get_name(&self, context: &Context) -> Name {
        match context.types[self.0] {
            TypePayload::NamedStruct(_, name, _, _) => name,
            _ => panic!("Cannot get the name of anything other than a named-struct"),
        }
    }

    /// Get the name of an element of a named-struct type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let module = context.create_module().build();
    /// # let elements = vec![("my_field", u32_ty, None)];
    /// # let location = None;
    /// # let struct_ty = module.create_named_struct_type(&mut context, "my_struct", &elements, location);
    /// let name = struct_ty.get_element_name(&context, 0);
    /// ```
    pub fn get_element_name(&self, context: &Context, index: usize) -> Name {
        match &context.types[self.0] {
            TypePayload::NamedStruct(_, _, elements, _) => elements[index].0,
            _ => panic!("Cannot get the element name of anything other than a named-struct"),
        }
    }

    /// Get the location of a named-struct type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let u32_ty = context.get_uint_type(32);
    /// # let module = context.create_module().build();
    /// # let elements = vec![("my_field", u32_ty, None)];
    /// # let location = None;
    /// # let struct_ty = module.create_named_struct_type(&mut context, "my_struct", &elements, location);
    /// let location = struct_ty.get_location(&context);
    /// ```
    pub fn get_location(&self, context: &Context) -> Option<Location> {
        match context.types[self.0] {
            TypePayload::NamedStruct(_, _, _, location) => location,
            _ => panic!("Cannot get the location of anything other than a named-struct"),
        }
    }

    pub fn get_displayer<'a>(&self, context: &'a Context) -> TypeDisplayer<'a> {
        TypeDisplayer { ty: *self, context }
    }
}
