extern crate codespan;
extern crate rmp_serde;
extern crate serde;

#[cfg(feature = "nightly")]
mod benchmarks;

use crate::*;
use codespan::{FileId, Span};
use codespan_reporting::diagnostic::{Diagnostic, Label};
use std::collections::HashMap;
use std::str::FromStr;

struct Assembler<'a> {
    data: &'a str,
    offset: u32,
    file: FileId,
    modules: HashMap<&'a str, Module>,
    functions: HashMap<(&'a str, Module), Function>,
    variables: HashMap<&'a str, Value>,
    structs: HashMap<&'a str, Type>,
    current_blocks: HashMap<&'a str, Block>,
    current_values: HashMap<&'a str, Value>,
    current_module: Option<Module>,
    current_function: Option<Function>,
}

impl<'a> Assembler<'a> {
    pub fn new(file: FileId, data: &'a str) -> Self {
        Self {
            data,
            offset: 0,
            file,
            modules: HashMap::new(),
            functions: HashMap::new(),
            variables: HashMap::new(),
            structs: HashMap::new(),
            current_blocks: HashMap::new(),
            current_values: HashMap::new(),
            current_module: None,
            current_function: None,
        }
    }

    fn get_current_char(&self) -> Option<char> {
        self.get_current_str().chars().next()
    }

