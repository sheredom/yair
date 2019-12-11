use crate::*;
use std::hash::Hash;
use std::hash::Hasher;

#[derive(Clone, Debug, PartialEq)]
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
                bytemuck::cast::<f64, u64>(*val).hash(state);
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

impl Typed for Constant {
    /// Get the type of a constant.
    ///
    /// # Examples
    ///
    /// ```
    /// # use yair::*;
    /// # let mut module = Module::create_module().build();
    /// # let constant = module.get_bool_constant(true);
    /// let ty = constant.get_type(&module);
    /// ```
    fn get_type(&self, _: &Module) -> Type {
        match self {
            Constant::Bool(_, ty) => *ty,
            Constant::Int(_, ty) => *ty,
            Constant::UInt(_, ty) => *ty,
            Constant::Float(_, ty) => *ty,
            Constant::Composite(_, ty) => *ty,
        }
    }
}
