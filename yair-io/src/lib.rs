extern crate codespan;
extern crate rmp_serde;
extern crate serde;
extern crate yair;

use codespan::{FileId, Span};
use codespan_reporting::diagnostic::{Diagnostic, Label};
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
        if let Some(x) = self.try_parse_int_or_float_val(library, c, 8) {
            return Some(x);
        }

        if let Some(x) = self.try_parse_int_or_float_val(library, c, 16) {
            return Some(x);
        }

        if let Some(x) = self.try_parse_int_or_float_val(library, c, 32) {
            return Some(x);
        }

        if let Some(x) = self.try_parse_int_or_float_val(library, c, 64) {
            return Some(x);
        }

        None
    }

    fn parse_struct_type(&mut self, library: &mut Library) -> Result<Type, Diagnostic> {
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

            let element_type = self.parse_type(library)?;

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
            Err(_) => Err(Diagnostic::new_error(
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

        Ok(library.get_vec_type(element_type, width))
    }

    fn parse_array_type(&mut self, library: &mut Library) -> Result<Type, Diagnostic> {
        // Skip the '['
        self.bump_current();

        self.skip_comments_or_whitespace();

        let element_type = self.parse_type(library)?;

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

        if let Some(t) = self.try_parse_int_or_float(library, 'i') {
            return Ok(t);
        }

        if let Some(t) = self.try_parse_int_or_float(library, 'u') {
            return Ok(t);
        }

        if let Some(t) = self.try_parse_int_or_float(library, 'f') {
            return Ok(t);
        }

        if self.get_current_str().starts_with("void") {
            self.bump_current_by("void".len());
            Ok(library.get_void_ty())
        } else if self.get_current_str().starts_with("bool") {
            self.bump_current_by("bool".len());
            Ok(library.get_bool_ty())
        } else if self.get_current_str().starts_with('<') {
            self.parse_vector_type(library)
        } else if self.get_current_str().starts_with('{') {
            self.parse_struct_type(library)
        } else if self.get_current_str().starts_with('[') {
            self.parse_array_type(library)
        } else if self.get_current_str().starts_with('*') {
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

        for (i, arg) in args.iter().enumerate().take(block.get_num_args(library)) {
            self.current_values.insert(arg.0, block.get_arg(library, i));
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

                    let value = builder.insert(aggregate, value, index, None);

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

        if !self.peek_if_next_symbol(")") && !self.pop_if_next_symbol(",")? {
            return Err(Diagnostic::new_error(
                "Expected ',' between arguments",
                Label::new(self.file, self.single_char_span(), "should be ','"),
            ));
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

fn get_domain(domain: Domain) -> &'static str {
    match domain {
        Domain::CrossDevice => "any",
        Domain::CPU => "cpu",
        Domain::GPU => "gpu",
        Domain::STACK => "stack",
    }
}

fn get_type_name(library: &Library, ty: Type) -> String {
    if ty.is_void(library) {
        "void".to_string()
    } else if ty.is_boolean(library) {
        "bool".to_string()
    } else if ty.is_vector(library) {
        format!(
            "<{}, {}>",
            get_type_name(library, ty.get_element(library, 0)),
            ty.get_len(library)
        )
    } else if ty.is_array(library) {
        format!(
            "[{}, {}]",
            get_type_name(library, ty.get_element(library, 0)),
            ty.get_len(library)
        )
    } else if ty.is_struct(library) {
        let mut string = "{".to_string();

        for i in 0..ty.get_len(library) {
            if i != 0 {
                string.push_str(", ");
            }

            string.push_str(&get_type_name(library, ty.get_element(library, i)));
        }

        string.push_str("}");

        string
    } else if ty.is_int(library) {
        format!("i{}", ty.get_bits(library))
    } else if ty.is_uint(library) {
        format!("u{}", ty.get_bits(library))
    } else if ty.is_float(library) {
        format!("f{}", ty.get_bits(library))
    } else if ty.is_ptr(library) {
        format!(
            "*({}) {}",
            get_domain(ty.get_domain(library)),
            get_type_name(library, ty.get_pointee(library))
        )
    } else {
        panic!("Unknown type");
    }
}

fn get_identifier(string: &str) -> String {
    if string
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        string.to_string()
    } else {
        format!("\"{}\"", string)
    }
}

fn get_loc(library: &Library, loc: &Option<Location>) -> String {
    if loc.is_none() {
        return "".to_string();
    }

    format!(
        " !\"{}\":{:?}:{:?}",
        loc.unwrap().get_name(library),
        loc.unwrap().get_start(),
        loc.unwrap().get_end()
    )
}

pub fn disassemble(library: &Library, mut writer: impl std::io::Write) -> std::io::Result<()> {
    let modules = library.get_modules();

    for module in modules {
        let name = module.get_name(library);

        writer.write_fmt(format_args!("mod "))?;

        writer.write_fmt(format_args!("{}", get_identifier(name)))?;

        writer.write_fmt(format_args!(" {{"))?;

        let mut printed_newline = false;

        for global in module.get_globals(library) {
            if !printed_newline {
                writer.write_fmt(format_args!("\n"))?;
                printed_newline = true;
            }

            let export = if global.is_export(library) {
                "  export "
            } else {
                "  "
            };
            let name = get_identifier(global.get_name(library));
            let ty = global.get_type(library).get_pointee(library);
            let ty_name = get_type_name(library, ty);

            writer.write_fmt(format_args!("{}var {} : {}\n", export, name, ty_name))?;
        }

        for function in module.get_functions(library) {
            if !printed_newline {
                writer.write_fmt(format_args!("\n"))?;
                printed_newline = true;
            }

            let export = if function.is_export(library) {
                "  export "
            } else {
                "  "
            };

            let name = get_identifier(function.get_name(library));

            writer.write_fmt(format_args!("{}fn {}(", export, name))?;

            for i in 0..function.get_num_args(library) {
                if i > 0 {
                    writer.write_fmt(format_args!(", "))?;
                }

                let arg = function.get_arg(library, i);

                let arg_name = get_identifier(arg.get_name(library));
                let ty_name = get_type_name(library, arg.get_type(library));

                writer.write_fmt(format_args!("{} : {}", arg_name, ty_name))?;
            }

            let ret_ty_name = get_type_name(library, function.get_return_type(library));

            writer.write_fmt(format_args!(") : {}", ret_ty_name))?;

            let mut first = true;

            for block in function.get_blocks(library) {
                let block_id = block.get_unique_index();

                if first {
                    writer.write_fmt(format_args!(" {{\n"))?;
                    first = false;
                }

                writer.write_fmt(format_args!("    b{}(", block_id))?;

                let mut values = HashMap::new();

                for i in 0..block.get_num_args(library) {
                    if i > 0 {
                        writer.write_fmt(format_args!(", "))?;
                    }

                    let arg = block.get_arg(library, i);

                    let arg_name = "v".to_string() + &arg.get_unique_index().to_string();
                    let ty_name = get_type_name(library, arg.get_type(library));

                    writer.write_fmt(format_args!("{} : {}", arg_name, ty_name))?;

                    values.insert(arg, arg_name);
                }

                writer.write_fmt(format_args!("):\n"))?;

                for value in block.get_insts(library) {
                    let inst = value.get_inst(library);

                    let inst_name = "v".to_string() + &value.get_unique_index().to_string();

                    values.insert(value, inst_name);

                    match inst {
                        Instruction::Return(loc) => writer
                            .write_fmt(format_args!("      ret {}\n", get_loc(library, loc)))?,
                        Instruction::ReturnValue(_, val, loc) => writer.write_fmt(format_args!(
                            "      ret {}{}\n",
                            values.get(&val).expect("ICE: bad"),
                            get_loc(library, loc)
                        ))?,
                        Instruction::Cmp(ty, cmp, a, b, loc) => writer.write_fmt(format_args!(
                            "      {} = {} {} {} {}{}\n",
                            values.get(&value).expect("ICE: bad"),
                            cmp,
                            get_type_name(library, *ty),
                            values.get(&a).expect("ICE: bad"),
                            values.get(&b).expect("ICE: bad"),
                            get_loc(library, loc)
                        ))?,
                        Instruction::Unary(ty, unary, a, loc) => writer.write_fmt(format_args!(
                            "      {} = {} {} {}{}\n",
                            values.get(&value).expect("ICE: bad"),
                            unary,
                            get_type_name(library, *ty),
                            values.get(&a).expect("ICE: bad"),
                            get_loc(library, loc)
                        ))?,
                        Instruction::Binary(ty, binary, a, b, loc) => {
                            writer.write_fmt(format_args!(
                                "      {} = {} {} {} {}{}\n",
                                values.get(&value).expect("ICE: bad"),
                                binary,
                                get_type_name(library, *ty),
                                values.get(&a).expect("ICE: bad"),
                                values.get(&b).expect("ICE: bad"),
                                get_loc(library, loc)
                            ))?
                        }
                        Instruction::Cast(ty, val, loc) => writer.write_fmt(format_args!(
                            "      {} = {} cast {}{}\n",
                            values.get(&value).expect("ICE: bad"),
                            get_type_name(library, *ty),
                            values.get(&val).expect("ICE: bad"),
                            get_loc(library, loc)
                        ))?,
                        Instruction::BitCast(ty, val, loc) => writer.write_fmt(format_args!(
                            "      {} = {} bitcast {}{}\n",
                            values.get(&value).expect("ICE: bad"),
                            get_type_name(library, *ty),
                            values.get(&val).expect("ICE: bad"),
                            get_loc(library, loc)
                        ))?,
                        Instruction::Load(ptr, loc) => writer.write_fmt(format_args!(
                            "      {} = load {}{}\n",
                            values.get(&value).expect("ICE: bad"),
                            values.get(&ptr).expect("ICE: bad"),
                            get_loc(library, loc)
                        ))?,
                        Instruction::Store(ptr, val, loc) => writer.write_fmt(format_args!(
                            "      store {} {}{}\n",
                            values.get(&ptr).expect("ICE: bad"),
                            values.get(&val).expect("ICE: bad"),
                            get_loc(library, loc)
                        ))?,
                        Instruction::Extract(agg, index, loc) => writer.write_fmt(format_args!(
                            "      {} = extract {} from {}{}\n",
                            values.get(&value).expect("ICE: bad"),
                            values.get(&agg).expect("ICE: bad"),
                            index,
                            get_loc(library, loc)
                        ))?,
                        Instruction::Insert(agg, elem, index, loc) => {
                            writer.write_fmt(format_args!(
                                "      {} = insert {}, {}, {}{}\n",
                                values.get(&value).expect("ICE: bad"),
                                values.get(&agg).expect("ICE: bad"),
                                values.get(&elem).expect("ICE: bad"),
                                index,
                                get_loc(library, loc)
                            ))?
                        }
                        Instruction::StackAlloc(name, ty, loc) => {
                            writer.write_fmt(format_args!(
                                "      {} = alloc {} {}{}\n",
                                values.get(&value).expect("ICE: bad"),
                                get_identifier(name.get_name(library)),
                                get_type_name(library, *ty),
                                get_loc(library, loc)
                            ))?
                        }
                        Instruction::Call(func, args, loc) => {
                            writer.write_fmt(format_args!(
                                "      {} = call {}",
                                values.get(&value).expect("ICE: bad"),
                                get_identifier(func.get_name(library)),
                            ))?;

                            for arg in args {
                                writer.write_fmt(format_args!(
                                    " {}",
                                    values.get(&arg).expect("ICE: bad")
                                ))?;
                            }

                            writer.write_fmt(format_args!("{}", get_loc(library, loc)))?
                        }
                        Instruction::Branch(block, args, loc) => {
                            writer
                                .write_fmt(format_args!("      br {}", block.get_unique_index()))?;

                            for arg in args {
                                writer.write_fmt(format_args!(
                                    " {}",
                                    values.get(&arg).expect("ICE: bad")
                                ))?;
                            }

                            writer.write_fmt(format_args!("{}", get_loc(library, loc)))?
                        }
                        Instruction::ConditionalBranch(
                            cond,
                            true_block,
                            false_block,
                            args,
                            loc,
                        ) => {
                            writer.write_fmt(format_args!(
                                "      condbr {} {} {}",
                                values.get(&cond).expect("ICE: bad"),
                                true_block.get_unique_index(),
                                false_block.get_unique_index()
                            ))?;

                            for arg in args {
                                writer.write_fmt(format_args!(
                                    " {}",
                                    values.get(&arg).expect("ICE: bad")
                                ))?;
                            }

                            writer.write_fmt(format_args!("{}", get_loc(library, loc)))?
                        }
                        Instruction::Select(_, cond, true_val, false_val, loc) => writer
                            .write_fmt(format_args!(
                                "      {} = select {} {} {}{}\n",
                                values.get(&value).expect("ICE: bad"),
                                values.get(&cond).expect("ICE: bad"),
                                values.get(&true_val).expect("ICE: bad"),
                                values.get(&false_val).expect("ICE: bad"),
                                get_loc(library, loc)
                            ))?,
                        Instruction::GetElementPtr(_, ptr, args, loc) => {
                            writer.write_fmt(format_args!(
                                "      {} = gep {}",
                                values.get(&value).expect("ICE: bad"),
                                values.get(&ptr).expect("ICE: bad"),
                            ))?;

                            for arg in args {
                                writer.write_fmt(format_args!(
                                    " {}",
                                    values.get(&arg).expect("ICE: bad")
                                ))?;
                            }

                            writer.write_fmt(format_args!("{}", get_loc(library, loc)))?
                        }
                    }
                }
            }

            // If we found at least one block, the function had a body
            if !first {
                writer.write_fmt(format_args!("  }}\n"))?;
            }
        }

        writer.write_fmt(format_args!("}}\n"))?;
    }

    Ok(())
}