    fn get_current_str(&self) -> &'a str {
        &self.data[(self.offset as usize)..]
    }

    fn peek_if_next_symbol(&mut self, str: &str) -> bool {
        self.skip_comments_or_whitespace();

        self.get_current_str().starts_with(str)
    }

    fn pop_if_next_symbol(&mut self, str: &str) -> Result<bool, Diagnostic> {
        self.skip_comments_or_whitespace();

        if self.get_current_str().is_empty() {
            Err(Diagnostic::new_error(
                "unexpected end of file",
                Label::new(self.file, self.single_char_span(), "here"),
            ))
        } else if self.get_current_str().starts_with(str) {
            self.bump_current_by(str.len());
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn bump_current(&mut self) {
        self.offset += self.get_current_char().unwrap().len_utf8() as u32;
    }

    fn bump_current_by(&mut self, len: usize) {
        for _ in 0..len {
            self.offset += self.get_current_char().unwrap().len_utf8() as u32;
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.get_current_char() {
            if !c.is_whitespace() {
                break;
            }

            self.bump_current();
        }
    }

    fn skip_comments_or_whitespace(&mut self) {
        self.skip_whitespace();

        // Skip any of our allowed comments '//' -> newline
        while self.get_current_str().starts_with("//") {
            // Skip the two '//' characfters.
            self.bump_current();
            self.bump_current();

            while let Some(c) = self.get_current_char() {
                match c {
                    '\n' => break,
                    '\r' => break,
                    _ => self.bump_current(),
                }
            }

            self.skip_whitespace();
        }
    }

    fn make_span(&self, start: u32, end: u32) -> Span {
        let my_start = if start as usize == self.data.len() {
            start - 1
        } else {
            start
        };

        Span::new(my_start, end)
    }

    fn single_char_span(&self) -> Span {
        let offset = if self.offset as usize == self.data.len() {
            self.offset - 1
        } else {
            self.offset
        };

        Span::new(offset, offset + 1)
    }

    fn parse_quoted_identifier(&mut self) -> Result<&'a str, Diagnostic> {
        // Skip the leading quote '"'.
        self.bump_current();

        let start = self.offset as usize;

        loop {
            let c = match self.get_current_char() {
                Some(c) => c,
                None => {
                    return Err(Diagnostic::new_error(
                        "Expected closing quote",
                        Label::new(self.file, self.single_char_span(), "missing quote '\"'"),
                    ));
                }
            };

            if c == '"' {
                break;
            }

            self.bump_current();
        }

        let end = self.offset as usize;

        // Skip the trailing quote.
        self.bump_current();

        Ok(&self.data[start..end])
    }

    fn parse_unquoted_identifier(&mut self) -> &'a str {
        let start = self.offset as usize;

        while let Some(c) = self.get_current_char() {
            if !c.is_ascii_alphanumeric() && c != '_' {
                break;
            }

            self.bump_current();
        }

        &self.data[start..(self.offset as usize)]
    }

    fn unexpected_end_of_file(&mut self) -> Diagnostic {
        Diagnostic::new_error(
            "Unexpected end of file",
            Label::new(self.file, self.single_char_span(), "missing identifier"),
        )
    }

    fn parse_identifier_with_span(&mut self) -> Result<(&'a str, Span), Diagnostic> {
        self.skip_comments_or_whitespace();

        if self.get_current_str().is_empty() {
            return Err(self.unexpected_end_of_file());
        }

        let start = self.offset;

        let identifier = if self.peek_if_next_symbol("\"") {
            self.parse_quoted_identifier()?
        } else {
            self.parse_unquoted_identifier()
        };

        let end = self.offset;

        Ok((identifier, self.make_span(start, end)))
    }

    fn parse_identifier(&mut self) -> Result<&'a str, Diagnostic> {
        Ok(self.parse_identifier_with_span()?.0)
    }

    fn try_parse_int_or_float_val(
        &mut self,
        context: &mut Context,
        c: char,
        bits: u8,
    ) -> Option<Type> {
        let str = c.to_string() + &bits.to_string();

        if !self.get_current_str().starts_with(&str) {
            return None;
        }

        self.bump_current_by(str.len());

        match c {
            'i' => Some(context.get_int_type(bits)),
            'u' => Some(context.get_uint_type(bits)),
            'f' => Some(context.get_float_type(bits)),
            _ => None,
        }
    }

    fn try_parse_int_or_float(&mut self, context: &mut Context, c: char) -> Option<Type> {
        if let Some(x) = self.try_parse_int_or_float_val(context, c, 8) {
            return Some(x);
        }

        if let Some(x) = self.try_parse_int_or_float_val(context, c, 16) {
            return Some(x);
        }

        if let Some(x) = self.try_parse_int_or_float_val(context, c, 32) {
            return Some(x);
        }

        if let Some(x) = self.try_parse_int_or_float_val(context, c, 64) {
            return Some(x);
        }

        None
    }

    fn parse_struct_type(&mut self, context: &mut Context) -> Result<Type, Diagnostic> {
        // Skip the '{'
        self.bump_current();

        let mut element_types = Vec::new();

        loop {
            self.skip_comments_or_whitespace();

            if self.get_current_str().is_empty() {
                return Err(Diagnostic::new_error(
                    "Expected '}' to close a struct",
                    Label::new(self.file, self.single_char_span(), "should be '}'"),
                ));
            }

            if self.get_current_str().starts_with('}') {
                // Skip the '}'
                self.bump_current();

                break;
            }

            let element_type = self.parse_type(context)?;

            element_types.push(element_type);

            self.skip_comments_or_whitespace();

            if self.get_current_str().starts_with('}') {
                // Skip the '}'
                self.bump_current();

                break;
            }

            if !self.get_current_str().starts_with(',') {
                return Err(Diagnostic::new_error(
                    "Expected ',' between elements of a struct",
                    Label::new(self.file, self.single_char_span(), "should be ','"),
                ));
            }

            // Skip the ','
            self.bump_current();
        }

        Ok(context.get_struct_type(&element_types))
    }

    fn parse_literal<T: FromStr>(&mut self) -> Result<T, Diagnostic> {
        self.skip_comments_or_whitespace();

        let mut str = self.get_current_str();

        let start = str.len();

        loop {
            if !(str.starts_with(char::is_numeric)
                || str.starts_with('-')
                || str.starts_with('.')
                || str.starts_with('e'))
            {
                break;
            } else {
                str = &str[1..];
            }
        }

        let len = start - str.len();

        if len == 0 {
            return Err(Diagnostic::new_error(
                "Unexpected empty literal",
                Label::new(self.file, self.single_char_span(), "expected literal here"),
            ));
        }

        let split = self.get_current_str().split_at(len).0;

        // Skip the literal.
        self.bump_current_by(split.len());

        match split.parse::<T>() {
            Ok(t) => Ok(t),
            Err(_) => Err(Diagnostic::new_error(
                "Literal did not parse as the correct type",
                Label::new(self.file, self.single_char_span(), "here"),
            )),
        }
    }

    fn parse_constant(&mut self, context: &mut Context, ty: Type) -> Result<Value, Diagnostic> {
        if ty.is_int(context) {
            let cnst = self.parse_literal()?;
            Ok(context.get_int_constant(ty.get_bits(context) as u8, cnst))
        } else if ty.is_uint(context) {
            let cnst = self.parse_literal()?;
            Ok(context.get_uint_constant(ty.get_bits(context) as u8, cnst))
        } else if ty.is_boolean(context) {
            if self.pop_if_next_symbol("true")? {
                Ok(context.get_bool_constant(true))
            } else if self.pop_if_next_symbol("false")? {
                Ok(context.get_bool_constant(false))
            } else {
                Err(Diagnostic::new_error(
                    "Expected 'true' or 'false' for boolean constant",
                    Label::new(self.file, self.single_char_span(), "here"),
                ))
            }
        } else if ty.is_pointer(context) {
            if self.pop_if_next_symbol("null")? {
                Ok(context.get_pointer_constant_null(ty))
            } else {
                Err(Diagnostic::new_error(
                    "Expected 'true' or 'false' for boolean constant",
                    Label::new(self.file, self.single_char_span(), "here"),
                ))
            }
        } else if ty.is_float(context) {
            let cnst = self.parse_literal()?;
            Ok(context.get_float_constant(ty.get_bits(context) as u8, cnst))
        } else if ty.is_array(context) {
            if !self.pop_if_next_symbol("[")? {
                return Err(Diagnostic::new_error(
                    "Expected '[' to open an array constant",
                    Label::new(self.file, self.single_char_span(), "here"),
                ));
            }

            let len = ty.get_len(context);

            // Array elements have all the same type.
            let elem_ty = ty.get_element(context, 0);

            let mut constants = vec![self.parse_constant(context, elem_ty)?];

            for _ in 1..len {
                if !self.pop_if_next_symbol(",")? {
                    return Err(Diagnostic::new_error(
                        "Expected ',' between array constant elements",
                        Label::new(self.file, self.single_char_span(), "here"),
                    ));
                }

                constants.push(self.parse_constant(context, elem_ty)?);
            }

            if !self.pop_if_next_symbol("]")? {
                return Err(Diagnostic::new_error(
                    "Expected ']' to close an array constant",
                    Label::new(self.file, self.single_char_span(), "here"),
                ));
            }

            Ok(context.get_composite_constant(ty, &constants))
        } else if ty.is_vector(context) {
            if !self.pop_if_next_symbol("<")? {
                return Err(Diagnostic::new_error(
                    "Expected '<' to open a vector constant",
                    Label::new(self.file, self.single_char_span(), "here"),
                ));
            }

            let len = ty.get_len(context);

            // Vector elements have all the same type.
            let elem_ty = ty.get_element(context, 0);

            let mut constants = vec![self.parse_constant(context, elem_ty)?];

            for _ in 1..len {
                if !self.pop_if_next_symbol(",")? {
                    return Err(Diagnostic::new_error(
                        "Expected ',' between vector constant elements",
                        Label::new(self.file, self.single_char_span(), "here"),
                    ));
                }

                constants.push(self.parse_constant(context, elem_ty)?);
            }

            if !self.pop_if_next_symbol(">")? {
                return Err(Diagnostic::new_error(
                    "Expected '>' to close a vector constant",
                    Label::new(self.file, self.single_char_span(), "here"),
                ));
            }

            Ok(context.get_composite_constant(ty, &constants))
        } else if ty.is_struct(context) {
            if !self.pop_if_next_symbol("{")? {
                return Err(Diagnostic::new_error(
                    "Expected '{' to open a struct constant",
                    Label::new(self.file, self.single_char_span(), "here"),
                ));
            }

            let len = ty.get_len(context);

            let mut constants = vec![self.parse_constant(context, ty.get_element(context, 0))?];

            for i in 1..len {
                if !self.pop_if_next_symbol(",")? {
                    return Err(Diagnostic::new_error(
                        "Expected ',' between struct constant elements",
                        Label::new(self.file, self.single_char_span(), "here"),
                    ));
                }

                constants.push(self.parse_constant(context, ty.get_element(context, i))?);
            }

            if !self.pop_if_next_symbol("}")? {
                return Err(Diagnostic::new_error(
                    "Expected '}' to close a struct constant",
                    Label::new(self.file, self.single_char_span(), "here"),
                ));
            }

            Ok(context.get_composite_constant(ty, &constants))
        } else {
            std::unreachable!();
        }
    }

    fn parse_vector_type(&mut self, context: &mut Context) -> Result<Type, Diagnostic> {
        // Skip the '<'
        self.bump_current();

        self.skip_comments_or_whitespace();

        let element_type = self.parse_type(context)?;

        self.skip_comments_or_whitespace();

        if !self.get_current_str().starts_with(',') {
            return Err(Diagnostic::new_error(
                "Vector type was malformed",
                Label::new(self.file, self.single_char_span(), "missing ','"),
            ));
        }

        // Skip the ','
        self.bump_current();

        let width = self.parse_literal()?;

        if !self.get_current_str().starts_with('>') {
            return Err(Diagnostic::new_error(
                "Vector type was malformed",
                Label::new(self.file, self.single_char_span(), "missing '>'"),
            ));
        }

        // Skip the '>'
        self.bump_current();

        Ok(context.get_vector_type(element_type, width))
    }

    fn parse_array_type(&mut self, context: &mut Context) -> Result<Type, Diagnostic> {
        // Skip the '['
        self.bump_current();

        self.skip_comments_or_whitespace();

        let element_type = self.parse_type(context)?;

        self.skip_comments_or_whitespace();

        if !self.get_current_str().starts_with(',') {
            return Err(Diagnostic::new_error(
                "Array type was malformed",
                Label::new(self.file, self.single_char_span(), "missing ','"),
            ));
        }

        // Skip the ','
        self.bump_current();

        let len = self.parse_literal()?;

        if !self.get_current_str().starts_with(']') {
            return Err(Diagnostic::new_error(
                "Array type was malformed",
                Label::new(self.file, self.single_char_span(), "missing ']'"),
            ));
        }

        // Skip the ']'
        self.bump_current();

        Ok(context.get_array_type(element_type, len))
    }

    fn parse_domain(&mut self) -> Result<Domain, Diagnostic> {
        if self.get_current_str().starts_with("any") {
            self.bump_current_by("any".len());
            Ok(Domain::CrossDevice)
        } else if self.get_current_str().starts_with("cpu") {
            self.bump_current_by("cpu".len());
            Ok(Domain::Cpu)
        } else if self.get_current_str().starts_with("gpu") {
            self.bump_current_by("gpu".len());
            Ok(Domain::Gpu)
        } else if self.get_current_str().starts_with("stack") {
            self.bump_current_by("stack".len());
            Ok(Domain::Stack)
        } else {
            Err(Diagnostic::new_error(
                "Invalid pointer domain - expected any, cpu, gpu, or stack",
                Label::new(self.file, self.single_char_span(), "unknown domain"),
            ))
        }
    }

    fn parse_pointer_type(&mut self, context: &mut Context) -> Result<Type, Diagnostic> {
        // Skiop the '*'
        self.bump_current();

        self.skip_comments_or_whitespace();

        let domain = self.parse_domain()?;

        Ok(context.get_pointer_type(domain))
    }

    fn parse_type(&mut self, context: &mut Context) -> Result<Type, Diagnostic> {
        self.skip_comments_or_whitespace();

        // Do we have a named struct?
        if self.pop_if_next_symbol("%")? {
            let identifier = self.parse_identifier()?;

            if !self.structs.contains_key(identifier) {
                return Err(Diagnostic::new_error(
                    "Unknown named struct type",
                    Label::new(self.file, self.single_char_span(), "unknown type"),
                ));
            }

            return Ok(self.structs[identifier]);
        }

        if let Some(t) = self.try_parse_int_or_float(context, 'i') {
            return Ok(t);
        }

        if let Some(t) = self.try_parse_int_or_float(context, 'u') {
            return Ok(t);
        }

        if let Some(t) = self.try_parse_int_or_float(context, 'f') {
            return Ok(t);
        }

        if self.get_current_str().starts_with("void") {
            self.bump_current_by("void".len());
            Ok(context.get_void_type())
        } else if self.get_current_str().starts_with("bool") {
            self.bump_current_by("bool".len());
            Ok(context.get_bool_type())
        } else if self.get_current_str().starts_with('<') {
            self.parse_vector_type(context)
        } else if self.get_current_str().starts_with('{') {
            self.parse_struct_type(context)
        } else if self.get_current_str().starts_with('[') {
            self.parse_array_type(context)
        } else if self.get_current_str().starts_with('*') {
            self.parse_pointer_type(context)
        } else {
            Err(Diagnostic::new_error(
                "Could not deduce type",
                Label::new(self.file, self.single_char_span(), "unknown type"),
            ))
        }
    }

    fn parse_value(&mut self) -> Result<Value, Diagnostic> {
        let (name, span) = self.parse_identifier_with_span()?;

        // First we check the current values.
        if let Some(v) = self.current_values.get(name) {
            return Ok(*v);
        }

        // Then we check for a global with the name.
        if let Some(v) = self.variables.get(name) {
            return Ok(*v);
        }

        Err(Diagnostic::new_error(
            "Unknown identified value",
            Label::new(self.file, span, "no match for this name"),
        ))
    }

    fn parse_block(&mut self, context: &mut Context) -> Result<(), Diagnostic> {
        self.skip_comments_or_whitespace();

        let name = self.parse_identifier()?;

        if !self.pop_if_next_symbol("(")? {
            return Err(Diagnostic::new_error(
                "Expected '(' to open block arguments",
                Label::new(self.file, self.single_char_span(), "unknown token"),
            ));
        }

        let function = self.current_function.unwrap();

        let mut args = Vec::new();

        while !self.pop_if_next_symbol(")")? {
            args.push(self.parse_arg(context)?);
        }

        if !self.pop_if_next_symbol(":")? {
            return Err(Diagnostic::new_error(
                "Expected ':' between block definition and its instruction",
                Label::new(self.file, self.single_char_span(), "here"),
            ));
        }

        let block = if let Some(block) = self.current_blocks.get(name) {
            let mut block_args = block.get_args_mut(context);
            args.iter().for_each(|(_, b)| block_args.push(*b));
            *block
        } else {
            let mut builder = function.create_block(context);

            for (_, ty) in &args {
                builder = builder.with_arg(*ty);
            }

            let block = builder.build();

            self.current_blocks.insert(name, block);

            block
        };

        for (i, arg) in args.iter().enumerate().take(block.get_num_args(context)) {
            self.current_values.insert(arg.0, block.get_arg(context, i));
        }

        let func_ret_is_void = self
            .current_function
            .unwrap()
            .get_return_type(context)
            .is_void(context);
        let mut builder = block.create_instructions(context);

        loop {
            if self.pop_if_next_symbol("ret")? {
                let paused = builder.pause_building();
                let loc = self.parse_loc(context)?;
                builder = InstructionBuilder::resume_building(context, paused);

                if func_ret_is_void {
                    builder.ret(loc);
                } else {
                    builder.ret_val(self.parse_value()?, loc);
                }

                break;
            } else if self.pop_if_next_symbol("store")? {
                let paused_builder = builder.pause_building();
                let ty = self.parse_type(context)?;
                builder = InstructionBuilder::resume_building(context, paused_builder);

                if !self.pop_if_next_symbol(",")? {
                    return Err(Diagnostic::new_error(
                        "Expected ',' between arguments to an instruction",
                        Label::new(self.file, self.single_char_span(), "here"),
                    ));
                }

                let ptr = self.parse_value()?;

                if !self.pop_if_next_symbol(",")? {
                    return Err(Diagnostic::new_error(
                        "Expected ',' between arguments to an instruction",
                        Label::new(self.file, self.single_char_span(), "here"),
                    ));
                }

                let value = self.parse_value()?;

                let paused = builder.pause_building();
                let loc = self.parse_loc(context)?;
                builder = InstructionBuilder::resume_building(context, paused);

                builder.store(ty, ptr, value, loc);
            } else if self.pop_if_next_symbol("br")? {
                let name = self.parse_identifier()?;

                if !self.pop_if_next_symbol("(")? {
                    return Err(Diagnostic::new_error(
                        "Expected ',' between arguments to a function call",
                        Label::new(self.file, self.single_char_span(), "here"),
                    ));
                }

                let mut args = Vec::new();

                loop {
                    self.skip_comments_or_whitespace();

                    if self.peek_if_next_symbol(")") {
                        self.bump_current();
                        break;
                    }

                    let value = self.parse_value()?;
                    args.push(value);

                    self.skip_comments_or_whitespace();

                    if self.peek_if_next_symbol(",") {
                        self.bump_current();
                        continue;
                    }
                }

                let block = if let Some(block) = self.current_blocks.get(name) {
                    *block
                } else {
                    let paused_builder = builder.pause_building();

                    let block_builder = self.current_function.unwrap().create_block(context);

                    // Make the block with no args for now, we'll fill it out later when we actually hit the creation of the block.

                    let block = block_builder.build();

                    self.current_blocks.insert(name, block);

                    builder = InstructionBuilder::resume_building(context, paused_builder);

                    block
                };

                let paused = builder.pause_building();
                let loc = self.parse_loc(context)?;
                builder = InstructionBuilder::resume_building(context, paused);

                builder.branch(block, &args, loc);

                break;
            } else if self.pop_if_next_symbol("cbr")? {
                let cond = self.parse_value()?;

                if !self.pop_if_next_symbol(",")? {
                    return Err(Diagnostic::new_error(
                        "Expected ',' between parts of a conditional branch",
                        Label::new(self.file, self.single_char_span(), "here"),
                    ));
                }

                let true_br = self.parse_identifier()?;

                if !self.pop_if_next_symbol("(")? {
                    return Err(Diagnostic::new_error(
                        "Expected '(' to open arguments to a conditional branch",
                        Label::new(self.file, self.single_char_span(), "here"),
                    ));
                }

                let mut true_args = Vec::new();

                loop {
                    self.skip_comments_or_whitespace();

                    if self.peek_if_next_symbol(")") {
                        self.bump_current();
                        break;
                    }

                    let value = self.parse_value()?;
                    true_args.push(value);

                    self.skip_comments_or_whitespace();

                    if self.peek_if_next_symbol(",") {
                        self.bump_current();
                        continue;
                    }
                }

                let true_block = if let Some(block) = self.current_blocks.get(true_br) {
                    *block
                } else {
                    let paused_builder = builder.pause_building();

                    let block_builder = self.current_function.unwrap().create_block(context);

                    // Make the block with no args for now, we'll fill it out later when we actually hit the creation of the block.

                    let block = block_builder.build();

                    self.current_blocks.insert(true_br, block);

                    builder = InstructionBuilder::resume_building(context, paused_builder);

                    block
                };

                if !self.pop_if_next_symbol(",")? {
                    return Err(Diagnostic::new_error(
                        "Expected ',' between parts of a conditional branch",
                        Label::new(self.file, self.single_char_span(), "here"),
                    ));
                }

                let false_br = self.parse_identifier()?;

                if !self.pop_if_next_symbol("(")? {
                    return Err(Diagnostic::new_error(
                        "Expected '(' to open arguments to a conditional branch",
                        Label::new(self.file, self.single_char_span(), "here"),
                    ));
                }

                let mut false_args = Vec::new();

                loop {
                    self.skip_comments_or_whitespace();

                    if self.peek_if_next_symbol(")") {
                        self.bump_current();
                        break;
                    }

                    let value = self.parse_value()?;
                    false_args.push(value);

                    self.skip_comments_or_whitespace();

                    if self.peek_if_next_symbol(",") {
                        self.bump_current();
                        continue;
                    }
                }

                let false_block = if let Some(block) = self.current_blocks.get(false_br) {
                    *block
                } else {
                    let paused_builder = builder.pause_building();

                    let block_builder = self.current_function.unwrap().create_block(context);

                    // Make the block with no args for now, we'll fill it out later when we actually hit the creation of the block.

                    let block = block_builder.build();

                    self.current_blocks.insert(false_br, block);

                    builder = InstructionBuilder::resume_building(context, paused_builder);

                    block
                };

                let paused = builder.pause_building();
                let loc = self.parse_loc(context)?;
                builder = InstructionBuilder::resume_building(context, paused);

                builder.conditional_branch(
                    cond,
                    true_block,
                    false_block,
                    &true_args,
                    &false_args,
                    loc,
                );

                break;
            } else {
                // Everything below here assigns into an SSA variable.
                let identifier = self.parse_identifier()?;

                if !self.pop_if_next_symbol("=")? {
                    return Err(Diagnostic::new_error(
                        "Expected '=' between symbol and instruction",
                        Label::new(self.file, self.single_char_span(), "here"),
                    ));
                }

                if self.pop_if_next_symbol("extract")? {
                    let aggregate = self.parse_value()?;

                    if !self.pop_if_next_symbol(",")? {
                        return Err(Diagnostic::new_error(
                            "Expected ',' between arguments to an instruction",
                            Label::new(self.file, self.single_char_span(), "here"),
                        ));
                    }

                    let index = self.parse_literal()?;

                    let paused = builder.pause_building();
                    let loc = self.parse_loc(context)?;
                    builder = InstructionBuilder::resume_building(context, paused);

                    let value = builder.extract(aggregate, index, loc);

                    self.current_values.insert(identifier, value);
                } else if self.pop_if_next_symbol("insert")? {
                    let aggregate = self.parse_value()?;

                    if !self.pop_if_next_symbol(",")? {
                        return Err(Diagnostic::new_error(
                            "Expected ',' between arguments to an instruction",
                            Label::new(self.file, self.single_char_span(), "here"),
                        ));
                    }

                    let value = self.parse_value()?;

                    if !self.pop_if_next_symbol(",")? {
                        return Err(Diagnostic::new_error(
                            "Expected ',' between arguments to an instruction",
                            Label::new(self.file, self.single_char_span(), "here"),
                        ));
                    }

                    let index = self.parse_literal()?;

                    let paused = builder.pause_building();
                    let loc = self.parse_loc(context)?;
                    builder = InstructionBuilder::resume_building(context, paused);

                    let value = builder.insert(aggregate, value, index, loc);

                    self.current_values.insert(identifier, value);
                } else if self.pop_if_next_symbol("cmp")? {
                    let cmp = if self.pop_if_next_symbol("eq")? {
                        Cmp::Eq
                    } else if self.pop_if_next_symbol("ne")? {
                        Cmp::Ne
                    } else if self.pop_if_next_symbol("lt")? {
                        Cmp::Lt
                    } else if self.pop_if_next_symbol("le")? {
                        Cmp::Le
                    } else if self.pop_if_next_symbol("gt")? {
                        Cmp::Gt
                    } else if self.pop_if_next_symbol("ge")? {
                        Cmp::Ge
                    } else {
                        return Err(Diagnostic::new_error(
                            "Could not parse the kind of the compare (should be one of eq, ne, lt, le, gt, ge)",
                            Label::new(self.file, self.single_char_span(), "here"),
                        ));
                    };

                    let lhs = self.parse_value()?;

                    if !self.pop_if_next_symbol(",")? {
                        return Err(Diagnostic::new_error(
                            "Expected ',' between arguments to an instruction",
                            Label::new(self.file, self.single_char_span(), "here"),
                        ));
                    }

                    let rhs = self.parse_value()?;

                    let paused = builder.pause_building();
                    let loc = self.parse_loc(context)?;
                    builder = InstructionBuilder::resume_building(context, paused);

                    let value = builder.cmp(cmp, lhs, rhs, loc);

                    self.current_values.insert(identifier, value);
                } else if self.pop_if_next_symbol("add")? {
                    let lhs = self.parse_value()?;

                    if !self.pop_if_next_symbol(",")? {
                        return Err(Diagnostic::new_error(
                            "Expected ',' between arguments to an instruction",
                            Label::new(self.file, self.single_char_span(), "here"),
                        ));
                    }

                    let rhs = self.parse_value()?;

                    let paused = builder.pause_building();
                    let loc = self.parse_loc(context)?;
                    builder = InstructionBuilder::resume_building(context, paused);

                    let value = builder.add(lhs, rhs, loc);

                    self.current_values.insert(identifier, value);
                } else if self.pop_if_next_symbol("sub")? {
                    let lhs = self.parse_value()?;

                    if !self.pop_if_next_symbol(",")? {
                        return Err(Diagnostic::new_error(
                            "Expected ',' between arguments to an instruction",
                            Label::new(self.file, self.single_char_span(), "here"),
                        ));
                    }

                    let rhs = self.parse_value()?;

                    let paused = builder.pause_building();
                    let loc = self.parse_loc(context)?;
                    builder = InstructionBuilder::resume_building(context, paused);

                    let value = builder.sub(lhs, rhs, loc);

                    self.current_values.insert(identifier, value);
                } else if self.pop_if_next_symbol("mul")? {
                    let lhs = self.parse_value()?;

                    if !self.pop_if_next_symbol(",")? {
                        return Err(Diagnostic::new_error(
                            "Expected ',' between arguments to an instruction",
                            Label::new(self.file, self.single_char_span(), "here"),
                        ));
                    }

                    let rhs = self.parse_value()?;

                    let paused = builder.pause_building();
                    let loc = self.parse_loc(context)?;
                    builder = InstructionBuilder::resume_building(context, paused);

                    let value = builder.mul(lhs, rhs, loc);

                    self.current_values.insert(identifier, value);
                } else if self.pop_if_next_symbol("div")? {
                    let lhs = self.parse_value()?;

                    if !self.pop_if_next_symbol(",")? {
                        return Err(Diagnostic::new_error(
                            "Expected ',' between arguments to an instruction",
                            Label::new(self.file, self.single_char_span(), "here"),
                        ));
                    }

                    let rhs = self.parse_value()?;

                    let paused = builder.pause_building();
                    let loc = self.parse_loc(context)?;
                    builder = InstructionBuilder::resume_building(context, paused);

                    let value = builder.div(lhs, rhs, loc);

                    self.current_values.insert(identifier, value);
                } else if self.pop_if_next_symbol("rem")? {
                    let lhs = self.parse_value()?;

                    if !self.pop_if_next_symbol(",")? {
                        return Err(Diagnostic::new_error(
                            "Expected ',' between arguments to an instruction",
                            Label::new(self.file, self.single_char_span(), "here"),
                        ));
                    }

                    let rhs = self.parse_value()?;

                    let paused = builder.pause_building();
                    let loc = self.parse_loc(context)?;
                    builder = InstructionBuilder::resume_building(context, paused);

                    let value = builder.rem(lhs, rhs, loc);

                    self.current_values.insert(identifier, value);
                } else if self.pop_if_next_symbol("neg")? {
                    let lhs = self.parse_value()?;

                    let paused = builder.pause_building();
                    let loc = self.parse_loc(context)?;
                    builder = InstructionBuilder::resume_building(context, paused);

                    let value = builder.neg(lhs, loc);

                    self.current_values.insert(identifier, value);
                } else if self.pop_if_next_symbol("and")? {
                    let lhs = self.parse_value()?;

                    if !self.pop_if_next_symbol(",")? {
                        return Err(Diagnostic::new_error(
                            "Expected ',' between arguments to an instruction",
                            Label::new(self.file, self.single_char_span(), "here"),
                        ));
                    }

                    let rhs = self.parse_value()?;

                    let paused = builder.pause_building();
                    let loc = self.parse_loc(context)?;
                    builder = InstructionBuilder::resume_building(context, paused);

                    let value = builder.and(lhs, rhs, loc);

                    self.current_values.insert(identifier, value);
                } else if self.pop_if_next_symbol("or")? {
                    let lhs = self.parse_value()?;

                    if !self.pop_if_next_symbol(",")? {
                        return Err(Diagnostic::new_error(
                            "Expected ',' between arguments to an instruction",
                            Label::new(self.file, self.single_char_span(), "here"),
                        ));
                    }

                    let rhs = self.parse_value()?;

                    let paused = builder.pause_building();
                    let loc = self.parse_loc(context)?;
                    builder = InstructionBuilder::resume_building(context, paused);

                    let value = builder.or(lhs, rhs, loc);

                    self.current_values.insert(identifier, value);
                } else if self.pop_if_next_symbol("xor")? {
                    let lhs = self.parse_value()?;

                    if !self.pop_if_next_symbol(",")? {
                        return Err(Diagnostic::new_error(
                            "Expected ',' between arguments to an instruction",
                            Label::new(self.file, self.single_char_span(), "here"),
                        ));
                    }

                    let rhs = self.parse_value()?;

                    let paused = builder.pause_building();
                    let loc = self.parse_loc(context)?;
                    builder = InstructionBuilder::resume_building(context, paused);

                    let value = builder.xor(lhs, rhs, loc);

                    self.current_values.insert(identifier, value);
                } else if self.pop_if_next_symbol("not")? {
                    let lhs = self.parse_value()?;

                    let paused = builder.pause_building();
                    let loc = self.parse_loc(context)?;
                    builder = InstructionBuilder::resume_building(context, paused);

                    let value = builder.not(lhs, loc);

                    self.current_values.insert(identifier, value);
                } else if self.pop_if_next_symbol("shl")? {
                    let lhs = self.parse_value()?;

                    if !self.pop_if_next_symbol(",")? {
                        return Err(Diagnostic::new_error(
                            "Expected ',' between arguments to an instruction",
                            Label::new(self.file, self.single_char_span(), "here"),
                        ));
                    }

                    let rhs = self.parse_value()?;

                    let paused = builder.pause_building();
                    let loc = self.parse_loc(context)?;
                    builder = InstructionBuilder::resume_building(context, paused);

                    let value = builder.shl(lhs, rhs, loc);

                    self.current_values.insert(identifier, value);
                } else if self.pop_if_next_symbol("shr")? {
                    let lhs = self.parse_value()?;

                    if !self.pop_if_next_symbol(",")? {
                        return Err(Diagnostic::new_error(
                            "Expected ',' between arguments to an instruction",
                            Label::new(self.file, self.single_char_span(), "here"),
                        ));
                    }

                    let rhs = self.parse_value()?;

                    let paused = builder.pause_building();
                    let loc = self.parse_loc(context)?;
                    builder = InstructionBuilder::resume_building(context, paused);

                    let value = builder.shr(lhs, rhs, loc);

                    self.current_values.insert(identifier, value);
                } else if self.pop_if_next_symbol("cast")? {
                    let lhs = self.parse_value()?;

                    if !self.pop_if_next_symbol("to")? {
                        return Err(Diagnostic::new_error(
                            "Expected 'to' between argument and type",
                            Label::new(self.file, self.single_char_span(), "here"),
                        ));
                    }

                    let paused_builder = builder.pause_building();
                    let ty = self.parse_type(context)?;
                    builder = InstructionBuilder::resume_building(context, paused_builder);

                    let paused = builder.pause_building();
                    let loc = self.parse_loc(context)?;
                    builder = InstructionBuilder::resume_building(context, paused);

                    let value = builder.cast(lhs, ty, loc);

                    self.current_values.insert(identifier, value);
                } else if self.pop_if_next_symbol("bitcast")? {
                    let lhs = self.parse_value()?;

                    if !self.pop_if_next_symbol("to")? {
                        return Err(Diagnostic::new_error(
                            "Expected 'to' between argument and type",
                            Label::new(self.file, self.single_char_span(), "here"),
                        ));
                    }

                    let paused_builder = builder.pause_building();
                    let ty = self.parse_type(context)?;
                    builder = InstructionBuilder::resume_building(context, paused_builder);

                    let paused = builder.pause_building();
                    let loc = self.parse_loc(context)?;
                    builder = InstructionBuilder::resume_building(context, paused);

                    let value = builder.bitcast(lhs, ty, loc);

                    self.current_values.insert(identifier, value);
                } else if self.pop_if_next_symbol("load")? {
                    let paused_builder = builder.pause_building();
                    let ty = self.parse_type(context)?;
                    builder = InstructionBuilder::resume_building(context, paused_builder);

                    if !self.pop_if_next_symbol(",")? {
                        return Err(Diagnostic::new_error(
                            "Expected ',' between arguments to an instruction",
                            Label::new(self.file, self.single_char_span(), "here"),
                        ));
                    }

                    let ptr = self.parse_value()?;

                    let paused = builder.pause_building();
                    let loc = self.parse_loc(context)?;
                    builder = InstructionBuilder::resume_building(context, paused);

                    let value = builder.load(ty, ptr, loc);

                    self.current_values.insert(identifier, value);
                } else if self.pop_if_next_symbol("stackalloc")? {
                    self.skip_comments_or_whitespace();

                    let name = self.parse_identifier()?;

                    if !self.pop_if_next_symbol(",")? {
                        return Err(Diagnostic::new_error(
                            "Expected ',' between arguments to an instruction",
                            Label::new(self.file, self.single_char_span(), "here"),
                        ));
                    }

                    let paused_builder = builder.pause_building();
                    let ty = self.parse_type(context)?;
                    let loc = self.parse_loc(context)?;
                    builder = InstructionBuilder::resume_building(context, paused_builder);

                    let value = builder.stack_alloc(name, ty, loc);

                    self.current_values.insert(identifier, value);
                } else if self.pop_if_next_symbol("call")? {
                    let name = self.parse_identifier()?;

                    let module = if self.pop_if_next_symbol("from")? {
                        let module_name = self.parse_identifier()?;

                        if !self.modules.contains_key(&module_name) {
                            return Err(Diagnostic::new_error(
                                "Unknown module",
                                Label::new(self.file, self.single_char_span(), "here"),
                            ));
                        }

                        *self.modules.get(module_name).unwrap()
                    } else {
                        self.current_module.unwrap()
                    };

                    let key = (name, module);

                    if !self.functions.contains_key(&key) {
                        return Err(Diagnostic::new_error(
                            "Unknown function called",
                            Label::new(self.file, self.single_char_span(), "here"),
                        ));
                    }

                    let function = *self.functions.get(&key).unwrap();

                    if !self.pop_if_next_symbol("(")? {
                        return Err(Diagnostic::new_error(
                            "Expected ',' between arguments to a function call",
                            Label::new(self.file, self.single_char_span(), "here"),
                        ));
                    }

                    let mut args = Vec::new();

                    loop {
                        self.skip_comments_or_whitespace();

                        if self.peek_if_next_symbol(")") {
                            self.bump_current();
                            break;
                        }

                        let value = self.parse_value()?;
                        args.push(value);

                        self.skip_comments_or_whitespace();

                        if self.peek_if_next_symbol(",") {
                            self.bump_current();
                            continue;
                        }
                    }

                    let paused = builder.pause_building();
                    let loc = self.parse_loc(context)?;
                    builder = InstructionBuilder::resume_building(context, paused);

                    let value = builder.call(function, &args, loc);

                    self.current_values.insert(identifier, value);
                } else if self.pop_if_next_symbol("select")? {
                    let cond = self.parse_value()?;

                    if !self.pop_if_next_symbol(",")? {
                        return Err(Diagnostic::new_error(
                            "Expected ',' between arguments to an instruction",
                            Label::new(self.file, self.single_char_span(), "here"),
                        ));
                    }

                    let lhs = self.parse_value()?;

                    if !self.pop_if_next_symbol(",")? {
                        return Err(Diagnostic::new_error(
                            "Expected ',' between arguments to an instruction",
                            Label::new(self.file, self.single_char_span(), "here"),
                        ));
                    }

                    let rhs = self.parse_value()?;

                    let paused = builder.pause_building();
                    let loc = self.parse_loc(context)?;
                    builder = InstructionBuilder::resume_building(context, paused);

                    let value = builder.select(cond, lhs, rhs, loc);

                    self.current_values.insert(identifier, value);
                } else if self.pop_if_next_symbol("indexinto")? {
                    let paused_builder = builder.pause_building();
                    let ty = self.parse_type(context)?;
                    builder = InstructionBuilder::resume_building(context, paused_builder);

                    if !self.pop_if_next_symbol(",")? {
                        return Err(Diagnostic::new_error(
                            "Expected ',' between arguments to an instruction",
                            Label::new(self.file, self.single_char_span(), "here"),
                        ));
                    }

                    let ptr = self.parse_value()?;

                    if !self.pop_if_next_symbol(",")? {
                        return Err(Diagnostic::new_error(
                            "Expected ',' between arguments to an instruction",
                            Label::new(self.file, self.single_char_span(), "here"),
                        ));
                    }

                    let mut indices = Vec::new();

                    loop {
                        indices.push(self.parse_value()?);

                        if !self.pop_if_next_symbol(",")? {
                            break;
                        }
                    }

                    let paused = builder.pause_building();
                    let loc = self.parse_loc(context)?;
                    builder = InstructionBuilder::resume_building(context, paused);

                    let value = builder.index_into(ty, ptr, &indices, loc);

                    self.current_values.insert(identifier, value);
                } else if self.pop_if_next_symbol("const")? {
                    let paused = builder.pause_building();
                    let ty = self.parse_type(context)?;

                    let value = self.parse_constant(context, ty)?;

                    builder = InstructionBuilder::resume_building(context, paused);

                    self.current_values.insert(identifier, value);
                }
            }
        }

        // Wipe the current values as they are block-local.
        self.current_values.clear();

        Ok(())
    }

    fn parse_fn_body(&mut self, context: &mut Context) -> Result<(), Diagnostic> {
        // Skip the '{'
        self.bump_current();

        loop {
            self.parse_block(context)?;

            self.skip_comments_or_whitespace();

            if self.get_current_str().starts_with('}') {
                break;
            }
        }

        // SKip the '}'
        self.bump_current();

        // Wipe the blocks as they are function-local.
        self.current_blocks.clear();

        Ok(())
    }

    fn parse_arg(&mut self, context: &mut Context) -> Result<(&'a str, Type), Diagnostic> {
        let name = self.parse_identifier()?;

        if !self.pop_if_next_symbol(":")? {
            return Err(Diagnostic::new_error(
                "Expected ':' to declare an argument's type",
                Label::new(self.file, self.single_char_span(), "missing ':'"),
            ));
        }

        let ty = self.parse_type(context)?;

        if !self.peek_if_next_symbol(")") && !self.pop_if_next_symbol(",")? {
            return Err(Diagnostic::new_error(
                "Expected ',' between arguments",
                Label::new(self.file, self.single_char_span(), "should be ','"),
            ));
        }

        Ok((name, ty))
    }

    fn parse_loc(&mut self, context: &mut Context) -> Result<Option<Location>, Diagnostic> {
        self.skip_comments_or_whitespace();

        // If we don't have the '!' that starts a location, bail!
        if !self.pop_if_next_symbol("!")? {
            return Ok(None);
        }

        let loc = self.parse_quoted_identifier()?;

        if !self.pop_if_next_symbol(":")? {
            return Err(Diagnostic::new_error(
                "Expected ':' between file and line of location",
                Label::new(self.file, self.single_char_span(), "should be ':'"),
            ));
        }

        let line = self.parse_literal()?;

        if !self.pop_if_next_symbol(":")? {
            return Err(Diagnostic::new_error(
                "Expected ':' between line and column of location",
                Label::new(self.file, self.single_char_span(), "should be ':'"),
            ));
        }

        let column = self.parse_literal()?;

        Ok(Some(context.get_location(loc, line, column)))
    }

    fn parse_fn(&mut self, context: &mut Context) -> Result<(), Diagnostic> {
        debug_assert!(self.get_current_str().starts_with("fn"));

        self.bump_current_by("fn".len());

        self.skip_comments_or_whitespace();

        let mut attributes = FunctionAttributes::default();

        if self.pop_if_next_symbol("[")? {
            loop {
                self.skip_comments_or_whitespace();

                if self.pop_if_next_symbol("export")? {
                    attributes |= FunctionAttribute::Export;
                } else if self.pop_if_next_symbol("job")? {
                    attributes |= FunctionAttribute::Job;
                } else {
                    return Err(Diagnostic::new_error(
                        "Unknown function attribute",
                        Label::new(self.file, self.single_char_span(), "here"),
                    ));
                }

                self.skip_comments_or_whitespace();

                if self.pop_if_next_symbol("]")? {
                    self.skip_comments_or_whitespace();
                    break;
                } else if self.pop_if_next_symbol(",")? {
                    continue;
                } else {
                    return Err(Diagnostic::new_error(
                        "Expected ']' or ',' in function attributes",
                        Label::new(self.file, self.single_char_span(), "missing '(' or ','"),
                    ));
                }
            }
        }

        let name = self.parse_identifier()?;

        self.skip_comments_or_whitespace();

        if !self.pop_if_next_symbol("(")? {
            return Err(Diagnostic::new_error(
                "Expected '(' to open a functions arguments",
                Label::new(self.file, self.single_char_span(), "missing '('"),
            ));
        }

        let mut args = Vec::new();

        loop {
            if self.pop_if_next_symbol(")")? {
                self.skip_comments_or_whitespace();
                break;
            }

            args.push(self.parse_arg(context)?);
        }

        self.skip_comments_or_whitespace();

        if !self.get_current_str().starts_with(':') {
            return Err(Diagnostic::new_error(
                "Expected ':' to declare a functions return type",
                Label::new(self.file, self.single_char_span(), "missing ':'"),
            ));
        }

        // Skip the ':'
        self.bump_current();

        self.skip_comments_or_whitespace();

        let return_type = self.parse_type(context)?;

        let module = self.current_module.unwrap();

        let location = self.parse_loc(context)?;

        let mut builder = module
            .create_function(context)
            .with_name(name)
            .with_attributes(attributes)
            .with_return_type(return_type);

        if let Some(location) = location {
            builder = builder.with_location(location);
        }

        for (name, ty) in args {
            builder = builder.with_arg(name, ty);
        }

        let function = builder.build();

        self.functions.insert((name, module), function);

        self.current_function = Some(function);

        self.skip_comments_or_whitespace();

        if self.get_current_str().starts_with('{') {
            self.parse_fn_body(context)?;
        }

        self.current_function = None;

        Ok(())
    }

    fn parse_var(&mut self, context: &mut Context) -> Result<(), Diagnostic> {
        assert!(self.get_current_str().starts_with("var"));

        self.bump_current_by("var".len());

        self.skip_comments_or_whitespace();

        let mut attributes = GlobalAttributes::default();

        if self.pop_if_next_symbol("[")? {
            loop {
                self.skip_comments_or_whitespace();

                if self.pop_if_next_symbol("export")? {
                    attributes |= GlobalAttribute::Export;
                } else {
                    return Err(Diagnostic::new_error(
                        "Unknown variable attribute",
                        Label::new(self.file, self.single_char_span(), "here"),
                    ));
                }

                self.skip_comments_or_whitespace();

                if self.pop_if_next_symbol("]")? {
                    self.skip_comments_or_whitespace();
                    break;
                } else if self.pop_if_next_symbol(",")? {
                    continue;
                } else {
                    return Err(Diagnostic::new_error(
                        "Expected ']' or ',' in variable attributes",
                        Label::new(self.file, self.single_char_span(), "missing '(' or ','"),
                    ));
                }
            }
        }

        let identifier = self.parse_identifier()?;

        self.skip_comments_or_whitespace();

        if !self.get_current_str().starts_with(':') {
            return Err(Diagnostic::new_error(
                "Expected ':' to declare a variables type",
                Label::new(self.file, self.single_char_span(), "missing ':'"),
            ));
        }

        // Skip the ':'
        self.bump_current();

        self.skip_comments_or_whitespace();

        let domain = self.parse_domain()?;

        if !self.get_current_str().starts_with(',') {
            return Err(Diagnostic::new_error(
                "Expected ',' between domain and type of a global",
                Label::new(self.file, self.single_char_span(), "here"),
            ));
        }

        // Skip the ','
        self.bump_current();

        let ty = self.parse_type(context)?;

        let module = self.current_module.unwrap();

        let location = self.parse_loc(context)?;

        let builder = module
            .create_global(context)
            .with_attributes(attributes)
            .with_name(identifier)
            .with_type(ty)
            .with_domain(domain);

        let var = if let Some(location) = location {
            builder.with_location(location)
        } else {
            builder
        }
        .build();

        self.variables.insert(identifier, var);

        Ok(())
    }

    fn parse_struct(&mut self, context: &mut Context) -> Result<(), Diagnostic> {
        let identifier = self.parse_identifier()?;

        if !self.pop_if_next_symbol(":")? {
            return Err(Diagnostic::new_error(
                "Expected ':' to declare a struct type",
                Label::new(self.file, self.single_char_span(), "missing ':'"),
            ));
        }

        if !self.pop_if_next_symbol("{")? {
            return Err(Diagnostic::new_error(
                "Expected '{' to open a declaration of a struct type",
                Label::new(self.file, self.single_char_span(), "missing '{'"),
            ));
        }

        let mut elements = Vec::new();

        loop {
            let element_name = self.parse_identifier()?;

            if !self.pop_if_next_symbol(":")? {
                return Err(Diagnostic::new_error(
                    "Expected ':' for a structs element type",
                    Label::new(self.file, self.single_char_span(), "missing ':'"),
                ));
            }

            let element_type = self.parse_type(context)?;

            let location = self.parse_loc(context)?;

            elements.push((element_name, element_type, location));

            if self.pop_if_next_symbol(",")? {
                continue;
            } else if self.pop_if_next_symbol("}")? {
                break;
            } else {
                return Err(Diagnostic::new_error(
                    "Expected ',' or '}' for a struct type declaration",
                    Label::new(self.file, self.single_char_span(), "missing ',' or '}'"),
                ));
            }
        }

        let location = self.parse_loc(context)?;

        let named_struct = self
            .current_module
            .unwrap()
            .create_named_struct_type(context, identifier, &elements, location);

        self.structs.insert(identifier, named_struct);

        Ok(())
    }

    fn parse_fn_or_var(&mut self, context: &mut Context) -> Result<(), Diagnostic> {
        if self.get_current_str().starts_with("fn") {
            self.parse_fn(context)
        } else if self.get_current_str().starts_with("var") {
            self.parse_var(context)
        } else if self.pop_if_next_symbol("struct")? {
            self.parse_struct(context)
        } else if self.get_current_str().starts_with('}') || self.get_current_str().is_empty() {
            Ok(())
        } else {
            Err(Diagnostic::new_error(
                "Unknown declaration within module",
                Label::new(
                    self.file,
                    self.single_char_span(),
                    "expected fn, var, struct, or '}' to close the module",
                ),
            ))
        }
    }

    fn parse_mod(&mut self, context: &mut Context) -> Result<(), Diagnostic> {
        // Skip the leading mod
        self.bump_current_by("mod".len());

        self.skip_comments_or_whitespace();

        let name = self.parse_identifier()?;

        self.skip_comments_or_whitespace();

        if !self.get_current_str().starts_with('{') {
            return Err(Diagnostic::new_error(
                "Expected '{' to open a module",
                Label::new(self.file, self.single_char_span(), "should be '{'"),
            ));
        }

        let module = context.create_module().with_name(name).build();
        self.modules.insert(name, module);

        // Record our current module so that stuff in the module know where they live.
        self.current_module = Some(module);

        // Skip the '{'.
        self.bump_current();

        loop {
            self.skip_comments_or_whitespace();

            if self.get_current_str().is_empty() {
                return Err(Diagnostic::new_error(
                    "Expected '}' to close a module",
                    Label::new(self.file, self.single_char_span(), "should be '}'"),
                ));
            }

            if self.get_current_str().starts_with('}') {
                // Skip the '}'.
                self.bump_current();

                // And we're done parsing what is in the module.
                break;
            }

            self.parse_fn_or_var(context)?;
        }

        // And reset the current module when exiting.
        self.current_module = None;
        self.structs.clear();
        self.variables.clear();

        Ok(())
    }

    pub fn build(mut self, context: &mut Context) -> Result<(), Diagnostic> {
        loop {
            self.skip_comments_or_whitespace();

            if self.get_current_str().is_empty() {
                return Ok(());
            }

            if self.get_current_str().starts_with("mod") {
                match self.parse_mod(context) {
                    Ok(_) => (),
                    Err(e) => return Err(e),
                }
            } else {
                return Err(Diagnostic::new_error(
                    "Unknown symbol in input",
                    Label::new(self.file, self.single_char_span(), "here"),
                ));
            }
        }
    }
}

