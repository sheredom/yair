use crate::*;
use libc::*;
use llvm_sys::core::*;
use llvm_sys::prelude::*;
use llvm_sys::*;
use std::collections::HashMap;
use std::ffi::CString;
use std::io::{Seek, Write};
use std::path::Path;
use std::ptr;

const empty_name: *const libc::c_char = b"\0".as_ptr() as *const libc::c_char;

const DW_ATE_VOID: debuginfo::LLVMDWARFTypeEncoding = 0x00;
const DW_ATE_BOOLEAN: debuginfo::LLVMDWARFTypeEncoding = 0x02;
const DW_ATE_FLOAT: debuginfo::LLVMDWARFTypeEncoding = 0x04;
const DW_ATE_SIGNED: debuginfo::LLVMDWARFTypeEncoding = 0x05;
const DW_ATE_UNSIGNED: debuginfo::LLVMDWARFTypeEncoding = 0x07;

pub struct Llvm {
    context: LLVMContextRef,
    dibuilder: LLVMDIBuilderRef,
    ditypes: HashMap<Type, LLVMMetadataRef>,
    types: HashMap<Type, LLVMTypeRef>,
    filenames: HashMap<Name, LLVMMetadataRef>,
    current_module: LLVMModuleRef,
    function_map: HashMap<Function, LLVMValueRef>,
    block_map: HashMap<Block, LLVMBasicBlockRef>,
    value_map: HashMap<Value, LLVMValueRef>,
    triple: *const libc::c_char,
    target: target::LLVMTargetDataRef,
}

pub enum Error {}

impl Llvm {
    fn new(platform: CodeGenPlatform) -> Self {
        let context = unsafe { core::LLVMContextCreate() };

        let triple = match platform {
            CodeGenPlatform::MacOsAppleSilicon => b"aarch64-apple-darwin\0",
            _ => panic!("Unknown platform"),
        }
        .as_ptr() as *const libc::c_char;

        let target = unsafe { target::LLVMCreateTargetData(triple) };

        Self {
            context,
            dibuilder: std::ptr::null_mut(),
            ditypes: HashMap::new(),
            types: HashMap::new(),
            filenames: HashMap::new(),
            current_module: ptr::null_mut(),
            function_map: HashMap::new(),
            block_map: HashMap::new(),
            value_map: HashMap::new(),
            triple,
            target,
        }
    }

    fn get_or_insert_filename(&mut self, library: &Library, name: Name) -> LLVMMetadataRef {
        if let Some(filename) = self.filenames.get(&name) {
            *filename
        } else {
            let name_str = name.get_name(library);

            let filename = Path::new(name_str).file_name().unwrap().to_str().unwrap();
            let directory = name_str.strip_suffix(filename).unwrap();

            let metadata = unsafe {
                debuginfo::LLVMDIBuilderCreateFile(
                    self.dibuilder,
                    filename.as_ptr() as *const libc::c_char,
                    filename.len(),
                    directory.as_ptr() as *const libc::c_char,
                    directory.len(),
                )
            };

            self.filenames.insert(name, metadata);

            metadata
        }
    }

    fn make_location(&mut self, library: &Library, location: Location) -> LLVMMetadataRef {
        let line = location.get_line() as libc::c_uint;
        let column = location.get_column() as libc::c_uint;

        let filename = self.get_or_insert_filename(library, location.get_name(library));

        unsafe {
            debuginfo::LLVMDIBuilderCreateDebugLocation(
                self.context,
                line,
                column,
                filename,
                ptr::null_mut(),
            )
        }
    }

    fn get_or_insert_type(&mut self, library: &Library, ty: Type) -> Result<LLVMTypeRef, Error> {
        if let Some(t) = self.types.get(&ty) {
            Ok(*t)
        } else {
            let llvm_ty = self.make_type(library, ty)?;
            self.types.insert(ty, llvm_ty);

            Ok(llvm_ty)
        }
    }

    fn get_or_insert_debug_type(
        &mut self,
        library: &Library,
        ty: Type,
    ) -> Result<LLVMMetadataRef, Error> {
        if !self.ditypes.contains_key(&ty) {
            let llvm_dity = self.insert_debug_type(library, ty)?;
            self.ditypes.insert(ty, llvm_dity);
        }

        Ok(*self.ditypes.get(&ty).unwrap())
    }

