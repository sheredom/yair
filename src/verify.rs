use crate::*;
use std::collections::HashSet;

pub enum VerifyError<'a> {
    FunctionAndFirstBlockMustHaveMatchingArguments(&'a Library, Function, Block),
    BranchAndBlockMustHaveMatchingArguments(&'a Library, Value, Block),
    ReturnDoesNotMatchType(&'a Library, Value, Type),
    ReturnValueDoesNotMatchType(&'a Library, Value, Type, Type),
    ConditionMustBeBool(&'a Library, Value, Value, Type),
    ValueTypesMustMatch(&'a Library, Value, Value, Value),
    TypeMustBeIntVector(&'a Library, Value, Value),
    TypeMustBeBoolOrIntVector(&'a Library, Value, Value),
    TypeMustBeFloatOrIntVector(&'a Library, Value, Value),
    TypeMustBeBoolOrFloatOrIntVector(&'a Library, Value, Value),
    VectorWidthsMustMatch(&'a Library, Value, Value, Type),
    ValueUsedWasNotLive(&'a Library, Value, Value),
    CallAndFunctionMustHaveMatchingArguments(&'a Library, Function, Value),
}

impl<'a> std::fmt::Display for VerifyError<'a> {
    fn fmt(
        &self,
        formatter: &mut std::fmt::Formatter<'_>,
    ) -> std::result::Result<(), std::fmt::Error> {
        match self {
            VerifyError::FunctionAndFirstBlockMustHaveMatchingArguments(l, f, b) => {
                writeln!(formatter, "A function and the first block contained within it must have matching arguments:")?;
                writeln!(formatter, "  {}", f.get_displayer(l))?;
                writeln!(formatter, "And:")?;
                writeln!(formatter, "  {}", b.get_displayer(l))
            }
            VerifyError::BranchAndBlockMustHaveMatchingArguments(l, v, b) => {
                writeln!(formatter, "A branch and its target block contained within it must have matching arguments:")?;
                writeln!(formatter, "  {}", v.get_inst_displayer(l))?;
                writeln!(formatter, "And:")?;
                writeln!(formatter, "  {}", b.get_displayer(l))
            }
            VerifyError::ReturnDoesNotMatchType(l, v, t) => {
                writeln!(
                    formatter,
                    "A return and a function must have matching types:"
                )?;
                writeln!(formatter, "  {}", v.get_inst_displayer(l))?;
                writeln!(formatter, "And:")?;
                writeln!(formatter, "  {}", t.get_displayer(l))
            }
            VerifyError::ReturnValueDoesNotMatchType(l, v, rt, ft) => {
                writeln!(
                    formatter,
                    "A return and a function must have matching types:"
                )?;
                writeln!(formatter, "  {}", v.get_inst_displayer(l))?;
                writeln!(formatter, "Which has type:")?;
                writeln!(formatter, "  {}", rt.get_displayer(l))?;
                writeln!(formatter, "And:")?;
                writeln!(formatter, "  {}", ft.get_displayer(l))
            }
            VerifyError::ConditionMustBeBool(l, v, c, t) => {
                writeln!(formatter, "Condition must be of type bool:")?;
                writeln!(formatter, "  {}", v.get_inst_displayer(l))?;
                writeln!(formatter, "Which has condition:")?;
                writeln!(formatter, "  {}", c.get_displayer(l))?;
                writeln!(formatter, "Which has type:")?;
                writeln!(formatter, "  {}", t.get_displayer(l))
            }
            VerifyError::ValueTypesMustMatch(l, i, a, b) => {
                writeln!(formatter, "Types must match:")?;
                writeln!(formatter, "  {}", i.get_inst_displayer(l))?;
                writeln!(formatter, "With:")?;
                writeln!(formatter, "  {}", a.get_displayer(l))?;
                writeln!(formatter, "Which has type:")?;
                writeln!(formatter, "  {}", a.get_type(l).get_displayer(l))?;
                writeln!(formatter, "And:")?;
                writeln!(formatter, "  {}", b.get_displayer(l))?;
                writeln!(formatter, "Which has type:")?;
                writeln!(formatter, "  {}", b.get_type(l).get_displayer(l))
            }
            VerifyError::TypeMustBeIntVector(l, i, v) => {
                writeln!(formatter, "Type must be integral, or vector of integral:")?;
                writeln!(formatter, "  {}", i.get_inst_displayer(l))?;
                writeln!(formatter, "With:")?;
                writeln!(formatter, "  {}", v.get_displayer(l))?;
                writeln!(formatter, "Which has type:")?;
                writeln!(formatter, "  {}", v.get_type(l).get_displayer(l))
            }
            VerifyError::TypeMustBeBoolOrIntVector(l, i, v) => {
                writeln!(
                    formatter,
                    "Type must be bool, integral, or vector of bool or integral:"
                )?;
                writeln!(formatter, "  {}", i.get_inst_displayer(l))?;
                writeln!(formatter, "With:")?;
                writeln!(formatter, "  {}", v.get_displayer(l))?;
                writeln!(formatter, "Which has type:")?;
                writeln!(formatter, "  {}", v.get_type(l).get_displayer(l))
            }
            VerifyError::TypeMustBeFloatOrIntVector(l, i, v) => {
                writeln!(
                    formatter,
                    "Type must be float, integral, or vector of float or integral:"
                )?;
                writeln!(formatter, "  {}", i.get_inst_displayer(l))?;
                writeln!(formatter, "With:")?;
                writeln!(formatter, "  {}", v.get_displayer(l))?;
                writeln!(formatter, "Which has type:")?;
                writeln!(formatter, "  {}", v.get_type(l).get_displayer(l))
            }
            VerifyError::TypeMustBeBoolOrFloatOrIntVector(l, i, v) => {
                writeln!(
                    formatter,
                    "Type must be bool, float, integral, or vector of bool, float, or integral:"
                )?;
                writeln!(formatter, "  {}", i.get_inst_displayer(l))?;
                writeln!(formatter, "With:")?;
                writeln!(formatter, "  {}", v.get_displayer(l))?;
                writeln!(formatter, "Which has type:")?;
                writeln!(formatter, "  {}", v.get_type(l).get_displayer(l))
            }
            VerifyError::VectorWidthsMustMatch(l, i, v, t) => {
                writeln!(formatter, "Vector widths must match:")?;
                writeln!(formatter, "  {}", i.get_inst_displayer(l))?;
                writeln!(formatter, "With:")?;
                writeln!(formatter, "  {}", v.get_displayer(l))?;
                writeln!(formatter, "Which has type:")?;
                writeln!(formatter, "  {}", v.get_type(l).get_displayer(l))?;
                writeln!(formatter, "And:")?;
                writeln!(formatter, "  {}", t.get_displayer(l))
            }
            VerifyError::ValueUsedWasNotLive(l, i, v) => {
                writeln!(formatter, "Value used in block was not live:")?;
                writeln!(formatter, "  {}", v.get_displayer(l))?;
                writeln!(formatter, "Used by:")?;
                writeln!(formatter, "  {}", i.get_inst_displayer(l))
            }
            VerifyError::CallAndFunctionMustHaveMatchingArguments(l, f, i) => {
                writeln!(
                    formatter,
                    "A call and its called function must have matching arguments:"
                )?;
                writeln!(formatter, "  {}", i.get_inst_displayer(l))?;
                writeln!(formatter, "And:")?;
                writeln!(formatter, "  {}", f.get_displayer(l))
            }
        }
    }
}

struct Verifier<'a> {
    library: &'a Library,
    live_values: HashSet<Value>,
}

impl<'a> Verifier<'a> {
    fn new(library: &'a Library) -> Self {
        Self {
            library,
            live_values: HashSet::new(),
        }
    }

    fn verify_branch_and_block(
        &mut self,
        branch: Value,
        branch_args: &[Value],
        block: Block,
    ) -> Result<(), VerifyError<'a>> {
        if branch_args.len() != block.get_num_args(self.library) {
            return Err(VerifyError::BranchAndBlockMustHaveMatchingArguments(
                self.library,
                branch,
                block,
            ));
        }

        for (barg, iarg) in block.get_args(self.library).zip(branch_args) {
            if barg.get_type(self.library) != iarg.get_type(self.library) {
                return Err(VerifyError::BranchAndBlockMustHaveMatchingArguments(
                    self.library,
                    branch,
                    block,
                ));
            }
        }

        for barg in branch_args {
            self.verify_live(branch, *barg)?;
        }

        Ok(())
    }

    fn verify_live(&mut self, inst: Value, value: Value) -> Result<(), VerifyError<'a>> {
        if !value.is_constant(self.library) && !self.live_values.contains(&value) {
            return Err(VerifyError::ValueUsedWasNotLive(self.library, inst, value));
        }

        Ok(())
    }

    fn verify_function(&mut self, function: Function) -> Result<(), VerifyError<'a>> {
        for (index, block) in function.get_blocks(self.library).enumerate() {
            // The first block has to have the same arguments as the function.
            if index == 0 {
                if function.get_num_args(self.library) != block.get_num_args(self.library) {
                    return Err(VerifyError::FunctionAndFirstBlockMustHaveMatchingArguments(
                        self.library,
                        function,
                        block,
                    ));
                }

                for (farg, barg) in function
                    .get_args(self.library)
                    .zip(block.get_args(self.library))
                {
                    if farg.get_type(self.library) != barg.get_type(self.library) {
                        return Err(VerifyError::FunctionAndFirstBlockMustHaveMatchingArguments(
                            self.library,
                            function,
                            block,
                        ));
                    }
                }
            }

            // Insert all the arguments as being live.
            block.get_args(self.library).for_each(|a| {
                self.live_values.insert(a);
            });

            for inst in block.get_insts(self.library) {
                match inst.get_inst(self.library) {
                    Instruction::Return(_) => {
                        let ty = function.get_return_type(self.library);

                        if !ty.is_void(self.library) {
                            return Err(VerifyError::ReturnDoesNotMatchType(
                                self.library,
                                inst,
                                ty,
                            ));
                        }
                    }
                    Instruction::ReturnValue(ret_ty, value, _) => {
                        let func_ty = function.get_return_type(self.library);

                        if *ret_ty != func_ty {
                            return Err(VerifyError::ReturnValueDoesNotMatchType(
                                self.library,
                                inst,
                                *ret_ty,
                                func_ty,
                            ));
                        }

                        self.verify_live(inst, *value)?;
                    }
                    Instruction::Branch(block, inst_args, _) => {
                        self.verify_branch_and_block(inst, inst_args, *block)?;
                    }
                    Instruction::Cmp(_, _, a, b, _) => {
                        let a_ty = a.get_type(self.library);
                        let b_ty = b.get_type(self.library);

                        if a_ty != b_ty {
                            return Err(VerifyError::ValueTypesMustMatch(
                                self.library,
                                inst,
                                *a,
                                *b,
                            ));
                        }

                        if !(a_ty.is_integral_or_integral_vector(self.library)
                            || a_ty.is_float_or_float_vector(self.library))
                        {
                            return Err(VerifyError::TypeMustBeFloatOrIntVector(
                                self.library,
                                inst,
                                *a,
                            ));
                        }

                        self.verify_live(inst, *a)?;
                        self.verify_live(inst, *b)?;
                    }
                    Instruction::Unary(_, o, v, _) => {
                        let ty = v.get_type(self.library);

                        if *o == Unary::Neg
                            && !(ty.is_integral_or_integral_vector(self.library)
                                || ty.is_float_or_float_vector(self.library))
                        {
                            return Err(VerifyError::TypeMustBeFloatOrIntVector(
                                self.library,
                                inst,
                                *v,
                            ));
                        }

                        if *o == Unary::Not
                            && !(ty.is_bool_or_bool_vector(self.library)
                                || ty.is_integral_or_integral_vector(self.library)
                                || ty.is_float_or_float_vector(self.library))
                        {
                            return Err(VerifyError::TypeMustBeBoolOrFloatOrIntVector(
                                self.library,
                                inst,
                                *v,
                            ));
                        }

                        self.verify_live(inst, *v)?;
                    }
                    Instruction::Binary(_, o, a, b, _) => {
                        let a_ty = a.get_type(self.library);
                        let b_ty = b.get_type(self.library);

                        if a_ty != b_ty {
                            return Err(VerifyError::ValueTypesMustMatch(
                                self.library,
                                inst,
                                *a,
                                *b,
                            ));
                        }

                        match o {
                            Binary::And | Binary::Or | Binary::Xor => {
                                if !(a_ty.is_bool_or_bool_vector(self.library)
                                    || a_ty.is_integral_or_integral_vector(self.library))
                                {
                                    return Err(VerifyError::TypeMustBeBoolOrIntVector(
                                        self.library,
                                        inst,
                                        *a,
                                    ));
                                }
                            }
                            Binary::Shl | Binary::Shr => {
                                if !a_ty.is_integral_or_integral_vector(self.library) {
                                    return Err(VerifyError::TypeMustBeIntVector(
                                        self.library,
                                        inst,
                                        *a,
                                    ));
                                }
                            }
                            _ => {
                                if !(a_ty.is_float_or_float_vector(self.library)
                                    || a_ty.is_integral_or_integral_vector(self.library))
                                {
                                    return Err(VerifyError::TypeMustBeFloatOrIntVector(
                                        self.library,
                                        inst,
                                        *a,
                                    ));
                                }
                            }
                        }

                        self.verify_live(inst, *a)?;
                        self.verify_live(inst, *b)?;
                    }
                    Instruction::Cast(ty, v, _) => {
                        let v_ty = v.get_type(self.library);

                        if !(v_ty.is_float_or_float_vector(self.library)
                            || v_ty.is_integral_or_integral_vector(self.library))
                        {
                            return Err(VerifyError::TypeMustBeFloatOrIntVector(
                                self.library,
                                inst,
                                *v,
                            ));
                        }

                        if !(ty.is_float_or_float_vector(self.library)
                            || ty.is_integral_or_integral_vector(self.library))
                        {
                            return Err(VerifyError::TypeMustBeFloatOrIntVector(
                                self.library,
                                inst,
                                *v,
                            ));
                        }

                        if ty.is_vector(self.library) ^ v_ty.is_vector(self.library) {
                            return Err(VerifyError::VectorWidthsMustMatch(
                                self.library,
                                inst,
                                *v,
                                *ty,
                            ));
                        }

                        if ty.is_vector(self.library)
                            && v_ty.is_vector(self.library)
                            && ty.get_len(self.library) != v_ty.get_len(self.library)
                        {
                            return Err(VerifyError::VectorWidthsMustMatch(
                                self.library,
                                inst,
                                *v,
                                *ty,
                            ));
                        }

                        self.verify_live(inst, *v)?;
                    }
                    Instruction::Call(func, args, _) => {
                        for arg in args {
                            self.verify_live(inst, *arg)?;
                        }

                        if args.len() != func.get_num_args(self.library) {
                            return Err(VerifyError::CallAndFunctionMustHaveMatchingArguments(
                                self.library,
                                *func,
                                inst,
                            ));
                        }

                        for (carg, farg) in args.iter().zip(func.get_args(self.library)) {
                            if carg.get_type(self.library) != farg.get_type(self.library) {
                                return Err(VerifyError::CallAndFunctionMustHaveMatchingArguments(
                                    self.library,
                                    *func,
                                    inst,
                                ));
                            }
                        }
                    }
                    Instruction::ConditionalBranch(
                        cond,
                        true_block,
                        false_block,
                        true_args,
                        false_args,
                        _,
                    ) => {
                        self.verify_branch_and_block(inst, true_args, *true_block)?;
                        self.verify_branch_and_block(inst, false_args, *false_block)?;

                        let cond_ty = cond.get_type(self.library);

                        if !cond_ty.is_boolean(self.library) {
                            return Err(VerifyError::ConditionMustBeBool(
                                self.library,
                                inst,
                                *cond,
                                cond_ty,
                            ));
                        }

                        self.verify_live(inst, *cond)?;
                    }
                    _ => todo!("{}", inst.get_inst_displayer(self.library)),
                }

                self.live_values.insert(inst);
            }

            // Remove all the arguments which are no longer live.
            block.get_args(self.library).for_each(|a| {
                self.live_values.remove(&a);
            });
        }

        Ok(())
    }

    fn verify_module(&mut self, module: Module) -> Result<(), VerifyError<'a>> {
        for function in module.get_functions(self.library) {
            self.verify_function(function)?;
        }

        Ok(())
    }
}

pub(crate) fn verify(library: &Library, module: Module) -> Result<(), VerifyError> {
    let mut verifier = Verifier::new(library);
    verifier.verify_module(module)
}