pub fn assemble(file: FileId, data: &str) -> Result<Context, Diagnostic> {
    let mut context = Context::new();
    let assembler = Assembler::new(file, &data);

    match assembler.build(&mut context) {
        Ok(_) => Ok(context),
        Err(d) => Err(d),
    }
}

fn get_domain(domain: Domain) -> &'static str {
    match domain {
        Domain::CrossDevice => "any",
        Domain::Cpu => "cpu",
        Domain::Gpu => "gpu",
        Domain::Stack => "stack",
    }
}

fn get_type_name(context: &Context, ty: Type) -> String {
    if ty.is_named_struct(context) {
        format!("%{}", ty.get_name(context).get_displayer(context))
    } else if ty.is_void(context) {
        "void".to_string()
    } else if ty.is_boolean(context) {
        "bool".to_string()
    } else if ty.is_vector(context) {
        format!(
            "<{}, {}>",
            get_type_name(context, ty.get_element(context, 0)),
            ty.get_len(context)
        )
    } else if ty.is_array(context) {
        format!(
            "[{}, {}]",
            get_type_name(context, ty.get_element(context, 0)),
            ty.get_len(context)
        )
    } else if ty.is_struct(context) {
        let mut string = "{".to_string();

        for i in 0..ty.get_len(context) {
            if i != 0 {
                string.push_str(", ");
            }

            string.push_str(&get_type_name(context, ty.get_element(context, i)));
        }

        string.push('}');

        string
    } else if ty.is_int(context) {
        format!("i{}", ty.get_bits(context))
    } else if ty.is_uint(context) {
        format!("u{}", ty.get_bits(context))
    } else if ty.is_float(context) {
        format!("f{}", ty.get_bits(context))
    } else if ty.is_pointer(context) {
        format!("*{}", get_domain(ty.get_domain(context)))
    } else {
        std::unreachable!();
    }
}