    fn insert_debug_type(&mut self, library: &Library, ty: Type) -> Result<LLVMMetadataRef, Error> {
        if ty.is_named_struct(library) {
            let filename = if let Some(location) = ty.get_location(library) {
                self.get_or_insert_filename(library, location.get_name(library))
            } else {
                ptr::null_mut()
            };

            let line = if let Some(location) = ty.get_location(library) {
                location.get_line()
            } else {
                0
            };

            let location = if let Some(location) = ty.get_location(library) {
                self.make_location(library, location)
            } else {
                ptr::null_mut()
            };

            let name = ty.get_name(library).get_name(library);
            let size = unsafe {
                target::LLVMABISizeOfType(self.target, self.get_or_insert_type(library, ty)?)
            };
            let mut elements = Vec::new();
            let runtimelang = 0; // C/C++ will have to do for now

            debug_assert!(false, "Need to get a target and the sizeof of the struct");

            Ok(unsafe {
                debuginfo::LLVMDIBuilderCreateStructType(
                    self.dibuilder,
                    location,
                    name.as_ptr() as *const libc::c_char,
                    name.len(),
                    filename,
                    line as libc::c_uint,
                    size,
                    16,
                    Default::default(),
                    ptr::null_mut(),
                    elements.as_mut_ptr(),
                    elements.len() as u32,
                    runtimelang,
                    ptr::null_mut(),
                    ptr::null(),
                    0,
                )
            })
        } else if ty.is_void(library) {
            let name = "void";
            Ok(unsafe {
                debuginfo::LLVMDIBuilderCreateBasicType(
                    self.dibuilder,
                    name.as_ptr() as *const libc::c_char,
                    name.len(),
                    0,
                    DW_ATE_VOID,
                    0,
                )
            })
        } else if ty.is_boolean(library) {
            let name = "bool";
            Ok(unsafe {
                debuginfo::LLVMDIBuilderCreateBasicType(
                    self.dibuilder,
                    name.as_ptr() as *const libc::c_char,
                    name.len(),
                    1,
                    DW_ATE_BOOLEAN,
                    0,
                )
            })
        } else if ty.is_int(library) {
            let bits = ty.get_bits(library);
            let name = "i".to_owned() + &bits.to_string();
            Ok(unsafe {
                debuginfo::LLVMDIBuilderCreateBasicType(
                    self.dibuilder,
                    name.as_ptr() as *const libc::c_char,
                    name.len(),
                    bits as u64,
                    DW_ATE_SIGNED,
                    0,
                )
            })
        } else if ty.is_uint(library) {
            let bits = ty.get_bits(library);
            let name = "i".to_owned() + &bits.to_string();
            Ok(unsafe {
                debuginfo::LLVMDIBuilderCreateBasicType(
                    self.dibuilder,
                    name.as_ptr() as *const libc::c_char,
                    name.len(),
                    bits as u64,
                    DW_ATE_UNSIGNED,
                    0,
                )
            })
        } else if ty.is_float(library) {
            let bits = ty.get_bits(library);
            let name = "f".to_owned() + &bits.to_string();
            Ok(unsafe {
                debuginfo::LLVMDIBuilderCreateBasicType(
                    self.dibuilder,
                    name.as_ptr() as *const libc::c_char,
                    name.len(),
                    bits as u64,
                    DW_ATE_FLOAT,
                    0,
                )
            })
        } else if ty.is_pointer(library) {
            let name = "void";
            let void_ty = unsafe {
                debuginfo::LLVMDIBuilderCreateBasicType(
                    self.dibuilder,
                    name.as_ptr() as *const libc::c_char,
                    name.len(),
                    0,
                    DW_ATE_VOID,
                    0,
                )
            };

            let domain = ty.get_domain(library);
            let name = domain.to_string();
            let size = unsafe {
                target::LLVMABISizeOfType(self.target, self.get_or_insert_type(library, ty)?)
            };
            let address_space = 0; // TODO: We should query this from the target!

            Ok(unsafe {
                debuginfo::LLVMDIBuilderCreatePointerType(
                    self.dibuilder,
                    void_ty,
                    size,
                    1,
                    address_space,
                    name.as_ptr() as *const libc::c_char,
                    name.len(),
                )
            })
        } else if ty.is_array(library) {
            let len = ty.get_len(library);
            let element_ty = ty.get_element(library, 0);

            let mut subscripts = Vec::new();

            debug_assert!(false, "subscripts need to be filled out!");

            Ok(unsafe {
                debuginfo::LLVMDIBuilderCreateArrayType(
                    self.dibuilder,
                    len as u64,
                    1,
                    self.get_or_insert_debug_type(library, element_ty)?,
                    subscripts.as_mut_ptr(),
                    subscripts.len() as u32,
                )
            })
        } else if ty.is_vector(library) {
            let len = ty.get_len(library);
            let element_ty = ty.get_element(library, 0);

            let mut subscripts = Vec::new();

            debug_assert!(false, "subscripts need to be filled out!");

            Ok(unsafe {
                debuginfo::LLVMDIBuilderCreateVectorType(
                    self.dibuilder,
                    len as u64,
                    1,
                    self.get_or_insert_debug_type(library, element_ty)?,
                    subscripts.as_mut_ptr(),
                    subscripts.len() as u32,
                )
            })
        } else if ty.is_struct(library) {
            panic!("Implement structs");
        } else {
            panic!("Unknown type!");
        }
    }

