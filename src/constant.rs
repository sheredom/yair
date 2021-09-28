use crate::*;
use std::hash::Hash;
use std::hash::Hasher;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "io", derive(Serialize, Deserialize))]
pub enum Constant {
    Bool(bool, Type),
    Int(i64, Type),
    UInt(u64, Type),
    Float(f64, Type),
    Pointer(Type),
    Composite(Vec<Value>, Type),
}

impl Eq for Constant {}

impl Hash for Constant {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Constant::Bool(val, ty) => {
                val.hash(state);
                ty.hash(state);
            }
            Constant::Int(val, ty) => {
                val.hash(state);
                ty.hash(state);
            }
            Constant::UInt(val, ty) => {
                val.hash(state);
                ty.hash(state);
            }
            Constant::Float(val, ty) => {
                (*val).to_bits().hash(state);
                ty.hash(state);
            }
            Constant::Pointer(ty) => {
                ty.hash(state);
            }
            Constant::Composite(vec, ty) => {
                for val in vec {
                    val.hash(state);
                }
                ty.hash(state);
            }
        }
    }
}

impl PartialEq for Constant {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Constant::Bool(a, a_ty) => match other {
                Constant::Bool(b, b_ty) => a == b && a_ty == b_ty,
                _ => false,
            },
            Constant::Int(a, a_ty) => match other {
                Constant::Int(b, b_ty) => a == b && a_ty == b_ty,
                _ => false,
            },
            Constant::UInt(a, a_ty) => match other {
                Constant::UInt(b, b_ty) => a == b && a_ty == b_ty,
                _ => false,
            },
            Constant::Float(a, a_ty) => match other {
                Constant::Float(b, b_ty) => a == b && a_ty == b_ty,
                _ => false,
            },
            Constant::Pointer(a_ty) => match other {
                Constant::Pointer(b_ty) => a_ty == b_ty,
                _ => false,
            },
            Constant::Composite(a, a_ty) => match other {
                Constant::Composite(b, b_ty) => a == b && a_ty == b_ty,
                _ => false,
            },
        }
    }
}

impl Typed for Constant {
    /// Get the type of a constant.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut context = Context::new();
    /// # let constant = context.get_bool_constant(true);
    /// let ty = constant.get_type(&context);
    /// ```
    fn get_type(&self, _: &Context) -> Type {
        match self {
            Constant::Bool(_, ty) => *ty,
            Constant::Int(_, ty) => *ty,
            Constant::UInt(_, ty) => *ty,
            Constant::Float(_, ty) => *ty,
            Constant::Pointer(ty) => *ty,
            Constant::Composite(_, ty) => *ty,
        }
    }
}