fn get_loc(context: &Context, loc: &Option<Location>) -> String {
    if let Some(loc) = loc {
        format!("{}", loc.get_displayer(context))
    } else {
        "".to_string()
    }
}

fn get_constant_literal(context: &Context, val: &Value) -> String {
    let cnst = val.get_constant(context);

    match cnst {
        Constant::Bool(b, _) => b.to_string(),
        Constant::Int(i, _) => i.to_string(),
        Constant::UInt(u, _) => u.to_string(),
        Constant::Float(f, _) => format!("{:e}", f),
        Constant::Pointer(_) => "null".to_string(),
        Constant::Composite(c, ty) => {
            let (open, close) = if ty.is_array(context) {
                ('[', ']')
            } else if ty.is_vector(context) {
                ('<', '>')
            } else if ty.is_struct(context) {
                ('{', '}')
            } else {
                std::unreachable!();
            };

            let mut literal = open.to_string();

            for e in c {
                literal += &get_constant_literal(context, e);
                literal += ", ";
            }

            literal.pop();
            literal.pop();
            literal.push(close);

            literal
        }
    }
}

fn write_if_constant(
    context: &Context,
    value: &Value,
    writer: &mut impl std::io::Write,
) -> std::io::Result<()> {
    if value.is_constant(context) {
        writeln!(
            writer,
            "      {} = const {} {}",
            value.get_displayer(context),
            value.get_type(context).get_displayer(context),
            get_constant_literal(context, &value)
        )?;
    }

    Ok(())
}

