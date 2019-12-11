extern crate bytemuck;
extern crate generational_arena;

mod argument;
mod block;
mod constant;
mod function;
mod global;
mod instructions;
mod location;
mod module;
mod value;

pub use argument::*;
pub use block::*;
pub use constant::*;
pub use function::*;
pub use global::*;
pub use instructions::*;
pub use location::*;
pub use module::*;
pub use value::*;

/// The domain that a memory location inhabits. Used by cross-function variables
/// and pointer types to encode where the memory resides.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Domain {
    CPU_AND_GPU,
    CPU,
    GPU,
    STACK,
}

#[derive(Debug)]
enum TypePayload {
    Void,
    Bool,
    Int(u8),
    UInt(u8),
    Float(u8),
    Vector(Type, u8),
    Pointer(Type, Domain),
    Struct(Vec<Type>),
    Function(Type, Vec<Type>),
    Array(Type, usize),
}

impl Default for TypePayload {
    fn default() -> Self {
        TypePayload::Void
    }
}

pub trait Typed {
    fn get_type(&self, module: &Module) -> Type;
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Type(pub(crate) generational_arena::Index);

impl Type {
    /// Get the number of bits required to represent the type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// let bits = u32_ty.get_bits(&module);
    /// # assert_eq!(bits, 32);
    /// ```
    pub fn get_bits(&self, module: &Module) -> usize {
        match module.types[self.0] {
            TypePayload::Void => 0,
            TypePayload::Int(x) => x as usize,
            TypePayload::UInt(x) => x as usize,
            TypePayload::Float(x) => x as usize,
            TypePayload::Vector(ty, width) => ty.get_bits(module) * (width as usize),
            _ => panic!("Cannot get the bit-width of type"),
        }
    }

    /// Get the pointed-to type from a pointer (the pointee type).
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let u32_ptr_ty = module.get_ptr_type(u32_ty, Domain::CPU);
    /// let pointee = u32_ptr_ty.get_pointee(&module);
    /// # assert_eq!(pointee, u32_ty);
    /// ```
    pub fn get_pointee(&self, module: &Module) -> Type {
        match module.types[self.0] {
            TypePayload::Pointer(ty, _) => ty,
            _ => panic!("Cannot get the pointee from a non pointer type"),
        }
    }

    /// Get the domain from a pointer.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let u32_ptr_ty = module.get_ptr_type(u32_ty, Domain::CPU);
    /// let domain = u32_ptr_ty.get_domain(&module);
    /// # assert_eq!(domain, Domain::CPU);
    /// ```
    pub fn get_domain(&self, module: &Module) -> Domain {
        match module.types[self.0] {
            TypePayload::Pointer(_, domain) => domain,
            _ => panic!("Cannot get the domain from a non pointer type"),
        }
    }

    /// Get the element from a vector or a struct type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let vec_ty = module.get_vec_type(u32_ty, 4);
    /// let element = vec_ty.get_element(&module, 3);
    /// # assert_eq!(element, u32_ty);
    /// ```
    pub fn get_element(&self, module: &Module, index: usize) -> Type {
        match &module.types[self.0] {
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
            _ => panic!("Cannot get the element from type"),
        }
    }

    /// Checks whether a type is a vector type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let vec_ty = module.get_vec_type(u32_ty, 4);
    /// let is_vec = vec_ty.is_vector(&module);
    /// # assert!(is_vec);
    /// ```
    pub fn is_vector(&self, module: &Module) -> bool {
        match module.types[self.0] {
            TypePayload::Vector(_, _) => true,
            _ => false,
        }
    }

    /// Checks whether a type is an integral (signed or unsigned) type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let vec_ty = module.get_vec_type(u32_ty, 4);
    /// let is_integral = u32_ty.is_integral(&module);
    /// # assert!(is_integral);
    /// # assert!(!vec_ty.is_integral(&module));
    /// ```
    pub fn is_integral(&self, module: &Module) -> bool {
        match module.types[self.0] {
            TypePayload::Int(_) => true,
            TypePayload::UInt(_) => true,
            _ => false,
        }
    }

