use crate::*;
use libc::*;
use llvm_sys::core::*;
use llvm_sys::prelude::*;
use llvm_sys::*;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::io::{Seek, Write};
use std::path::Path;
use std::ptr;

const EMPTY_NAME: *const libc::c_char = b"\0".as_ptr() as *const libc::c_char;

const DW_ATE_VOID: debuginfo::LLVMDWARFTypeEncoding = 0x00;
const DW_ATE_BOOLEAN: debuginfo::LLVMDWARFTypeEncoding = 0x02;
const DW_ATE_FLOAT: debuginfo::LLVMDWARFTypeEncoding = 0x04;
const DW_ATE_SIGNED: debuginfo::LLVMDWARFTypeEncoding = 0x05;
const DW_ATE_UNSIGNED: debuginfo::LLVMDWARFTypeEncoding = 0x07;

pub struct Llvm {
    context: LLVMContextRef,
    module: LLVMModuleRef,
    dibuilder: LLVMDIBuilderRef,
    ditypes: HashMap<Type, LLVMMetadataRef>,
    types: HashMap<Type, LLVMTypeRef>,
    filenames: HashMap<Name, LLVMMetadataRef>,
    function_map: HashMap<Function, LLVMValueRef>,
    block_map: HashMap<Block, LLVMBasicBlockRef>,
    value_map: HashMap<Value, LLVMValueRef>,
    target_data: target::LLVMTargetDataRef,
    target_machine: target_machine::LLVMTargetMachineRef,
}

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Llvm(CString),
}

impl From<std::io::Error> for Error {
    fn from(io_error: std::io::Error) -> Self {
        Error::Io(io_error)
    }
}

impl Drop for Llvm {
    fn drop(&mut self) {
        unsafe { target::LLVMDisposeTargetData(self.target_data) };
        unsafe { target_machine::LLVMDisposeTargetMachine(self.target_machine) };
        unsafe { debuginfo::LLVMDisposeDIBuilder(self.dibuilder) };
        unsafe { core::LLVMDisposeModule(self.module) };
        unsafe { core::LLVMContextDispose(self.context) };
    }
}

impl Llvm {
    fn new(platform: CodeGenPlatform) -> Result<Self, Error> {
        unsafe {
            target::LLVMInitializeAArch64Target();
            target::LLVMInitializeAArch64TargetMC();
            target::LLVMInitializeAArch64AsmParser();
            target::LLVMInitializeAArch64AsmPrinter();
            target::LLVMInitializeAArch64TargetInfo();
            target::LLVMInitializeAArch64Disassembler();
            target::LLVMInitializeX86Target();
            target::LLVMInitializeX86TargetMC();
            target::LLVMInitializeX86AsmParser();
            target::LLVMInitializeX86AsmPrinter();
            target::LLVMInitializeX86TargetInfo();
            target::LLVMInitializeX86Disassembler();
        }

        let context = unsafe { core::LLVMContextCreate() };

        let triple = match platform {
            CodeGenPlatform::Windows64Bit => b"x86_64-pc-win32-msvc\0",
            CodeGenPlatform::MacOsAppleSilicon => b"aarch64-apple-darwin\0",
        }
        .as_ptr() as *const libc::c_char;

        let mut target = ptr::null_mut();
        let mut error_message = ptr::null_mut();

        if unsafe {
            target_machine::LLVMGetTargetFromTriple(triple, &mut target, &mut error_message)
        } != 0
        {
            let cstr = unsafe { CStr::from_ptr(error_message) }.to_owned();
            unsafe { LLVMDisposeMessage(error_message) };
            return Err(Error::Llvm(cstr));
        }

        unsafe { LLVMDisposeMessage(error_message) };

        let target_machine = unsafe {
            target_machine::LLVMCreateTargetMachine(
                target,
                triple,
                ptr::null_mut(),
                ptr::null_mut(),
                target_machine::LLVMCodeGenOptLevel::LLVMCodeGenLevelDefault,
                target_machine::LLVMRelocMode::LLVMRelocPIC,
                target_machine::LLVMCodeModel::LLVMCodeModelLarge,
            )
        };

        let target_data = unsafe { target_machine::LLVMCreateTargetDataLayout(target_machine) };

        let module = unsafe { core::LLVMModuleCreateWithNameInContext(EMPTY_NAME, context) };
        unsafe { core::LLVMSetTarget(module, triple) };

        let dibuilder = unsafe { debuginfo::LLVMCreateDIBuilder(module) };

        Ok(Self {
            context,
            module,
            dibuilder,
            ditypes: HashMap::new(),
            types: HashMap::new(),
            filenames: HashMap::new(),
            function_map: HashMap::new(),
            block_map: HashMap::new(),
            value_map: HashMap::new(),
            target_data,
            target_machine,
        })
    }

