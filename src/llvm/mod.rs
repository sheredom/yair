use crate::*;
use libc::*;
use llvm_sys::core::*;
use llvm_sys::prelude::*;
use llvm_sys::*;
use std::io::{Seek, Write};

pub struct Llvm {
    context: LLVMContextRef,
}

pub enum Error {}

impl Llvm {
    fn new() -> Self {
        Self {
            context: unsafe { core::LLVMContextCreate() },
        }
    }

    fn make_type(&self, library: &Library, ty: Type) -> Result<LLVMTypeRef, Error> {
        if ty.is_void(library) {
            Ok(unsafe { LLVMVoidTypeInContext(self.context) })
        } else if ty.is_boolean(library) {
            Ok(unsafe { LLVMInt1TypeInContext(self.context) })
        } else if ty.is_int(library) || ty.is_uint(library) {
            Ok(unsafe { LLVMIntTypeInContext(self.context, ty.get_bits(library) as c_uint) })
        } else if ty.is_float(library) {
            match ty.get_bits(library) {
                16 => Ok(unsafe { LLVMHalfTypeInContext(self.context) }),
                32 => Ok(unsafe { LLVMFloatTypeInContext(self.context) }),
                64 => Ok(unsafe { LLVMDoubleTypeInContext(self.context) }),
                _ => unreachable!(),
            }
        } else if ty.is_pointer(library) {
            Ok(unsafe { LLVMPointerType(LLVMInt8TypeInContext(self.context), 0 as c_uint) })
        } else if ty.is_array(library) {
            let len = ty.get_len(library);
            let element = self.make_type(library, ty.get_element(library, 0))?;
            Ok(unsafe { core::LLVMArrayType(element, len as c_uint) })
        } else if ty.is_vector(library) {
            let len = ty.get_len(library);
            let element = self.make_type(library, ty.get_element(library, 0))?;
            Ok(unsafe { core::LLVMVectorType(element, len as c_uint) })
        } else if ty.is_struct(library) {
            let mut elements = Vec::new();
            for i in 0..ty.get_len(library) {
                elements.push(self.make_type(library, ty.get_element(library, i))?);
            }

            Ok(unsafe {
                core::LLVMStructTypeInContext(
                    self.context,
                    elements.as_mut_ptr(),
                    elements.len() as c_uint,
                    true as LLVMBool,
                )
            })
        } else {
            panic!("Unknown type!");
        }
    }

    fn make_module(&self, library: &Library) -> Result<LLVMModule, Error> {
        for module in library.get_modules() {
            for global in module.get_globals(library) {
                let ty = self.make_type(library, global.get_type(library));
            }
        }

        todo!();
    }
}

impl CodeGen for Llvm {
    type Error = Error;

    fn generate<W: Seek + Write>(library: &Library, _writer: &mut W) -> Result<(), Self::Error> {
        let codegen = Self::new();

        let llvm_module = codegen.make_module(library)?;

        todo!();
    }
}