    /// Checks whether a type is an integral (signed or unsigned) type, or a vector of integral.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let vec_ty = module.get_vec_type(u32_ty, 4);
    /// # let bool_ty = module.get_bool_ty();
    /// let is_integral = u32_ty.is_integral_or_integral_vector(&module);
    /// let is_vector_integral = vec_ty.is_integral_or_integral_vector(&module);
    /// # assert!(is_integral);
    /// # assert!(is_vector_integral);
    /// # assert!(!bool_ty.is_integral(&module));
    /// ```
    pub fn is_integral_or_integral_vector(&self, module: &Module) -> bool {
        let mut ty = *self;

        if ty.is_vector(module) {
            ty = ty.get_element(module, 0);
        }

        ty.is_integral(module)
    }

    /// Checks whether a type is a boolean type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let vec_ty = module.get_vec_type(u32_ty, 4);
    /// # let bool_ty = module.get_bool_ty();
    /// let is_boolean = bool_ty.is_boolean(&module);
    /// # assert!(is_boolean);
    /// # assert!(!vec_ty.is_boolean(&module));
    /// ```
    pub fn is_boolean(&self, module: &Module) -> bool {
        match module.types[self.0] {
            TypePayload::Bool => true,
            _ => false,
        }
    }

    /// Checks whether a type is a pointer type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let bool_ty = module.get_bool_ty();
    /// # let ptr_ty = module.get_ptr_type(bool_ty, Domain::CPU);
    /// let is_ptr = ptr_ty.is_ptr(&module);
    /// # assert!(is_ptr);
    /// # assert!(!bool_ty.is_ptr(&module));
    /// ```
    pub fn is_ptr(&self, module: &Module) -> bool {
        match module.types[self.0] {
            TypePayload::Pointer(_, _) => true,
            _ => false,
        }
    }

    /// Get the type at the index into the type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let u32_ty = module.get_uint_type(32);
    /// # let ty = module.get_array_ty(u32_ty, 42);
    /// # let index = module.get_int_constant(8, 0);
    /// let indexed_type = ty.get_indexed(&module, index);
    /// # assert_eq!(indexed_type, u32_ty);
    /// ```
    pub fn get_indexed(&self, module: &Module, index: Value) -> Type {
        match &module.types[self.0] {
            TypePayload::Array(ty, _) => *ty,
            TypePayload::Struct(tys) => {
                assert!(
                    index.is_constant(module),
                    "Cannot index into a struct with a non-constant"
                );

                match index.get_constant(module) {
                    Constant::Int(c, _) => tys[*c as usize],
                    Constant::UInt(c, _) => tys[*c as usize],
                    _ => panic!("Cannot index into a struct with a non-integral constant"),
                }
            }
            _ => panic!("Unable to index into type!"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn bad_int_ty() {
        let mut module = Module::create_module().build();
        let _ = module.get_int_type(31);
    }

    #[test]
    #[should_panic]
    fn bad_uint_ty() {
        let mut module = Module::create_module().build();
        let _ = module.get_uint_type(13);
    }

    #[test]
    #[should_panic]
    fn bad_vec_ty_element() {
        let mut module = Module::create_module().build();
        let void_ty = module.get_void_type();
        let _ = module.get_vec_type(void_ty, 4);
    }

    #[test]
    #[should_panic]
    fn bad_vec_ty_width() {
        let mut module = Module::create_module().build();
        let u32_ty = module.get_uint_type(32);
        let _ = module.get_vec_type(u32_ty, 1);
    }

    #[test]
    #[should_panic]
    fn bad_get_bits() {
        let mut module = Module::create_module().build();
        let u32_ty = module.get_uint_type(32);
        let u32_ptr_ty = module.get_ptr_type(u32_ty, Domain::CPU);
        let _ = u32_ptr_ty.get_bits(&module);
    }

    #[test]
    #[should_panic]
    fn bad_get_pointee() {
        let mut module = Module::create_module().build();
        let u32_ty = module.get_uint_type(32);
        let _ = u32_ty.get_pointee(&module);
    }

    #[test]
    #[should_panic]
    fn bad_get_element_index() {
        let mut module = Module::create_module().build();
        let u32_ty = module.get_uint_type(32);
        let vec_ty = module.get_vec_type(u32_ty, 4);
        vec_ty.get_element(&module, 5);
    }

    #[test]
    #[should_panic]
    fn bad_get_element_type() {
        let mut module = Module::create_module().build();
        let u32_ty = module.get_uint_type(32);
        u32_ty.get_element(&module, 0);
    }
}

// STRUCTS
// GLOBALS