pub fn disassemble(context: &Context, mut writer: impl std::io::Write) -> std::io::Result<()> {
    let modules = context.get_modules();

    for module in modules {
        let name = module.get_name(context);

        write!(writer, "mod {} {{", name.get_displayer(context))?;

        let mut printed_newline = false;

        for named_struct in module.get_named_structs(context) {
            if !printed_newline {
                writeln!(writer)?;
                printed_newline = true;
            }

            write!(writer, "  struct ")?;

            let name = named_struct.get_name(context).get_displayer(context);

            write!(writer, "{} : {{", name,)?;

            let len = named_struct.get_len(context);

            for i in 0..named_struct.get_len(context) {
                let name = named_struct.get_element_name(context, i);
                let ty = named_struct.get_element(context, i);
                let location = named_struct.get_location(context);

                write!(
                    writer,
                    "{} : {}{}",
                    name.get_displayer(context),
                    get_type_name(context, ty),
                    get_loc(context, &location)
                )?;

                if i != (len - 1) {
                    write!(writer, ", ")?;
                }
            }

            writeln!(writer, "}}")?;
        }

        let mut printed_newline = false;

        for global in module.get_globals(context) {
            if !printed_newline {
                writeln!(writer)?;
                printed_newline = true;
            }

            write!(writer, "  var ")?;

            if global.is_export(context) {
                write!(writer, "[export] ")?;
            }

            let name = global.get_name(context).get_displayer(context);
            let domain = get_domain(global.get_global_domain(context));
            let ty = global.get_global_backing_type(context);
            let ty_name = get_type_name(context, ty);
            let location = global.get_location(context);

            writeln!(
                writer,
                "{} : {}, {}{}",
                name,
                domain,
                ty_name,
                get_loc(context, &location)
            )?;
        }

        for function in module.get_functions(context) {
            if !printed_newline {
                writeln!(writer)?;
                printed_newline = true;
            }

            write!(writer, "  {}", function.get_displayer(context))?;

            let mut first = true;

            for block in function.get_blocks(context) {
                if first {
                    writeln!(writer, " {{")?;
                    first = false;
                }

                writeln!(writer, "    {}:", block.get_displayer(context))?;

                for value in block.get_insts(context) {
                    match value.get_inst(context) {
                        Instruction::ReturnValue(_, r, _) => {
                            write_if_constant(context, r, &mut writer)?;
                        }
                        Instruction::Cmp(_, _, a, b, _) => {
                            write_if_constant(context, a, &mut writer)?;
                            write_if_constant(context, b, &mut writer)?;
                        }
                        Instruction::Unary(_, _, a, _) => {
                            write_if_constant(context, a, &mut writer)?;
                        }
                        Instruction::Binary(_, _, a, b, _) => {
                            write_if_constant(context, a, &mut writer)?;
                            write_if_constant(context, b, &mut writer)?;
                        }
                        Instruction::Cast(_, val, _) => {
                            write_if_constant(context, val, &mut writer)?;
                        }
                        Instruction::BitCast(_, val, _) => {
                            write_if_constant(context, val, &mut writer)?;
                        }
                        Instruction::Load(_, ptr, _) => {
                            write_if_constant(context, ptr, &mut writer)?;
                        }
                        Instruction::Store(_, ptr, val, _) => {
                            write_if_constant(context, ptr, &mut writer)?;
                            write_if_constant(context, val, &mut writer)?;
                        }
                        Instruction::Extract(agg, _, _) => {
                            write_if_constant(context, agg, &mut writer)?;
                        }
                        Instruction::Insert(agg, elem, _, _) => {
                            write_if_constant(context, agg, &mut writer)?;
                            write_if_constant(context, elem, &mut writer)?;
                        }
                        Instruction::Call(_, args, _) => {
                            for arg in args {
                                write_if_constant(context, arg, &mut writer)?;
                            }
                        }
                        Instruction::Branch(_, args, _) => {
                            for arg in args {
                                write_if_constant(context, arg, &mut writer)?;
                            }
                        }
                        Instruction::ConditionalBranch(cond, _, _, true_args, false_args, _) => {
                            write_if_constant(context, cond, &mut writer)?;

                            for arg in true_args {
                                write_if_constant(context, arg, &mut writer)?;
                            }

                            for arg in false_args {
                                write_if_constant(context, arg, &mut writer)?;
                            }
                        }
                        Instruction::Select(_, cond, true_val, false_val, _) => {
                            write_if_constant(context, cond, &mut writer)?;
                            write_if_constant(context, true_val, &mut writer)?;
                            write_if_constant(context, false_val, &mut writer)?;
                        }
                        Instruction::IndexInto(_, ptr, args, _) => {
                            write_if_constant(context, ptr, &mut writer)?;

                            for arg in args {
                                write_if_constant(context, arg, &mut writer)?;
                            }
                        }
                        _ => (),
                    }

                    writeln!(writer, "      {}", value.get_inst_displayer(context))?;
                }
            }

            // If we found at least one block, the function had a body
            if !first {
                writer.write_fmt(format_args!("  }}\n"))?;
            } else {
                writer.write_fmt(format_args!("\n"))?;
            }
        }

        writer.write_fmt(format_args!("}}\n"))?;
    }

    Ok(())
}