    fn make_type(&mut self, library: &Library, ty: Type) -> Result<LLVMTypeRef, Error> {
        if ty.is_named_struct(library) {
            let name_cstr = CString::new(ty.get_name(library).get_name(library)).unwrap();
            let name = name_cstr.as_ptr() as *const libc::c_char;

            let struct_type = unsafe { core::LLVMStructCreateNamed(self.context, name) };

            let mut elements = Vec::new();
            for i in 0..ty.get_len(library) {
                elements.push(self.get_or_insert_type(library, ty.get_element(library, i))?);
            }

            unsafe {
                core::LLVMStructSetBody(
                    struct_type,
                    elements.as_mut_ptr(),
                    elements.len() as c_uint,
                    true as LLVMBool,
                );
            }

            Ok(struct_type)
        } else if ty.is_void(library) {
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
            Ok(unsafe { LLVMPointerType(LLVMInt8TypeInContext(self.context), 0) })
        } else if ty.is_array(library) {
            let len = ty.get_len(library);
            let element = self.get_or_insert_type(library, ty.get_element(library, 0))?;
            Ok(unsafe { core::LLVMArrayType(element, len as c_uint) })
        } else if ty.is_vector(library) {
            let len = ty.get_len(library);
            let element = self.get_or_insert_type(library, ty.get_element(library, 0))?;
            Ok(unsafe { core::LLVMVectorType(element, len as c_uint) })
        } else if ty.is_struct(library) {
            let mut elements = Vec::new();
            for i in 0..ty.get_len(library) {
                elements.push(self.get_or_insert_type(library, ty.get_element(library, i))?);
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

    fn make_function_declaration(
        &mut self,
        library: &Library,
        function: Function,
    ) -> Result<LLVMValueRef, Error> {
        // TODO: I should really make this do the ABI transformation...

        let mut elements = Vec::new();

        for arg in function.get_args(library) {
            elements.push(self.get_or_insert_type(library, arg.get_type(library))?);
        }

        let return_type = self.get_or_insert_type(library, function.get_return_type(library))?;

        let function_type = unsafe {
            core::LLVMFunctionType(
                return_type,
                elements.as_mut_ptr(),
                elements.len() as libc::c_uint,
                0,
            )
        };

        let name_cstr = CString::new(function.get_name(library).get_name(library)).unwrap();
        let name = name_cstr.as_ptr() as *const libc::c_char;

        Ok(unsafe { core::LLVMAddFunction(self.current_module, name, function_type) })
    }

    fn add_function_body(
        &mut self,
        library: &Library,
        function: Function,
        llvm_function: LLVMValueRef,
    ) -> Result<(), Error> {
        let builder = unsafe { core::LLVMCreateBuilderInContext(self.context) };

        for block in function.get_blocks(library) {
            let llvm_block = unsafe {
                core::LLVMAppendBasicBlockInContext(self.context, llvm_function, empty_name)
            };

            self.block_map.insert(block, llvm_block);

            if function.is_entry_block(library, block) {
                for (function_argument, block_argument) in
                    function.get_args(library).zip(block.get_args(library))
                {
                    let llvm_function_argument = *self.value_map.get(&function_argument).unwrap();
                    self.value_map
                        .insert(block_argument, llvm_function_argument);
                }
            } else {
                for argument in block.get_args(library) {
                    unsafe { core::LLVMPositionBuilderAtEnd(builder, llvm_block) };

                    let llvm_ty = self.get_or_insert_type(library, argument.get_type(library))?;

                    let llvm_phi = unsafe { core::LLVMBuildPhi(builder, llvm_ty, empty_name) };

                    self.value_map.insert(argument, llvm_phi);
                }
            }
        }

        for block in function.get_blocks(library) {
            let llvm_block = *self.block_map.get(&block).unwrap();

            self.add_block_body(library, function, block, llvm_block, builder)?;
        }

        Ok(())
    }

    fn get_or_insert_value(
        &mut self,
        library: &Library,
        value: Value,
    ) -> Result<LLVMValueRef, Error> {
        if !self.value_map.contains_key(&value) {
            if value.is_constant(library) {
                match value.get_constant(library) {
                    Constant::Bool(constant, ty) => {
                        let llvm_ty = self.get_or_insert_type(library, *ty)?;
                        let llvm_value = unsafe {
                            core::LLVMConstInt(llvm_ty, *constant as libc::c_ulonglong, 1)
                        };
                        self.value_map.insert(value, llvm_value);
                    }
                    Constant::Composite(values, ty) => {
                        let llvm_ty = self.get_or_insert_type(library, *ty)?;

                        let mut llvm_values = Vec::new();

                        for value in values {
                            llvm_values.push(self.get_or_insert_value(library, *value)?);
                        }

                        let llvm_value = if ty.is_array(library) {
                            let element_ty = ty.get_element(library, 0);
                            let llvm_element_ty = self.get_or_insert_type(library, element_ty)?;
                            unsafe {
                                core::LLVMConstArray(
                                    llvm_element_ty,
                                    llvm_values.as_mut_ptr(),
                                    llvm_values.len() as libc::c_uint,
                                )
                            }
                        } else if ty.is_vector(library) {
                            unsafe {
                                core::LLVMConstVector(
                                    llvm_values.as_mut_ptr(),
                                    llvm_values.len() as libc::c_uint,
                                )
                            }
                        } else if ty.is_struct(library) {
                            unsafe {
                                core::LLVMConstStruct(
                                    llvm_values.as_mut_ptr(),
                                    llvm_values.len() as libc::c_uint,
                                    0,
                                )
                            }
                        } else {
                            unreachable!()
                        };

                        self.value_map.insert(value, llvm_value);
                    }
                    Constant::Float(constant, ty) => {
                        let llvm_ty = self.get_or_insert_type(library, *ty)?;
                        let llvm_value =
                            unsafe { core::LLVMConstReal(llvm_ty, *constant as libc::c_double) };
                        self.value_map.insert(value, llvm_value);
                    }
                    Constant::Int(constant, ty) => {
                        let llvm_ty = self.get_or_insert_type(library, *ty)?;
                        let llvm_value = unsafe {
                            core::LLVMConstInt(llvm_ty, *constant as libc::c_ulonglong, 1)
                        };
                        self.value_map.insert(value, llvm_value);
                    }
                    Constant::Pointer(ty) => {
                        let llvm_ty = self.get_or_insert_type(library, *ty)?;
                        let llvm_value = unsafe { core::LLVMConstPointerNull(llvm_ty) };
                        self.value_map.insert(value, llvm_value);
                    }
                    Constant::UInt(constant, ty) => {
                        let llvm_ty = self.get_or_insert_type(library, *ty)?;
                        let llvm_value = unsafe {
                            core::LLVMConstInt(llvm_ty, *constant as libc::c_ulonglong, 0)
                        };
                        self.value_map.insert(value, llvm_value);
                    }
                }
            } else {
                panic!("Unknown value!")
            }
        }

        Ok(*self.value_map.get(&value).unwrap())
    }

    fn add_block_body(
        &mut self,
        library: &Library,
        function: Function,
        block: Block,
        llvm_block: LLVMBasicBlockRef,
        builder: LLVMBuilderRef,
    ) -> Result<(), Error> {
        for instruction in block.get_insts(library) {
            let instruction_name_cstr =
                CString::new(format!("{}", instruction.get_displayer(library)).to_string())
                    .unwrap();
            let instruction_name = instruction_name_cstr.as_ptr() as *const libc::c_char;

            if let Some(location) = instruction.get_location(library) {
                let llvm_location = self.make_location(library, location);

                unsafe { core::LLVMSetCurrentDebugLocation2(builder, llvm_location) };
            }

            let llvm_value = match instruction.get_inst(library) {
                Instruction::Binary(ty, op, x, y, _) => {
                    let llvm_x = self.get_or_insert_value(library, *x)?;
                    let llvm_y = self.get_or_insert_value(library, *y)?;

                    match op {
                        Binary::Add => unsafe {
                            core::LLVMBuildAdd(builder, llvm_x, llvm_y, instruction_name)
                        },
                        Binary::And => unsafe {
                            core::LLVMBuildAnd(builder, llvm_x, llvm_y, instruction_name)
                        },
                        Binary::Div => {
                            if ty.is_int_or_int_vector(library) {
                                unsafe {
                                    core::LLVMBuildSDiv(builder, llvm_x, llvm_y, instruction_name)
                                }
                            } else {
                                unsafe {
                                    core::LLVMBuildUDiv(builder, llvm_x, llvm_y, instruction_name)
                                }
                            }
                        }
                        Binary::Mul => unsafe {
                            core::LLVMBuildMul(builder, llvm_x, llvm_y, instruction_name)
                        },
                        Binary::Or => unsafe {
                            core::LLVMBuildOr(builder, llvm_x, llvm_y, instruction_name)
                        },
                        Binary::Rem => {
                            if ty.is_int_or_int_vector(library) {
                                unsafe {
                                    core::LLVMBuildSRem(builder, llvm_x, llvm_y, instruction_name)
                                }
                            } else {
                                unsafe {
                                    core::LLVMBuildURem(builder, llvm_x, llvm_y, instruction_name)
                                }
                            }
                        }
                        Binary::Shl => unsafe {
                            core::LLVMBuildShl(builder, llvm_x, llvm_y, instruction_name)
                        },
                        Binary::Shr => {
                            if ty.is_int_or_int_vector(library) {
                                unsafe {
                                    core::LLVMBuildAShr(builder, llvm_x, llvm_y, instruction_name)
                                }
                            } else {
                                unsafe {
                                    core::LLVMBuildLShr(builder, llvm_x, llvm_y, instruction_name)
                                }
                            }
                        }
                        Binary::Sub => unsafe {
                            core::LLVMBuildSub(builder, llvm_x, llvm_y, instruction_name)
                        },
                        Binary::Xor => unsafe {
                            core::LLVMBuildXor(builder, llvm_x, llvm_y, instruction_name)
                        },
                    }
                }
                Instruction::BitCast(ty, x, _) => {
                    let llvm_ty = self.get_or_insert_type(library, *ty)?;
                    let llvm_x = self.get_or_insert_value(library, *x)?;

                    unsafe { core::LLVMBuildBitCast(builder, llvm_x, llvm_ty, instruction_name) }
                }
                Instruction::Branch(block, arguments, _) => {
                    todo!()
                }
                Instruction::Call(function, arguments, _) => {
                    let llvm_function = *self.function_map.get(function).unwrap();

                    let mut llvm_values = Vec::new();

                    for argument in arguments {
                        llvm_values.push(self.get_or_insert_value(library, *argument)?);
                    }

                    unsafe {
                        core::LLVMBuildCall(
                            builder,
                            llvm_function,
                            llvm_values.as_mut_ptr(),
                            llvm_values.len() as libc::c_uint,
                            instruction_name,
                        )
                    }
                }
                Instruction::Cast(ty, x, _) => {
                    todo!()
                }
                Instruction::Cmp(ty, op, x, y, _) => {
                    let llvm_x = self.get_or_insert_value(library, *x)?;
                    let llvm_y = self.get_or_insert_value(library, *y)?;

                    if ty.is_float_or_float_vector(library) {
                        let predicate = match op {
                            Cmp::Eq => LLVMRealPredicate::LLVMRealOEQ,
                            Cmp::Ge => LLVMRealPredicate::LLVMRealOGE,
                            Cmp::Gt => LLVMRealPredicate::LLVMRealOGT,
                            Cmp::Le => LLVMRealPredicate::LLVMRealOLE,
                            Cmp::Lt => LLVMRealPredicate::LLVMRealOLT,
                            Cmp::Ne => LLVMRealPredicate::LLVMRealUNE,
                        };

                        unsafe {
                            core::LLVMBuildFCmp(
                                builder,
                                predicate,
                                llvm_x,
                                llvm_y,
                                instruction_name,
                            )
                        }
                    } else {
                        let predicate = if ty.is_int_or_int_vector(library) {
                            match op {
                                Cmp::Eq => LLVMIntPredicate::LLVMIntEQ,
                                Cmp::Ge => LLVMIntPredicate::LLVMIntSGE,
                                Cmp::Gt => LLVMIntPredicate::LLVMIntSGT,
                                Cmp::Le => LLVMIntPredicate::LLVMIntSLE,
                                Cmp::Lt => LLVMIntPredicate::LLVMIntSLT,
                                Cmp::Ne => LLVMIntPredicate::LLVMIntNE,
                            }
                        } else {
                            match op {
                                Cmp::Eq => LLVMIntPredicate::LLVMIntEQ,
                                Cmp::Ge => LLVMIntPredicate::LLVMIntUGE,
                                Cmp::Gt => LLVMIntPredicate::LLVMIntUGT,
                                Cmp::Le => LLVMIntPredicate::LLVMIntULE,
                                Cmp::Lt => LLVMIntPredicate::LLVMIntULT,
                                Cmp::Ne => LLVMIntPredicate::LLVMIntNE,
                            }
                        };

                        unsafe {
                            core::LLVMBuildICmp(
                                builder,
                                predicate,
                                llvm_x,
                                llvm_y,
                                instruction_name,
                            )
                        }
                    }
                }
                _ => todo!(),
            };

            unsafe { core::LLVMSetInstDebugLocation(builder, llvm_value) };

            self.value_map.insert(instruction, llvm_value);
        }

        Ok(())
    }

    fn make_module(&mut self, library: &Library) -> Result<LLVMModule, Error> {
        for module in library.get_modules() {
            let name_cstr = CString::new(module.get_name(library).get_name(library)).unwrap();
            let name = name_cstr.as_ptr() as *const libc::c_char;

            self.current_module =
                unsafe { core::LLVMModuleCreateWithNameInContext(name, self.context) };

            unsafe { core::LLVMSetTarget(self.current_module, self.triple) };

            self.dibuilder = unsafe { debuginfo::LLVMCreateDIBuilder(self.current_module) };

            for global in module.get_globals(library) {
                self.get_or_insert_type(library, global.get_type(library));
            }

            for function in module.get_functions(library) {
                let llvm_function = self.make_function_declaration(library, function)?;
                self.function_map.insert(function, llvm_function);
            }

            for function in module.get_functions(library) {
                self.add_function_body(
                    library,
                    function,
                    *self.function_map.get(&function).unwrap(),
                );
            }
        }

        todo!();
    }
}

impl CodeGen for Llvm {
    type Error = Error;

    fn generate<W: Seek + Write>(
        library: &Library,
        platform: CodeGenPlatform,
        _writer: &mut W,
    ) -> Result<(), Self::Error> {
        let mut codegen = Self::new(platform);

        let _llvm_module = codegen.make_module(library)?;

        todo!();
    }
}
