extern crate codespan;
extern crate rmp_serde;
extern crate serde;
extern crate yair;

use codespan::{FileId, Span};
use codespan_reporting::diagnostic::{Diagnostic, Label};
use rmp_serde::Serializer;
use serde::Serialize;
use std::collections::HashMap;
use std::str::FromStr;
use yair::*;

struct Assembler<'a> {
    data: &'a str,
    offset: u32,
    file: FileId,
    modules: HashMap<&'a str, Module>,
    functions: HashMap<(&'a str, Module), Function>,
    variables: HashMap<(&'a str, Module), Value>,
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
            current_blocks: HashMap::new(),
            current_values: HashMap::new(),
            current_module: None,
            current_function: None,
        }
    }

    fn get_current_char(&self) -> Option<char> {
        self.get_current_str().chars().nth(0)
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

        if self.get_current_str().len() == 0 {
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
                        "Expected string identifier for module name",
                        Label::new(self.file, self.single_char_span(), "missing identifier"),
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

        if self.get_current_str().len() == 0 {
            return Err(self.unexpected_end_of_file());
        }

        let start = self.offset;

        let identifier = if self.peek_if_next_symbol("\"") {
            self.parse_quoted_identifier()?
        } else {
            self.parse_unquoted_identifier()
        };

        let end = self.offset;

        if identifier.is_empty() {
            Err(Diagnostic::new_error(
                "Expected string identifier for module name",
                Label::new(self.file, self.single_char_span(), "missing identifier"),
            ))
        } else {
            Ok((identifier, self.make_span(start, end)))
        }
    }

    fn parse_identifier(&mut self) -> Result<&'a str, Diagnostic> {
        Ok(self.parse_identifier_with_span()?.0)
    }

    fn try_parse_int_or_float_val(
        &mut self,
        library: &mut Library,
        c: char,
        bits: u8,
    ) -> Option<Type> {
        let str = c.to_string() + &bits.to_string();

        if !self.get_current_str().starts_with(&str) {
            return None;
        }

        self.bump_current_by(str.len());

        match c {
            'i' => Some(library.get_int_ty(bits)),
            'u' => Some(library.get_uint_ty(bits)),
            'f' => Some(library.get_float_ty(bits)),
            _ => panic!("Unhandled integer prefix"),
        }
    }

    fn try_parse_int_or_float(&mut self, library: &mut Library, c: char) -> Option<Type> {
        match self.try_parse_int_or_float_val(library, c, 8) {
            Some(x) => return Some(x),
            None => (),
        }

        match self.try_parse_int_or_float_val(library, c, 16) {
            Some(x) => return Some(x),
            None => (),
        }

        match self.try_parse_int_or_float_val(library, c, 32) {
            Some(x) => return Some(x),
            None => (),
        }

        match self.try_parse_int_or_float_val(library, c, 64) {
            Some(x) => return Some(x),
            None => (),
        }

        None
    }

    fn parse_struct_type(&mut self, library: &mut Library) -> Result<Type, Diagnostic> {
        // Skip the '{'
        self.bump_current();

        let mut element_types = Vec::new();

        loop {
            self.skip_comments_or_whitespace();

            if self.get_current_str().len() == 0 {
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

            let element_type = self.parse_type(library)?;

            element_types.push(element_type);

            self.skip_comments_or_whitespace();

            if self.get_current_str().starts_with('}') {
                // Skip the '}'
                self.bump_current();

                break;
            }

            if !self.get_current_str().starts_with(",") {
                return Err(Diagnostic::new_error(
                    "Expected ',' between elements of a struct",
                    Label::new(self.file, self.single_char_span(), "should be ','"),
                ));
            }

            // Skip the ','
            self.bump_current();
        }

        Ok(library.get_struct_ty(&element_types))
    }

    fn parse_literal<T: FromStr>(&mut self) -> Result<T, Diagnostic> {
        self.skip_comments_or_whitespace();

        let mut str = self.get_current_str();

        let start = str.len();

        loop {
            if !str.starts_with(char::is_numeric) {
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
            Err(e) => Err(Diagnostic::new_error(
                "Literal did not parse as the correct type",
                Label::new(self.file, self.single_char_span(), "here"),
            )),
        }
    }

    fn parse_vector_type(&mut self, library: &mut Library) -> Result<Type, Diagnostic> {
        // Skip the '<'
        self.bump_current();

        self.skip_comments_or_whitespace();

        let element_type = self.parse_type(library)?;

        self.skip_comments_or_whitespace();

        if !self.get_current_str().starts_with(",") {
            return Err(Diagnostic::new_error(
                "Vector type was malformed",
                Label::new(self.file, self.single_char_span(), "missing ','"),
            ));
        }

        // Skip the ','
        self.bump_current();

        let width = self.parse_literal()?;

        if !self.get_current_str().starts_with(">") {
            return Err(Diagnostic::new_error(
                "Vector type was malformed",
                Label::new(self.file, self.single_char_span(), "missing '>'"),
            ));
        }

        // Skip the '>'
        self.bump_current();

        Ok(library.get_vec_type(element_type, width))
    }

    fn parse_array_type(&mut self, library: &mut Library) -> Result<Type, Diagnostic> {
        // Skip the '['
        self.bump_current();

        self.skip_comments_or_whitespace();

        let element_type = self.parse_type(library)?;

        self.skip_comments_or_whitespace();

        if !self.get_current_str().starts_with(",") {
            return Err(Diagnostic::new_error(
                "Array type was malformed",
                Label::new(self.file, self.single_char_span(), "missing ','"),
            ));
        }

        // Skip the ','
        self.bump_current();

        let len = self.parse_literal()?;

        if !self.get_current_str().starts_with("]") {
            return Err(Diagnostic::new_error(
                "Array type was malformed",
                Label::new(self.file, self.single_char_span(), "missing ']'"),
            ));
        }

        // Skip the ']'
        self.bump_current();

        Ok(library.get_array_ty(element_type, len))
    }

    fn parse_pointer_type(&mut self, library: &mut Library) -> Result<Type, Diagnostic> {
        // Skiop the '*'
        self.bump_current();

        self.skip_comments_or_whitespace();

        // If we have the optional domain.
        let domain = if self.get_current_str().starts_with('(') {
            // Skip the '('
            self.bump_current();

            self.skip_comments_or_whitespace();

            let detected_domain = if self.get_current_str().starts_with("any") {
                self.bump_current_by("any".len());
                Domain::CrossDevice
            } else if self.get_current_str().starts_with("cpu") {
                self.bump_current_by("cpu".len());
                Domain::CPU
            } else if self.get_current_str().starts_with("gpu") {
                self.bump_current_by("gpu".len());
                Domain::GPU
            } else if self.get_current_str().starts_with("stack") {
                self.bump_current_by("stack".len());
                Domain::STACK
            } else {
                return Err(Diagnostic::new_error(
                    "Invalid pointer domain - expected any, cpu, gpu, or stack",
                    Label::new(self.file, self.single_char_span(), "unknown domain"),
                ));
            };

            self.skip_comments_or_whitespace();

            if !self.get_current_str().starts_with(')') {
                return Err(Diagnostic::new_error(
                    "Invalid pointer domain",
                    Label::new(self.file, self.single_char_span(), "expected ')'"),
                ));
            }

            // Skip ')'
            self.bump_current();

            self.skip_comments_or_whitespace();

            detected_domain
        } else {
            Domain::CrossDevice
        };

        let pointee_type = self.parse_type(library)?;

        Ok(library.get_ptr_type(pointee_type, domain))
    }

    fn parse_type(&mut self, library: &mut Library) -> Result<Type, Diagnostic> {
        self.skip_comments_or_whitespace();

        match self.try_parse_int_or_float(library, 'i') {
            Some(t) => return Ok(t),
            None => (),
        }

        match self.try_parse_int_or_float(library, 'u') {
            Some(t) => return Ok(t),
            None => (),
        }

        match self.try_parse_int_or_float(library, 'f') {
            Some(t) => return Ok(t),
            None => (),
        }

        if self.get_current_str().starts_with("void") {
            self.bump_current_by("void".len());
            Ok(library.get_void_ty())
        } else if self.get_current_str().starts_with("bool") {
            self.bump_current_by("bool".len());
            Ok(library.get_bool_ty())
        } else if self.get_current_str().starts_with("<") {
            self.parse_vector_type(library)
        } else if self.get_current_str().starts_with("{") {
            self.parse_struct_type(library)
        } else if self.get_current_str().starts_with("[") {
            self.parse_array_type(library)
        } else if self.get_current_str().starts_with("*") {
            self.parse_pointer_type(library)
        } else {
            Err(Diagnostic::new_error(
                "Could not deduce type",
                Label::new(self.file, self.single_char_span(), "unknown type"),
            ))
        }
    }

    fn parse_value(&mut self) -> Result<Value, Diagnostic> {
        let (name, span) = self.parse_identifier_with_span()?;

        match self.current_values.get(name) {
            Some(v) => Ok(*v),
            None => Err(Diagnostic::new_error(
                "Unknown identified value",
                Label::new(self.file, span, "no match for this name"),
            )),
        }
    }

    fn parse_block(&mut self, library: &mut Library) -> Result<(), Diagnostic> {
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
            args.push(self.parse_arg(library)?);
        }

        if !self.pop_if_next_symbol(":")? {
            return Err(Diagnostic::new_error(
                "Expected ':' between block definition and its instruction",
                Label::new(self.file, self.single_char_span(), "here"),
            ));
        }

        let mut builder = function.create_block(library);

        for (_, ty) in &args {
            builder = builder.with_argument(*ty);
        }

        let block = builder.build();

        self.current_blocks.insert(name, block);

        for i in 0..block.get_num_args(library) {
            self.current_values
                .insert(args[i].0, block.get_arg(library, i));
        }

        let func_ret_is_void = self
            .current_function
            .unwrap()
            .get_return_type(library)
            .is_void(library);
        let mut builder = block.create_instructions(library);

        loop {
            if self.pop_if_next_symbol("ret")? {
                if func_ret_is_void {
                    builder.ret(None);
                } else {
                    builder.ret_val(self.parse_value()?, None);
                }

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

                    let value = builder.extract(aggregate, index, None);

                    self.current_values.insert(identifier, value);
                }
            }
        }

        // Wipe the current values as they are block-local.
        self.current_values.clear();

        Ok(())
    }

    fn parse_fn_body(&mut self, library: &mut Library) -> Result<(), Diagnostic> {
        // Skip the '{'
        self.bump_current();

        loop {
            self.parse_block(library)?;

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

    fn parse_arg(&mut self, library: &mut Library) -> Result<(&'a str, Type), Diagnostic> {
        let name = self.parse_identifier()?;

        if !self.pop_if_next_symbol(":")? {
            return Err(Diagnostic::new_error(
                "Expected ':' to declare an argument's type",
                Label::new(self.file, self.single_char_span(), "missing ':'"),
            ));
        }

        let ty = self.parse_type(library)?;

        if !self.peek_if_next_symbol(")") {
            if !self.pop_if_next_symbol(",")? {
                return Err(Diagnostic::new_error(
                    "Expected ',' between arguments",
                    Label::new(self.file, self.single_char_span(), "should be ','"),
                ));
            }
        }

        Ok((name, ty))
    }

    fn parse_fn(&mut self, library: &mut Library, is_export: bool) -> Result<(), Diagnostic> {
        assert!(self.get_current_str().starts_with("fn"));

        self.bump_current_by("fn".len());

        self.skip_comments_or_whitespace();

        let name = self.parse_identifier()?;

        self.skip_comments_or_whitespace();

        if !self.get_current_str().starts_with('(') {
            return Err(Diagnostic::new_error(
                "Expected '(' to open a functions arguments",
                Label::new(self.file, self.single_char_span(), "missing '('"),
            ));
        }

        // Skip the '('
        self.bump_current();

        let mut args = Vec::new();

        loop {
            if self.pop_if_next_symbol(")")? {
                break;
            }

            args.push(self.parse_arg(library)?);
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

        let return_type = self.parse_type(library)?;

        let module = self.current_module.unwrap();

        let mut builder = module
            .create_function(library)
            .with_name(name)
            .with_export(is_export)
            .with_return_type(return_type);

        for (name, ty) in args {
            builder = builder.with_argument(name, ty);
        }

        let function = builder.build();

        self.functions.insert((name, module), function);

        self.current_function = Some(function);

        self.skip_comments_or_whitespace();

        if self.get_current_str().starts_with('{') {
            self.parse_fn_body(library)?;
        }

        self.current_function = None;

        Ok(())
    }

    fn parse_var(&mut self, library: &mut Library, is_export: bool) -> Result<(), Diagnostic> {
        assert!(self.get_current_str().starts_with("var"));

        self.bump_current_by("var".len());

        self.skip_comments_or_whitespace();

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

        let ty = self.parse_type(library)?;

        let module = self.current_module.unwrap();

        let var = module
            .create_global(library)
            .with_export(is_export)
            .with_name(identifier)
            .with_type(ty)
            .build();

        self.variables.insert((identifier, module), var);

        Ok(())
    }

    fn parse_fn_or_var(&mut self, library: &mut Library) -> Result<(), Diagnostic> {
        let is_export = self.get_current_str().starts_with("export");

        if is_export {
            self.bump_current_by("export".len());
            self.skip_comments_or_whitespace();
        }

        if self.get_current_str().starts_with("fn") {
            self.parse_fn(library, is_export)
        } else if self.get_current_str().starts_with("var") {
            self.parse_var(library, is_export)
        } else if self.get_current_str().starts_with('}') || self.get_current_str().is_empty() {
            Ok(())
        } else {
            Err(Diagnostic::new_error(
                "Unknown declaration within module",
                Label::new(
                    self.file,
                    self.single_char_span(),
                    "expected export, fn, var, or '}' to close the module",
                ),
            ))
        }
    }

    fn parse_mod(&mut self, library: &mut Library) -> Result<(), Diagnostic> {
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

        let module = library.create_module().with_name(name).build();
        self.modules.insert(name, module);

        // Record our current module so that stuff in the module know where they live.
        self.current_module = Some(module);

        // Skip the '{'.
        self.bump_current();

        loop {
            self.skip_comments_or_whitespace();

            if self.get_current_str().len() == 0 {
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

            self.parse_fn_or_var(library)?;
        }

        // And reset the current module when exiting.
        self.current_module = None;

        Ok(())
    }

    pub fn build(mut self, library: &mut Library) -> Result<(), Diagnostic> {
        loop {
            self.skip_comments_or_whitespace();

            if self.get_current_str().is_empty() {
                return Ok(());
            }

            if self.get_current_str().starts_with("mod") {
                match self.parse_mod(library) {
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

pub fn assemble(file: FileId, data: &str) -> Result<Library, Diagnostic> {
    let mut library = Library::new();
    let assembler = Assembler::new(file, &data);

    match assembler.build(&mut library) {
        Ok(_) => Ok(library),
        Err(d) => Err(d),
    }
}

pub fn disassemble(library: &Library, mut writer: impl std::io::Write) -> std::io::Result<()> {
    let modules = library.get_modules();

    for module in modules {
        let name = module.get_name(library);

        writer.write_fmt(format_args!("mod "))?;

        if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            writer.write_fmt(format_args!("{}", name))?;
        } else {
            writer.write_fmt(format_args!("\"{}\"", name))?;
        }

        writer.write_fmt(format_args!(" {{}}\n"))?;
    }

    Ok(())
}
