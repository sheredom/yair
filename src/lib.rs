extern crate enumset;
extern crate generational_arena;
extern crate radix_trie;

use enumset::*;

mod argument;
mod block;
mod codegen;
mod constant;
mod function;
mod global;
mod instructions;
mod library;
mod location;
mod module;
mod value;
mod verify;

#[cfg(feature = "io")]
pub mod io;

#[cfg(feature = "llvm")]
pub mod llvm;

pub use argument::*;
pub use block::*;
pub use codegen::*;
pub use constant::*;
pub use function::*;
pub use global::*;
pub use instructions::*;
pub use library::*;
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
    pub fn get_name<'a>(&self, library: &'a Library) -> &'a str {
        &library.names[self.0]
    }

    pub fn get_displayer<'a>(&self, library: &'a Library) -> NameDisplayer<'a> {
        NameDisplayer {
            name: *self,
            library,
        }
    }
}

pub struct NameDisplayer<'a> {
    pub(crate) name: Name,
    pub(crate) library: &'a Library,
}

impl<'a> std::fmt::Display for NameDisplayer<'a> {
    fn fmt(
        &self,
        writer: &mut std::fmt::Formatter<'_>,
    ) -> std::result::Result<(), std::fmt::Error> {
        let name = self.name.get_name(self.library);
        if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            write!(writer, "{}", name)
        } else {
            write!(writer, "\"{}\"", name)
        }
    }
}

pub trait Named {
    fn get_name(&self, library: &Library) -> Name;
}

pub trait Typed {
    fn get_type(&self, library: &Library) -> Type;
}

pub trait UniqueIndex {
    fn get_unique_index(&self) -> usize;
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "io", derive(Serialize, Deserialize))]
pub struct Type(pub(crate) generational_arena::Index);

pub struct TypeDisplayer<'a> {
    pub(crate) ty: Type,
    pub(crate) library: &'a Library,
}

