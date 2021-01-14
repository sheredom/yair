use crate::*;
use std::hash::Hash;
use std::hash::Hasher;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Constant {
    Bool(bool, Type),
    Int(i64, Type),
    UInt(u64, Type),
    Float(f64, Type),
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
                unsafe {
                    std::mem::transmute::<f64, u64>(*val)
                }.hash(state);
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
            Constant::Bool(a, _) => match other {
                Constant::Bool(b, _) => a == b,
                _ => false,
            },
            Constant::Int(a, _) => match other {
                Constant::Int(b, _) => a == b,
                _ => false,
            },
            Constant::UInt(a, _) => match other {
                Constant::UInt(b, _) => a == b,
                _ => false,
            },
            Constant::Float(a, _) => match other {
                Constant::Float(b, _) => a == b,
                _ => false,
            },
            Constant::Composite(a, _) => match other {
                Constant::Composite(b, _) => a == b,
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
    /// # let mut library = Library::new();
    /// # let constant = library.get_bool_constant(true);
    /// let ty = constant.get_type(&library);
    /// ```
    fn get_type(&self, _: &Library) -> Type {
        match self {
            Constant::Bool(_, ty) => *ty,
            Constant::Int(_, ty) => *ty,
            Constant::UInt(_, ty) => *ty,
            Constant::Float(_, ty) => *ty,
            Constant::Composite(_, ty) => *ty,
        }
    }
}