    fn get_or_insert_filename(&mut self, library: &Library, name: Name) -> LLVMMetadataRef {
        if let Some(filename) = self.filenames.get(&name) {
            *filename
        } else {
            let name_str = name.as_str(library);

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
        #[allow(clippy::map_entry)]
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

            let name = ty.get_name(library).as_str(library);
            let size = unsafe {
                target::LLVMABISizeOfType(self.target_data, self.get_or_insert_type(library, ty)?)
            };
            let mut elements = Vec::new();
            let runtimelang = 0; // C/C++ will have to do for now

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
                target::LLVMABISizeOfType(self.target_data, self.get_or_insert_type(library, ty)?)
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
            let name_cstr = CString::new(ty.get_name(library).as_str(library)).unwrap();
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
        llvm_module: LLVMModuleRef,
        module_name: &str,
    ) -> Result<LLVMValueRef, Error> {
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

        let name_string = if module_name.is_empty() {
            // We have an empty module name, so this is the global namespace.
            function.get_name(library).as_str(library).to_string()
        } else {
            module_name.to_owned() + "::" + function.get_name(library).as_str(library)
        };

        let name_cstr = CString::new(name_string).unwrap();
        let name = name_cstr.as_ptr() as *const libc::c_char;

        let llvm_function = unsafe { core::LLVMAddFunction(llvm_module, name, function_type) };

        for (index, arg) in function.get_args(library).enumerate() {
            let arg_name = arg.get_name(library).as_str(library);

            let llvm_arg = unsafe { core::LLVMGetParam(llvm_function, index as libc::c_uint) };

            unsafe {
                core::LLVMSetValueName2(
                    llvm_arg,
                    arg_name.as_ptr() as *const libc::c_char,
                    arg_name.len() as libc::size_t,
                )
            };
            self.value_map.insert(arg, llvm_arg);
        }

        Ok(llvm_function)
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
                core::LLVMAppendBasicBlockInContext(self.context, llvm_function, EMPTY_NAME)
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

                    let llvm_phi = unsafe { core::LLVMBuildPhi(builder, llvm_ty, EMPTY_NAME) };

                    self.value_map.insert(argument, llvm_phi);
                }
            }
        }

        for block in function.get_blocks(library) {
            let llvm_block = *self.block_map.get(&block).unwrap();

            self.add_block_body(library, block, llvm_block, builder)?;
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
        block: Block,
        llvm_block: LLVMBasicBlockRef,
        builder: LLVMBuilderRef,
    ) -> Result<(), Error> {
        unsafe { core::LLVMPositionBuilderAtEnd(builder, llvm_block) };

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
                        Binary::Add => {
                            if ty.is_float_or_float_vector(library) {
                                unsafe {
                                    core::LLVMBuildFAdd(builder, llvm_x, llvm_y, instruction_name)
                                }
                            } else {
                                unsafe {
                                    core::LLVMBuildAdd(builder, llvm_x, llvm_y, instruction_name)
                                }
                            }
                        }
                        Binary::And => unsafe {
                            core::LLVMBuildAnd(builder, llvm_x, llvm_y, instruction_name)
                        },
                        Binary::Div => {
                            if ty.is_float_or_float_vector(library) {
                                unsafe {
                                    core::LLVMBuildFDiv(builder, llvm_x, llvm_y, instruction_name)
                                }
                            } else if ty.is_int_or_int_vector(library) {
                                unsafe {
                                    core::LLVMBuildSDiv(builder, llvm_x, llvm_y, instruction_name)
                                }
                            } else {
                                unsafe {
                                    core::LLVMBuildUDiv(builder, llvm_x, llvm_y, instruction_name)
                                }
                            }
                        }
                        Binary::Mul => {
                            if ty.is_float_or_float_vector(library) {
                                unsafe {
                                    core::LLVMBuildFMul(builder, llvm_x, llvm_y, instruction_name)
                                }
                            } else {
                                unsafe {
                                    core::LLVMBuildMul(builder, llvm_x, llvm_y, instruction_name)
                                }
                            }
                        }
                        Binary::Or => unsafe {
                            core::LLVMBuildOr(builder, llvm_x, llvm_y, instruction_name)
                        },
                        Binary::Rem => {
                            if ty.is_float_or_float_vector(library) {
                                unsafe {
                                    core::LLVMBuildFRem(builder, llvm_x, llvm_y, instruction_name)
                                }
                            } else if ty.is_int_or_int_vector(library) {
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
                        Binary::Sub => {
                            if ty.is_float_or_float_vector(library) {
                                unsafe {
                                    core::LLVMBuildFSub(builder, llvm_x, llvm_y, instruction_name)
                                }
                            } else {
                                unsafe {
                                    core::LLVMBuildSub(builder, llvm_x, llvm_y, instruction_name)
                                }
                            }
                        }
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
                    for (argument, block_argument) in arguments.iter().zip(block.get_args(library))
                    {
                        let llvm_val = self.get_or_insert_value(library, *argument)?;
                        let llvm_phi = self.get_or_insert_value(library, block_argument)?;

                        let mut llvm_values = vec![llvm_val];
                        let mut llvm_blocks = vec![llvm_block];

                        unsafe {
                            core::LLVMAddIncoming(
                                llvm_phi,
                                llvm_values.as_mut_ptr(),
                                llvm_blocks.as_mut_ptr(),
                                1,
                            )
                        };
                    }

                    unsafe { core::LLVMBuildBr(builder, *self.block_map.get(block).unwrap()) }
                }
                Instruction::Call(function, arguments, _) => {
                    let llvm_function = *self.function_map.get(function).unwrap();

                    let mut llvm_values = Vec::new();

                    for argument in arguments {
                        llvm_values.push(self.get_or_insert_value(library, *argument)?);
                    }

                    let used_instruction_name =
                        if function.get_return_type(library).is_void(library) {
                            EMPTY_NAME
                        } else {
                            instruction_name
                        };

                    unsafe {
                        core::LLVMBuildCall(
                            builder,
                            llvm_function,
                            llvm_values.as_mut_ptr(),
                            llvm_values.len() as libc::c_uint,
                            used_instruction_name,
                        )
                    }
                }
                Instruction::Cast(ty, x, _) => {
                    let llvm_x = self.get_or_insert_value(library, *x)?;
                    let llvm_ty = self.get_or_insert_type(library, *ty)?;

                    let src_ty = x.get_type(library);

                    let dst_is_float = ty.is_float_or_float_vector(library);
                    let dst_is_int = ty.is_int_or_int_vector(library);
                    let dst_is_uint = ty.is_uint_or_uint_vector(library);

                    let src_is_float = src_ty.is_float_or_float_vector(library);
                    let src_is_int = src_ty.is_int_or_int_vector(library);
                    let src_is_uint = src_ty.is_uint_or_uint_vector(library);

                    // If both inputs are integers that are the same size, the cast is a no-op.
                    if (dst_is_int || dst_is_uint)
                        && (src_is_int || src_is_uint)
                        && src_ty.get_bits(library) == ty.get_bits(library)
                    {
                        let name = "llvm.ssa.copy";
                        let llvm_intrinsic_id = unsafe {
                            core::LLVMLookupIntrinsicID(
                                name.as_ptr() as *const libc::c_char,
                                name.len(),
                            )
                        };

                        let mut llvm_types = vec![llvm_ty];

                        let llvm_intrinsic = unsafe {
                            core::LLVMGetIntrinsicDeclaration(
                                self.module,
                                llvm_intrinsic_id,
                                llvm_types.as_mut_ptr(),
                                llvm_types.len() as libc::size_t,
                            )
                        };

                        let mut llvm_values = vec![llvm_x];

                        unsafe {
                            core::LLVMBuildCall(
                                builder,
                                llvm_intrinsic,
                                llvm_values.as_mut_ptr(),
                                llvm_values.len() as libc::c_uint,
                                instruction_name,
                            )
                        }
                    } else if dst_is_float {
                        if src_is_float {
                            unsafe {
                                core::LLVMBuildFPCast(builder, llvm_x, llvm_ty, instruction_name)
                            }
                        } else if src_is_int {
                            unsafe {
                                core::LLVMBuildSIToFP(builder, llvm_x, llvm_ty, instruction_name)
                            }
                        } else {
                            debug_assert!(src_is_uint);
                            unsafe {
                                core::LLVMBuildUIToFP(builder, llvm_x, llvm_ty, instruction_name)
                            }
                        }
                    } else if dst_is_int {
                        if src_is_float {
                            unsafe {
                                core::LLVMBuildFPToSI(builder, llvm_x, llvm_ty, instruction_name)
                            }
                        } else if src_is_int {
                            unsafe {
                                core::LLVMBuildIntCast2(
                                    builder,
                                    llvm_x,
                                    llvm_ty,
                                    1,
                                    instruction_name,
                                )
                            }
                        } else {
                            debug_assert!(src_is_uint);
                            unsafe {
                                core::LLVMBuildIntCast2(
                                    builder,
                                    llvm_x,
                                    llvm_ty,
                                    0,
                                    instruction_name,
                                )
                            }
                        }
                    } else {
                        debug_assert!(dst_is_uint);
                        if src_is_float {
                            unsafe {
                                core::LLVMBuildFPToUI(builder, llvm_x, llvm_ty, instruction_name)
                            }
                        } else if src_is_int {
                            unsafe {
                                core::LLVMBuildIntCast2(
                                    builder,
                                    llvm_x,
                                    llvm_ty,
                                    1,
                                    instruction_name,
                                )
                            }
                        } else {
                            debug_assert!(src_is_uint);
                            unsafe {
                                core::LLVMBuildIntCast2(
                                    builder,
                                    llvm_x,
                                    llvm_ty,
                                    0,
                                    instruction_name,
                                )
                            }
                        }
                    }
                }
                Instruction::Cmp(_, op, x, y, _) => {
                    let llvm_x = self.get_or_insert_value(library, *x)?;
                    let llvm_y = self.get_or_insert_value(library, *y)?;

                    let x_ty = x.get_type(library);

                    if x_ty.is_float_or_float_vector(library) {
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
                        let predicate = if x_ty.is_int_or_int_vector(library) {
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
                Instruction::ConditionalBranch(
                    condition,
                    true_block,
                    false_block,
                    true_arguments,
                    false_arguments,
                    _,
                ) => {
                    let llvm_condition = self.get_or_insert_value(library, *condition)?;

                    for (argument, block_argument) in
                        true_arguments.iter().zip(true_block.get_args(library))
                    {
                        let llvm_val = self.get_or_insert_value(library, *argument)?;
                        let llvm_phi = self.get_or_insert_value(library, block_argument)?;

                        let mut llvm_values = vec![llvm_val];
                        let mut llvm_blocks = vec![llvm_block];

                        unsafe {
                            core::LLVMAddIncoming(
                                llvm_phi,
                                llvm_values.as_mut_ptr(),
                                llvm_blocks.as_mut_ptr(),
                                1,
                            )
                        };
                    }

                    for (argument, block_argument) in
                        false_arguments.iter().zip(false_block.get_args(library))
                    {
                        let llvm_val = self.get_or_insert_value(library, *argument)?;
                        let llvm_phi = self.get_or_insert_value(library, block_argument)?;

                        let mut llvm_values = vec![llvm_val];
                        let mut llvm_blocks = vec![llvm_block];

                        unsafe {
                            core::LLVMAddIncoming(
                                llvm_phi,
                                llvm_values.as_mut_ptr(),
                                llvm_blocks.as_mut_ptr(),
                                1,
                            )
                        };
                    }

                    unsafe {
                        core::LLVMBuildCondBr(
                            builder,
                            llvm_condition,
                            *self.block_map.get(true_block).unwrap(),
                            *self.block_map.get(false_block).unwrap(),
                        )
                    }
                }
                Instruction::Extract(x, index, _) => {
                    let llvm_x = self.get_or_insert_value(library, *x)?;

                    if x.get_type(library).is_vector(library) {
                        let int_ty = unsafe { LLVMIntTypeInContext(self.context, 64) };
                        let llvm_index =
                            unsafe { core::LLVMConstInt(int_ty, *index as libc::c_ulonglong, 0) };

                        unsafe {
                            core::LLVMBuildExtractElement(
                                builder,
                                llvm_x,
                                llvm_index,
                                instruction_name,
                            )
                        }
                    } else {
                        unsafe {
                            core::LLVMBuildExtractValue(
                                builder,
                                llvm_x,
                                *index as libc::c_uint,
                                instruction_name,
                            )
                        }
                    }
                }
                Instruction::IndexInto(ty, ptr, indices, _) => {
                    let llvm_ty = self.get_or_insert_type(library, *ty)?;
                    let llvm_ptr = self.get_or_insert_value(library, *ptr)?;

                    let llvm_ptr_ty = unsafe { core::LLVMPointerType(llvm_ty, 0) };

                    let llvm_cast = unsafe {
                        core::LLVMBuildBitCast(builder, llvm_ptr, llvm_ptr_ty, instruction_name)
                    };

                    let mut llvm_indices = Vec::new();

                    for index in indices {
                        llvm_indices.push(self.get_or_insert_value(library, *index)?);
                    }

                    let llvm_gep = unsafe {
                        core::LLVMBuildInBoundsGEP2(
                            builder,
                            llvm_ty,
                            llvm_cast,
                            llvm_indices.as_mut_ptr(),
                            llvm_indices.len() as libc::c_uint,
                            instruction_name,
                        )
                    };

                    unsafe {
                        core::LLVMBuildBitCast(
                            builder,
                            llvm_gep,
                            core::LLVMTypeOf(llvm_ptr),
                            instruction_name,
                        )
                    }
                }
                Instruction::Insert(aggregate, x, index, _) => {
                    let llvm_aggregate = self.get_or_insert_value(library, *aggregate)?;
                    let llvm_x = self.get_or_insert_value(library, *x)?;

                    if aggregate.get_type(library).is_vector(library) {
                        let int_ty = unsafe { LLVMIntTypeInContext(self.context, 64) };
                        let llvm_index =
                            unsafe { core::LLVMConstInt(int_ty, *index as libc::c_ulonglong, 0) };

                        unsafe {
                            core::LLVMBuildInsertElement(
                                builder,
                                llvm_aggregate,
                                llvm_x,
                                llvm_index,
                                instruction_name,
                            )
                        }
                    } else {
                        unsafe {
                            core::LLVMBuildInsertValue(
                                builder,
                                llvm_aggregate,
                                llvm_x,
                                *index as libc::c_uint,
                                instruction_name,
                            )
                        }
                    }
                }
                Instruction::Load(ty, ptr, _) => {
                    let llvm_ty = self.get_or_insert_type(library, *ty)?;
                    let llvm_ptr = self.get_or_insert_value(library, *ptr)?;

                    let llvm_ptr_ty = unsafe { core::LLVMPointerType(llvm_ty, 0) };

                    let llvm_cast = unsafe {
                        core::LLVMBuildBitCast(builder, llvm_ptr, llvm_ptr_ty, instruction_name)
                    };

                    unsafe { core::LLVMBuildLoad2(builder, llvm_ty, llvm_cast, instruction_name) }
                }
                Instruction::Return(_) => unsafe { core::LLVMBuildRetVoid(builder) },
                Instruction::ReturnValue(_, x, _) => {
                    let llvm_x = self.get_or_insert_value(library, *x)?;

                    unsafe { core::LLVMBuildRet(builder, llvm_x) }
                }
                Instruction::Select(_, cond, x, y, _) => {
                    let llvm_cond = self.get_or_insert_value(library, *cond)?;
                    let llvm_x = self.get_or_insert_value(library, *x)?;
                    let llvm_y = self.get_or_insert_value(library, *y)?;

                    unsafe {
                        core::LLVMBuildSelect(builder, llvm_cond, llvm_x, llvm_y, instruction_name)
                    }
                }
                Instruction::StackAlloc(name, ty, _, _) => {
                    let name = name.as_str(library);
                    let len = name.len();
                    let name_cstr = CString::new(name).unwrap();
                    let name = name_cstr.as_ptr() as *const libc::c_char;

                    let llvm_ty = self.get_or_insert_type(library, *ty)?;

                    let alloca = unsafe { core::LLVMBuildAlloca(builder, llvm_ty, name) };

                    if let Some(location) = instruction.get_location(library) {
                        let line_number = location.get_line();

                        let llvm_location = unsafe { core::LLVMGetCurrentDebugLocation2(builder) };

                        debug_assert!(!llvm_location.is_null());

                        let llvm_scope = llvm_location;

                        let llvm_variable = unsafe {
                            debuginfo::LLVMDIBuilderCreateAutoVariable(
                                self.dibuilder,
                                llvm_scope,
                                name,
                                len as libc::size_t,
                                llvm_location,
                                line_number as libc::c_uint,
                                self.get_or_insert_debug_type(library, *ty)?,
                                0,
                                0,
                                0,
                            )
                        };

                        unsafe {
                            debuginfo::LLVMDIBuilderInsertDbgValueAtEnd(
                                self.dibuilder,
                                alloca,
                                llvm_variable,
                                debuginfo::LLVMDIBuilderCreateExpression(
                                    self.dibuilder,
                                    ptr::null_mut(),
                                    0,
                                ),
                                LLVMGetCurrentDebugLocation2(builder),
                                llvm_block,
                            )
                        };

                        todo!("Need a debug scope, which means subprograms and all that jazz");
                    }

                    alloca
                }
                Instruction::Store(ty, ptr, val, _) => {
                    let llvm_ty = self.get_or_insert_type(library, *ty)?;
                    let llvm_ptr = self.get_or_insert_value(library, *ptr)?;
                    let llvm_val = self.get_or_insert_value(library, *val)?;

                    let llvm_ptr_ty = unsafe { core::LLVMPointerType(llvm_ty, 0) };

                    let llvm_cast = unsafe {
                        core::LLVMBuildBitCast(builder, llvm_ptr, llvm_ptr_ty, instruction_name)
                    };

                    unsafe { core::LLVMBuildStore(builder, llvm_val, llvm_cast) }
                }
                Instruction::Unary(_, op, x, _) => {
                    let llvm_x = self.get_or_insert_value(library, *x)?;

                    match op {
                        Unary::Neg => {
                            let x_ty = x.get_type(library);
                            if x_ty.is_float_or_float_vector(library) {
                                unsafe { core::LLVMBuildFNeg(builder, llvm_x, instruction_name) }
                            } else {
                                unsafe { core::LLVMBuildNeg(builder, llvm_x, instruction_name) }
                            }
                        }
                        Unary::Not => unsafe {
                            core::LLVMBuildNot(builder, llvm_x, instruction_name)
                        },
                    }
                }
            };

            unsafe { core::LLVMSetInstDebugLocation(builder, llvm_value) };

            self.value_map.insert(instruction, llvm_value);
        }

        Ok(())
    }

    fn make_module(&mut self, library: &Library) -> Result<(), Error> {
        for module in library.get_modules() {
            let module_name = module.get_name(library).as_str(library).to_owned();

            for global in module.get_globals(library) {
                let name_cstr = CString::new(global.get_name(library).as_str(library)).unwrap();
                let name = name_cstr.as_ptr() as *const libc::c_char;

                let llvm_ty =
                    self.get_or_insert_type(library, global.get_global_backing_type(library))?;

                let llvm_global = unsafe { core::LLVMAddGlobal(self.module, llvm_ty, name) };

                let llvm_ptr_ty = self.get_or_insert_type(library, global.get_type(library))?;

                let llvm_bitcast = unsafe { core::LLVMConstBitCast(llvm_global, llvm_ptr_ty) };

                self.value_map.insert(global, llvm_bitcast);
            }

            for function in module.get_functions(library) {
                let llvm_function =
                    self.make_function_declaration(library, function, self.module, &module_name)?;
                self.function_map.insert(function, llvm_function);
            }

            for function in module.get_functions(library) {
                self.add_function_body(
                    library,
                    function,
                    *self.function_map.get(&function).unwrap(),
                )?;
            }
        }

        Ok(())
    }
}

impl CodeGen for Llvm {
    type Error = Error;

    fn generate<W: Seek + Write>(
        library: &Library,
        platform: CodeGenPlatform,
        output: CodeGenOutput,
        writer: &mut W,
    ) -> Result<(), Self::Error> {
        let mut codegen = Self::new(platform)?;

        codegen.make_module(library)?;

        match output {
            CodeGenOutput::Assembly => {
                let mut error_message = ptr::null_mut();
                let mut memory_buffer = ptr::null_mut();

                if unsafe {
                    target_machine::LLVMTargetMachineEmitToMemoryBuffer(
                        codegen.target_machine,
                        codegen.module,
                        target_machine::LLVMCodeGenFileType::LLVMAssemblyFile,
                        &mut error_message,
                        &mut memory_buffer,
                    )
                } != 0
                {
                    let cstr = unsafe { CStr::from_ptr(error_message) }.to_owned();
                    unsafe { LLVMDisposeMessage(error_message) };
                    return Err(Error::Llvm(cstr));
                }

                let memory_buffer_start =
                    unsafe { core::LLVMGetBufferStart(memory_buffer) } as *const u8;
                let memory_buffer_size = unsafe { core::LLVMGetBufferSize(memory_buffer) };

                writer.write_all(unsafe {
                    std::slice::from_raw_parts(memory_buffer_start, memory_buffer_size)
                })?;

                unsafe { LLVMDisposeMemoryBuffer(memory_buffer) };
                unsafe { LLVMDisposeMessage(error_message) };
            }
            CodeGenOutput::Intermediate => {
                let module_string = unsafe { core::LLVMPrintModuleToString(codegen.module) };

                let cstr = unsafe { CStr::from_ptr(module_string) };

                write!(writer, "{}", cstr.to_string_lossy())?;

                unsafe { LLVMDisposeMessage(module_string) };
            }
            CodeGenOutput::Object => {
                let mut error_message = ptr::null_mut();
                let mut memory_buffer = ptr::null_mut();

                if unsafe {
                    target_machine::LLVMTargetMachineEmitToMemoryBuffer(
                        codegen.target_machine,
                        codegen.module,
                        target_machine::LLVMCodeGenFileType::LLVMObjectFile,
                        &mut error_message,
                        &mut memory_buffer,
                    )
                } != 0
                {
                    let cstr = unsafe { CStr::from_ptr(error_message) }.to_owned();
                    unsafe { LLVMDisposeMessage(error_message) };
                    return Err(Error::Llvm(cstr));
                }

                let memory_buffer_start =
                    unsafe { core::LLVMGetBufferStart(memory_buffer) } as *const u8;
                let memory_buffer_size = unsafe { core::LLVMGetBufferSize(memory_buffer) };

                writer.write_all(unsafe {
                    std::slice::from_raw_parts(memory_buffer_start, memory_buffer_size)
                })?;

                unsafe { LLVMDisposeMemoryBuffer(memory_buffer) };
                unsafe { LLVMDisposeMessage(error_message) };
            }
        }

        Ok(())
    }
}