impl<'a> std::fmt::Display for TypeDisplayer<'a> {
    fn fmt(
        &self,
        writer: &mut std::fmt::Formatter<'_>,
    ) -> std::result::Result<(), std::fmt::Error> {
        if self.ty.is_void(self.library) {
            write!(writer, "void")
        } else if self.ty.is_boolean(self.library) {
            write!(writer, "bool")
        } else if self.ty.is_vector(self.library) {
            write!(
                writer,
                "<{}, {}>",
                self.ty
                    .get_element(self.library, 0)
                    .get_displayer(self.library),
                self.ty.get_len(self.library)
            )
        } else if self.ty.is_array(self.library) {
            write!(
                writer,
                "[{}, {}]",
                self.ty
                    .get_element(self.library, 0)
                    .get_displayer(self.library),
                self.ty.get_len(self.library)
            )
        } else if self.ty.is_struct(self.library) {
            write!(writer, "{{")?;

            for i in 0..self.ty.get_len(self.library) {
                if i != 0 {
                    write!(writer, ", ")?;
                }

                write!(
                    writer,
                    "{}",
                    self.ty
                        .get_element(self.library, i)
                        .get_displayer(self.library)
                )?;
            }

            write!(writer, "}}")
        } else if self.ty.is_int(self.library) {
            write!(writer, "i{}", self.ty.get_bits(self.library))
        } else if self.ty.is_uint(self.library) {
            write!(writer, "u{}", self.ty.get_bits(self.library))
        } else if self.ty.is_float(self.library) {
            write!(writer, "f{}", self.ty.get_bits(self.library))
        } else if self.ty.is_pointer(self.library) {
            write!(writer, "*{}", self.ty.get_domain(self.library))
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
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let u32_ty = library.get_uint_type(32);
    /// let bits = u32_ty.get_bits(&library);
    /// # assert_eq!(bits, 32);
    /// ```
    pub fn get_bits(&self, library: &Library) -> usize {
        match library.types[self.0] {
            TypePayload::Void => 0,
            TypePayload::Int(x) => x as usize,
            TypePayload::UInt(x) => x as usize,
            TypePayload::Float(x) => x as usize,
            TypePayload::Vector(ty, width) => ty.get_bits(library) * (width as usize),
            _ => panic!("Cannot get the bit-width of type"),
        }
    }

    /// Get the length of an aggregate type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let u32_ty = library.get_uint_type(32);
    /// let vec_ty = library.get_vector_type(u32_ty, 4);
    /// assert_eq!(vec_ty.get_len(&library), 4);
    /// ```
    pub fn get_len(&self, library: &Library) -> usize {
        match &library.types[self.0] {
            TypePayload::Vector(_, width) => *width as usize,
            TypePayload::Array(_, width) => *width,
            TypePayload::Struct(vec) => vec.len(),
            _ => panic!("Cannot get the length of a non-aggregate type"),
        }
    }

    /// Get the domain from a pointer.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let u32_ty = library.get_uint_type(32);
    /// # let u32_ptr_ty = library.get_pointer_type(Domain::Cpu);
    /// let domain = u32_ptr_ty.get_domain(&library);
    /// # assert_eq!(domain, Domain::Cpu);
    /// ```
    pub fn get_domain(&self, library: &Library) -> Domain {
        match library.types[self.0] {
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
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let u32_ty = library.get_uint_type(32);
    /// # let vec_ty = library.get_vector_type(u32_ty, 4);
    /// let element = vec_ty.get_element(&library, 3);
    /// # assert_eq!(element, u32_ty);
    /// ```
    pub fn get_element(&self, library: &Library, index: usize) -> Type {
        match &library.types[self.0] {
            TypePayload::Vector(ty, size) => {
                assert!(
                    index < (*size as usize),
                    "Index is beyond the end of the vector"
                );
                *ty
            }
            TypePayload::Struct(tys) => tys[index],
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
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let u32_ty = library.get_uint_type(32);
    /// # let array_ty = library.get_array_type(u32_ty, 4);
    /// let is_array = array_ty.is_array(&library);
    /// # assert!(is_array);
    /// ```
    pub fn is_array(&self, library: &Library) -> bool {
        matches!(library.types[self.0], TypePayload::Array(_, _))
    }

    /// Checks whether a type is a struct type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let u32_ty = library.get_uint_type(32);
    /// # let struct_ty = library.get_struct_type(&[ u32_ty ]);
    /// let is_struct = struct_ty.is_struct(&library);
    /// # assert!(is_struct);
    /// ```
    pub fn is_struct(&self, library: &Library) -> bool {
        matches!(library.types[self.0], TypePayload::Struct(_))
    }

    /// Checks whether a type is a vector type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let u32_ty = library.get_uint_type(32);
    /// # let vec_ty = library.get_vector_type(u32_ty, 4);
    /// let is_vec = vec_ty.is_vector(&library);
    /// # assert!(is_vec);
    /// ```
    pub fn is_vector(&self, library: &Library) -> bool {
        matches!(library.types[self.0], TypePayload::Vector(_, _))
    }

    /// Checks whether a type is an int type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let i32_ty = library.get_int_type(32);
    /// assert!(i32_ty.is_int(&library));
    /// ```
    pub fn is_int(&self, library: &Library) -> bool {
        matches!(library.types[self.0], TypePayload::Int(_))
    }

    /// Checks whether a type is an uint type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let u32_ty = library.get_uint_type(32);
    /// assert!(u32_ty.is_uint(&library));
    /// ```
    pub fn is_uint(&self, library: &Library) -> bool {
        matches!(library.types[self.0], TypePayload::UInt(_))
    }

    /// Checks whether a type is a float type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let f32_ty = library.get_float_type(32);
    /// assert!(f32_ty.is_float(&library));
    /// ```
    pub fn is_float(&self, library: &Library) -> bool {
        matches!(library.types[self.0], TypePayload::Float(_))
    }

    /// Checks whether a type is an integral (signed or unsigned) type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let u32_ty = library.get_uint_type(32);
    /// # let vec_ty = library.get_vector_type(u32_ty, 4);
    /// let is_integral = u32_ty.is_integral(&library);
    /// # assert!(is_integral);
    /// # assert!(!vec_ty.is_integral(&library));
    /// ```
    pub fn is_integral(&self, library: &Library) -> bool {
        matches!(
            library.types[self.0],
            TypePayload::Int(_) | TypePayload::UInt(_)
        )
    }

    /// Checks whether a type is an integral (signed or unsigned) type, or a vector of integral.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let u32_ty = library.get_uint_type(32);
    /// # let vec_ty = library.get_vector_type(u32_ty, 4);
    /// # let bool_ty = library.get_bool_type();
    /// let is_integral = u32_ty.is_integral_or_integral_vector(&library);
    /// let is_vector_integral = vec_ty.is_integral_or_integral_vector(&library);
    /// # assert!(is_integral);
    /// # assert!(is_vector_integral);
    /// # assert!(!bool_ty.is_integral_or_integral_vector(&library));
    /// ```
    pub fn is_integral_or_integral_vector(&self, library: &Library) -> bool {
        let mut ty = *self;

        if ty.is_vector(library) {
            ty = ty.get_element(library, 0);
        }

        ty.is_integral(library)
    }

    /// Checks whether a type is a float type, or a vector of float.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let f32_ty = library.get_float_type(32);
    /// # let vec_ty = library.get_vector_type(f32_ty, 4);
    /// # let bool_ty = library.get_bool_type();
    /// let is_float = f32_ty.is_float_or_float_vector(&library);
    /// let is_vector_float = vec_ty.is_float_or_float_vector(&library);
    /// # assert!(is_float);
    /// # assert!(is_vector_float);
    /// # assert!(!bool_ty.is_float_or_float_vector(&library));
    /// ```
    pub fn is_float_or_float_vector(&self, library: &Library) -> bool {
        let mut ty = *self;

        if ty.is_vector(library) {
            ty = ty.get_element(library, 0);
        }

        ty.is_float(library)
    }

    /// Checks whether a type is a float type, or a vector of float.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let f32_ty = library.get_float_type(32);
    /// # let bool_ty = library.get_bool_type();
    /// # let vec_ty = library.get_vector_type(bool_ty, 4);
    /// let is_bool = bool_ty.is_bool_or_bool_vector(&library);
    /// let is_vector_bool = vec_ty.is_bool_or_bool_vector(&library);
    /// # assert!(is_bool);
    /// # assert!(is_vector_bool);
    /// # assert!(!f32_ty.is_bool_or_bool_vector(&library));
    /// ```
    pub fn is_bool_or_bool_vector(&self, library: &Library) -> bool {
        let mut ty = *self;

        if ty.is_vector(library) {
            ty = ty.get_element(library, 0);
        }

        ty.is_boolean(library)
    }

    /// Checks whether a type is a boolean type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let u32_ty = library.get_uint_type(32);
    /// # let vec_ty = library.get_vector_type(u32_ty, 4);
    /// # let bool_ty = library.get_bool_type();
    /// let is_boolean = bool_ty.is_boolean(&library);
    /// # assert!(is_boolean);
    /// # assert!(!vec_ty.is_boolean(&library));
    /// ```
    pub fn is_boolean(&self, library: &Library) -> bool {
        matches!(library.types[self.0], TypePayload::Bool)
    }

    /// Checks whether a type is a void type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let u32_ty = library.get_uint_type(32);
    /// # let vec_ty = library.get_vector_type(u32_ty, 4);
    /// # let void_ty = library.get_void_type();
    /// assert!(void_ty.is_void(&library));
    /// # assert!(!vec_ty.is_boolean(&library));
    /// ```
    pub fn is_void(&self, library: &Library) -> bool {
        matches!(library.types[self.0], TypePayload::Void)
    }

    /// Checks whether a type is a pointer type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let bool_ty = library.get_bool_type();
    /// # let ptr_ty = library.get_pointer_type(Domain::Cpu);
    /// let is_pointer = ptr_ty.is_pointer(&library);
    /// # assert!(is_pointer);
    /// # assert!(!bool_ty.is_pointer(&library));
    /// ```
    pub fn is_pointer(&self, library: &Library) -> bool {
        matches!(library.types[self.0], TypePayload::Pointer(_))
    }

    /// Get the type at the index into the type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut library = Library::new();
    /// # let module = library.create_module().build();
    /// # let u32_ty = library.get_uint_type(32);
    /// # let ty = library.get_array_type(u32_ty, 42);
    /// # let index = library.get_int_constant(8, 0);
    /// let indexed_type = ty.get_indexed(&library, index);
    /// # assert_eq!(indexed_type, u32_ty);
    /// ```
    pub fn get_indexed(&self, library: &Library, index: Value) -> Type {
        match &library.types[self.0] {
            TypePayload::Array(ty, _) => *ty,
            TypePayload::Struct(tys) => {
                assert!(
                    index.is_constant(library),
                    "Cannot index into a struct with a non-constant"
                );

                match index.get_constant(library) {
                    Constant::Int(c, _) => tys[*c as usize],
                    Constant::UInt(c, _) => tys[*c as usize],
                    _ => panic!("Cannot index into a struct with a non-integral constant"),
                }
            }
            _ => panic!("Unable to index into type!"),
        }
    }

    pub fn get_displayer<'a>(&self, library: &'a Library) -> TypeDisplayer<'a> {
        TypeDisplayer { ty: *self, library }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn bad_int_ty() {
        let mut library = Library::new();
        let _ = library.get_int_type(31);
    }

    #[test]
    #[should_panic]
    fn bad_uint_ty() {
        let mut library = Library::new();
        let _ = library.get_uint_type(13);
    }

    #[test]
    #[should_panic]
    fn bad_vec_ty_element() {
        let mut library = Library::new();
        let void_ty = library.get_void_type();
        let _ = library.get_vector_type(void_ty, 4);
    }

    #[test]
    #[should_panic]
    fn bad_vec_ty_width() {
        let mut library = Library::new();
        let u32_ty = library.get_uint_type(32);
        let _ = library.get_vector_type(u32_ty, 1);
    }

    #[test]
    #[should_panic]
    fn bad_get_bits() {
        let mut library = Library::new();
        let ptr_ty = library.get_pointer_type(Domain::Cpu);
        let _ = ptr_ty.get_bits(&library);
    }

    #[test]
    #[should_panic]
    fn bad_get_element_index() {
        let mut library = Library::new();
        let u32_ty = library.get_uint_type(32);
        let vec_ty = library.get_vector_type(u32_ty, 4);
        vec_ty.get_element(&library, 5);
    }

    #[test]
    #[should_panic]
    fn bad_get_element_type() {
        let mut library = Library::new();
        let u32_ty = library.get_uint_type(32);
        u32_ty.get_element(&library, 0);
    }
}
