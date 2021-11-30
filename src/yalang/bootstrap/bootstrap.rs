#[macro_use]
extern crate clap;
extern crate codemap;
extern crate logos;

use clap::App;
use codemap::*;
use logos::Logos;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::hash::Hash;
use std::io::{self, Read, Write};
use std::rc::Rc;
use std::sync::Arc;
use yair::io::*;
use yair::*;

type ExportedPackageFunctions = Rc<RefCell<HashMap<String, HashMap<String, yair::Function>>>>;

#[derive(PartialEq, Eq)]
enum PrecedenceGroup {
    Arithmetic,
    Bitwise,
    Cast,
    Parenthesis,
    Comparison,
}

#[derive(Debug)]
enum OperandKind {
    Pointer((yair::Value, Option<yair::Location>)),
    Concrete(yair::Value),
    Float(f64),
    Integer(u64),
}

#[derive(Debug)]
struct Operand {
    location: Location,
    kind: OperandKind,
}

fn get_precedence(x: Token) -> (PrecedenceGroup, u8) {
    match x {
        Token::Mul | Token::Div | Token::Mod => (PrecedenceGroup::Arithmetic, 0),
        Token::Add | Token::Sub => (PrecedenceGroup::Arithmetic, 1),
        Token::Not => (PrecedenceGroup::Bitwise, 0),
        Token::And => (PrecedenceGroup::Bitwise, 1),
        Token::Or => (PrecedenceGroup::Bitwise, 2),
        Token::Xor => (PrecedenceGroup::Bitwise, 3),
        Token::As => (PrecedenceGroup::Cast, 0),
        Token::LParen | Token::RParen => (PrecedenceGroup::Parenthesis, u8::MAX),
        Token::Equals
        | Token::NotEquals
        | Token::LessThan
        | Token::LessThanEquals
        | Token::GreaterThan
        | Token::GreaterThanEquals => (PrecedenceGroup::Comparison, 0),
        _ => todo!(),
    }
}

fn is_unary(x: Token) -> bool {
    matches!(x, Token::As | Token::Not)
}

#[derive(Logos, Copy, Clone, Debug, PartialEq)]
enum Token {
    #[token("package")]
    Package,

    #[token("import")]
    Import,

    #[token("return")]
    Return,

    #[token(":")]
    Colon,

    #[token(";")]
    Semicolon,

    #[token("[")]
    LBracket,

    #[token("]")]
    RBracket,

    #[token("{")]
    LCurly,

    #[token("}")]
    RCurly,

    #[token("(")]
    LParen,

    #[token(")")]
    RParen,

    #[token("+")]
    Add,

    #[token("-")]
    Sub,

    #[token("*")]
    Mul,

    #[token("/")]
    Div,

    #[token("%")]
    Mod,

    #[token("&")]
    And,

    #[token("|")]
    Or,

    #[token("^")]
    Xor,

    #[token("as")]
    As,

    #[token(",")]
    Comma,

    #[token("!")]
    Not,

    #[token("==")]
    Equals,

    #[token("!=")]
    NotEquals,

    #[token("<")]
    LessThan,

    #[token("<=")]
    LessThanEquals,

    #[token(">")]
    GreaterThan,

    #[token(">=")]
    GreaterThanEquals,

    #[token("=")]
    Assignment,

    #[token("if")]
    If,

    #[token("else")]
    Else,

    #[token("while")]
    While,

    #[token("break")]
    Break,

    #[token("continue")]
    Continue,

    #[token("in")]
    In,

    #[token("..")]
    Range,

    #[regex("[\\p{L}][\\p{L}0-9]*(::[\\p{L}][\\p{L}0-9]*)*")]
    IdentifierWithNamespace,

    #[regex("[\\p{L}][\\p{L}0-9]*", priority = 2)]
    Identifier,

    #[regex("[1-9][0-9]*", priority = 2)]
    Integer,

    #[regex("[+-]?([0-9]+([.][0-9]*)?([eE][+-]?[0-9]+)?|[.][0-9]+([eE][+-]?[0-9]+)?)")]
    Float,

    #[token("true")]
    True,

    #[token("false")]
    False,

    #[regex("\"(\\.|[^\"])*\"")]
    String,

    #[error]
    // Skip whitespace.
    #[regex(r"[ \t\r\n\f]+", logos::skip)]
    // Skip single-line comments.
    #[regex("//[^\n\r]+", logos::skip)]
    // Skip multi-line comments.
    #[regex("/\\*[^*]*\\*+(?:[^/*][^*]*\\*+)*/", logos::skip)]
    Error,
}

type Location = codemap::Span;

enum ParseError {
    UnexpectedEndOfFile,
    ExpectedTokenNotFound(Token, Location),
    InvalidExpression(Location),
    OperatorsInDifferentPrecedenceGroups(Location, Token, Location, Token),
    TypesDoNotMatch(Location, yair::Type, Location, yair::Type),
    InvalidNonConcreteConstantUsed(Location),
    InvalidNonConcreteConstantsUsed(Location, Location),
    UnknownIdentifier(Location),
    ComparisonOperatorsAlwaysNeedParenthesis(Location, Token, Location, Token),
    UnknownIdentifierUsedInAssignment(Location),
    IdentifierShadowsPreviouslyDeclareIdentifier(Location, Location),
    ElseStatementWithoutIf(Location),
}

#[derive(Clone)]
struct Name {
    file: Arc<codemap::File>,
    location: Location,
}

impl Name {
    pub fn new(file: Arc<codemap::File>, location: Location) -> Self {
        Name { file, location }
    }

    pub fn as_str(&self) -> &str {
        self.file.source_slice(self.location)
    }
}

impl Hash for Name {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.as_str().hash(hasher)
    }
}

impl Eq for Name {}

impl PartialEq for Name {
    fn eq(&self, other: &Name) -> bool {
        self.as_str() == other.as_str()
    }
}

#[derive(Copy, Clone, Debug)]
struct TokenLoc {
    token: Token,
    location: codemap::Span,
}

struct Lexer<'a> {
    file: Arc<codemap::File>,
    lexer: logos::Lexer<'a, Token>,
    peek: Option<TokenLoc>,
}

impl<'a> Lexer<'a> {
    pub fn new(file: Arc<codemap::File>, source: &'a str) -> Self {
        let mut lexer = Self {
            file,
            lexer: Token::lexer(source),
            peek: None,
        };

        lexer.next();

        lexer
    }

    pub fn peek(&self) -> Option<TokenLoc> {
        self.peek
    }

    pub fn next(&mut self) -> Option<TokenLoc> {
        let next = self.peek;

        let token = self.lexer.next();

        self.peek = if let Some(token) = token {
            let lexer_span = self.lexer.span();
            let location = self
                .file
                .span
                .subspan(lexer_span.start as u64, lexer_span.end as u64);

            Some(TokenLoc { token, location })
        } else {
            None
        };

        next
    }
}

#[allow(dead_code)]
struct Parser<'a> {
    exported_package_functions: ExportedPackageFunctions,
    codemap: CodeMap,
    file: Arc<codemap::File>,
    functions: HashMap<String, yair::Function>,
    package: String,
    imports: Vec<String>,
    lexer: Lexer<'a>,
    identifiers: HashMap<Name, (yair::Type, yair::Value, Location)>,
    scoped_identifiers: Vec<Vec<Name>>,
    merge_blocks: Vec<yair::Block>,
    continue_blocks: Vec<yair::Block>,
}

impl<'a> Eq for Parser<'a> {}

impl<'a> PartialEq for Parser<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.file == other.file
    }
}

impl<'a> Ord for Parser<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // If we aren't in a package name but the other is, we always want the other one to be processed first.
        if self.package.is_empty() && !other.package.is_empty() {
            return std::cmp::Ordering::Greater;
        }

        // And the opposite if we are in a package but the other isn't, we always want to be processed first.
        if !self.package.is_empty() && other.package.is_empty() {
            return std::cmp::Ordering::Less;
        }

        // If our import list requires the other package, it goes first.
        if self.imports.contains(&other.package) {
            return std::cmp::Ordering::Greater;
        }

        // If the other import list requires our package, we go first.
        if other.imports.contains(&self.package) {
            return std::cmp::Ordering::Less;
        }

        std::cmp::Ordering::Equal
    }
}

impl<'a> PartialOrd for Parser<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Parser<'a> {
    pub fn new(
        exported_package_functions: ExportedPackageFunctions,
        name: String,
        data: &'a str,
    ) -> Parser<'a> {
        let mut codemap = CodeMap::new();

        let file = codemap.add_file(name, data.to_string());

        Parser {
            exported_package_functions,
            codemap,
            file: file.clone(),
            functions: HashMap::new(),
            package: "".to_string(),
            imports: Vec::new(),
            lexer: Lexer::new(file, data),
            identifiers: HashMap::new(),
            scoped_identifiers: Vec::new(),
            merge_blocks: Vec::new(),
            continue_blocks: Vec::new(),
        }
    }

    fn get_location(
        &self,
        location: codemap::Span,
        context: &mut yair::Context,
    ) -> Option<yair::Location> {
        let location = self.codemap.look_up_span(location);

        Some(context.get_location(
            location.file.name(),
            location.begin.line + 1,
            location.begin.column + 1,
        ))
    }

    fn expect_symbol(&mut self, token: Token) -> Result<Location, ParseError> {
        if let Some(token_loc) = self.lexer.next() {
            if token_loc.token == token {
                Ok(token_loc.location)
            } else {
                Err(ParseError::ExpectedTokenNotFound(token, token_loc.location))
            }
        } else {
            Err(ParseError::UnexpectedEndOfFile)
        }
    }

    fn bump_if_symbol(&mut self, token: Token) -> bool {
        if let Some(peek) = self.lexer.peek() {
            if token == peek.token {
                self.lexer.next();
                return true;
            }
        }

        false
    }

    fn get_kind(&mut self, kind: OperandKind, builder: &mut InstructionBuilder) -> OperandKind {
        if let OperandKind::Pointer((value, location)) = kind {
            OperandKind::Concrete(builder.load(value, location))
        } else {
            kind
        }
    }

    fn apply(
        &mut self,
        operand_stack: &mut Vec<Operand>,
        operator_stack: &mut Vec<(Location, Token)>,
        builder: &mut InstructionBuilder,
    ) -> Result<(), ParseError> {
        let y = operand_stack.pop().unwrap();

        let op = if let Some(op) = operator_stack.pop() {
            op
        } else {
            // No operator between operands `42 13`
            return Err(ParseError::InvalidExpression(y.location));
        };

        let location = self.get_location(op.0, builder.borrow_context());

        if is_unary(op.1) {
            // TODO: Check that not has a int or bool type!

            match self.get_kind(y.kind, builder) {
                OperandKind::Concrete(value) => {
                    let expr = match op.1 {
                        Token::Not => builder.not(value, location),
                        _ => todo!(),
                    };

                    operand_stack.push(Operand {
                        location: op.0,
                        kind: OperandKind::Concrete(expr),
                    });

                    Ok(())
                }
                _ => Err(ParseError::InvalidNonConcreteConstantUsed(y.location)),
            }
        } else {
            let x = operand_stack.pop().unwrap();

            let x_kind = self.get_kind(x.kind, builder);

            let y_kind = self.get_kind(y.kind, builder);

            let (x_value, y_value) = match x_kind {
                OperandKind::Concrete(x_value) => match y_kind {
                    OperandKind::Concrete(y_value) => {
                        let x_ty = x_value.get_type(builder.borrow_context());
                        let y_ty = y_value.get_type(builder.borrow_context());

                        if x_ty != y_ty {
                            return Err(ParseError::TypesDoNotMatch(
                                x.location, x_ty, y.location, y_ty,
                            ));
                        }

                        (x_value, y_value)
                    }
                    OperandKind::Float(y_float) => {
                        let x_ty = x_value.get_type(builder.borrow_context());

                        if !x_ty.is_float(builder.borrow_context()) {
                            let bits = x_ty.get_bits(builder.borrow_context());

                            let y_value = builder
                                .borrow_context()
                                .get_float_constant(bits as u8, y_float);

                            (x_value, y_value)
                        } else {
                            todo!()
                        }
                    }
                    OperandKind::Integer(y_int) => {
                        let x_ty = x_value.get_type(builder.borrow_context());

                        if x_ty.is_float(builder.borrow_context()) {
                            let bits = x_ty.get_bits(builder.borrow_context());

                            let y_value = builder
                                .borrow_context()
                                .get_float_constant(bits as u8, y_int as f64);

                            (x_value, y_value)
                        } else if x_ty.is_int(builder.borrow_context()) {
                            let bits = x_ty.get_bits(builder.borrow_context());

                            let y_value = builder
                                .borrow_context()
                                .get_int_constant(bits as u8, y_int as i64);

                            (x_value, y_value)
                        } else if x_ty.is_uint(builder.borrow_context()) {
                            let bits = x_ty.get_bits(builder.borrow_context());

                            let y_value = builder
                                .borrow_context()
                                .get_uint_constant(bits as u8, y_int);

                            (x_value, y_value)
                        } else {
                            todo!()
                        }
                    }
                    _ => panic!("Unhandled"),
                },
                OperandKind::Float(x_float) => match y_kind {
                    OperandKind::Concrete(y_value) => {
                        let y_ty = y_value.get_type(builder.borrow_context());

                        if !y_ty.is_float(builder.borrow_context()) {
                            let bits = y_ty.get_bits(builder.borrow_context());

                            let x_value = builder
                                .borrow_context()
                                .get_float_constant(bits as u8, x_float);

                            (x_value, y_value)
                        } else {
                            todo!()
                        }
                    }
                    _ => {
                        return Err(ParseError::InvalidNonConcreteConstantsUsed(
                            x.location, y.location,
                        ))
                    }
                },
                OperandKind::Integer(x_int) => match y_kind {
                    OperandKind::Concrete(y_value) => {
                        let y_ty = y_value.get_type(builder.borrow_context());

                        if y_ty.is_float(builder.borrow_context()) {
                            let bits = y_ty.get_bits(builder.borrow_context());

                            let x_value = builder
                                .borrow_context()
                                .get_float_constant(bits as u8, x_int as f64);

                            (x_value, y_value)
                        } else if y_ty.is_int(builder.borrow_context()) {
                            let bits = y_ty.get_bits(builder.borrow_context());

                            let x_value = builder
                                .borrow_context()
                                .get_int_constant(bits as u8, x_int as i64);

                            (x_value, y_value)
                        } else if y_ty.is_uint(builder.borrow_context()) {
                            let bits = y_ty.get_bits(builder.borrow_context());

                            let x_value = builder
                                .borrow_context()
                                .get_uint_constant(bits as u8, x_int);

                            (x_value, y_value)
                        } else {
                            todo!()
                        }
                    }
                    _ => {
                        return Err(ParseError::InvalidNonConcreteConstantsUsed(
                            x.location, y.location,
                        ))
                    }
                },
                _ => panic!("Unhandled"),
            };

            let expr = match op.1 {
                Token::Add => builder.add(x_value, y_value, location),
                Token::Sub => builder.sub(x_value, y_value, location),
                Token::Mul => builder.mul(x_value, y_value, location),
                Token::Div => builder.div(x_value, y_value, location),
                Token::Mod => builder.rem(x_value, y_value, location),
                Token::And => builder.and(x_value, y_value, location),
                Token::Or => builder.or(x_value, y_value, location),
                Token::Xor => builder.xor(x_value, y_value, location),
                Token::Equals => builder.cmp_eq(x_value, y_value, location),
                Token::NotEquals => builder.cmp_ne(x_value, y_value, location),
                Token::LessThan => builder.cmp_lt(x_value, y_value, location),
                Token::LessThanEquals => builder.cmp_le(x_value, y_value, location),
                Token::GreaterThan => builder.cmp_gt(x_value, y_value, location),
                Token::GreaterThanEquals => builder.cmp_ge(x_value, y_value, location),
                _ => todo!(),
            };

            operand_stack.push(Operand {
                location: op.0,
                kind: OperandKind::Concrete(expr),
            });

            Ok(())
        }
    }

    fn check_precedence(
        &mut self,
        x: (Location, Token),
        y: (Location, Token),
    ) -> Result<(), ParseError> {
        let x_precedence = get_precedence(x.1);
        let y_precedence = get_precedence(y.1);

        if x_precedence.0 == PrecedenceGroup::Comparison
            || y_precedence.0 == PrecedenceGroup::Comparison
        {
            Err(ParseError::ComparisonOperatorsAlwaysNeedParenthesis(
                y.0, y.1, x.0, x.1,
            ))
        } else if x_precedence.0 == PrecedenceGroup::Parenthesis
            || y_precedence.0 == PrecedenceGroup::Parenthesis
        {
            // Parenthesis sit outside precedence groups because they are used to form pairs of precedence groups.
            Ok(())
        } else if x_precedence.0 != y_precedence.0 {
            Err(ParseError::OperatorsInDifferentPrecedenceGroups(
                y.0, y.1, x.0, x.1,
            ))
        } else {
            Ok(())
        }
    }

    fn apply_if_lower_precedence_and_push_operator(
        &mut self,
        x: (Location, Token),
        operand_stack: &mut Vec<Operand>,
        operator_stack: &mut Vec<(Location, Token)>,
        builder: &mut InstructionBuilder,
    ) -> Result<(), ParseError> {
        self.apply_if_lower_precedence(x, operand_stack, operator_stack, builder)?;

        operator_stack.push(x);

        Ok(())
    }

    fn next_token(&mut self) -> Result<Token, ParseError> {
        if let Some(next) = self.lexer.next() {
            Ok(next.token)
        } else {
            Err(ParseError::UnexpectedEndOfFile)
        }
    }

    fn apply_if_lower_precedence(
        &mut self,
        x: (Location, Token),
        operand_stack: &mut Vec<Operand>,
        operator_stack: &mut Vec<(Location, Token)>,
        builder: &mut InstructionBuilder,
    ) -> Result<(), ParseError> {
        let x_precedence = get_precedence(x.1);

        while !operator_stack.is_empty() {
            let y = operator_stack.last().unwrap();

            if x.1 == Token::RParen && y.1 == Token::LParen {
                operator_stack.pop();
                break;
            }

            self.check_precedence(x, *y)?;

            let y_precedence = get_precedence(y.1);

            if x_precedence.1 < y_precedence.1 {
                break;
            }

            self.apply(operand_stack, operator_stack, builder)?;
        }

        Ok(())
    }

    fn parse_integer(&self, source: &str) -> Result<u64, ParseError> {
        if let Ok(i) = source.parse::<u64>() {
            Ok(i)
        } else {
            todo!()
        }
    }

    fn parse_float(&self, source: &str) -> Result<f64, ParseError> {
        if let Ok(f) = source.parse::<f64>() {
            Ok(f)
        } else {
            todo!()
        }
    }

    fn end_of_file_location(&self) -> Location {
        self.file
            .span
            .subspan(self.file.span.len(), self.file.span.len())
    }

    fn parse_expression(
        &mut self,
        ty: Type,
        builder: &mut InstructionBuilder,
    ) -> Result<(yair::Value, Location), ParseError> {
        let mut operand_stack = Vec::new();
        let mut operator_stack: Vec<(Location, Token)> = Vec::new();

        if ty.is_array(builder.borrow_context()) {
            // Array initializers always start with a  '{'.
            let start_location = self.expect_symbol(Token::LCurly)?;

            let element_ty = ty.get_element(builder.borrow_context(), 0);

            let mut initializer = builder.borrow_context().get_undef(ty);

            for i in 0..ty.get_len(builder.borrow_context()) {
                let (expr, location) = self.parse_expression(element_ty, builder)?;

                let location = self.get_location(location, builder.borrow_context());

                initializer = builder.insert(initializer, expr, i, location);

                if let Some(peek) = self.lexer.peek() {
                    if peek.token != Token::RCurly {
                        self.expect_symbol(Token::Comma)?;
                    }
                }
            }

            // Array initializers always end with a '}'.
            let end_location = self.expect_symbol(Token::RCurly)?;

            return Ok((initializer, start_location.merge(end_location)));
        }

        loop {
            if let Some(peek) = self.lexer.peek() {
                match peek.token {
                    Token::Semicolon => break,
                    Token::LCurly => break,
                    Token::RCurly => break,
                    Token::RBracket => break,
                    Token::Comma => break,
                    _ => (),
                }
            }

            let token_loc = self.lexer.next();

            if let Some(token_loc) = token_loc {
                let source = self.file.source_slice(token_loc.location);

                match token_loc.token {
                    Token::True => operand_stack.push(Operand {
                        location: token_loc.location,
                        kind: OperandKind::Concrete(
                            builder.borrow_context().get_bool_constant(true),
                        ),
                    }),
                    Token::False => operand_stack.push(Operand {
                        location: token_loc.location,
                        kind: OperandKind::Concrete(
                            builder.borrow_context().get_bool_constant(false),
                        ),
                    }),
                    Token::Integer => operand_stack.push(Operand {
                        location: token_loc.location,
                        kind: OperandKind::Integer(self.parse_integer(source)?),
                    }),
                    Token::Float => operand_stack.push(Operand {
                        location: token_loc.location,
                        kind: OperandKind::Float(self.parse_float(source)?),
                    }),
                    Token::LParen => operator_stack.push((token_loc.location, Token::LParen)),
                    Token::RParen => self.apply_if_lower_precedence(
                        (token_loc.location, Token::RParen),
                        &mut operand_stack,
                        &mut operator_stack,
                        builder,
                    )?,
                    Token::As => {
                        let range = token_loc.location;

                        if !operator_stack.is_empty() {
                            self.check_precedence(
                                (range, Token::As),
                                *operator_stack.last().unwrap(),
                            )?;
                        }

                        let ty = self.parse_type(None, builder.borrow_context())?;

                        let expr = if let Some(x) = operand_stack.pop() {
                            let kind = self.get_kind(x.kind, builder);

                            match kind {
                                OperandKind::Concrete(v) => {
                                    let location =
                                        self.get_location(range, builder.borrow_context());
                                    builder.cast(v, ty, location)
                                }
                                OperandKind::Float(f) => {
                                    if ty.is_float(builder.borrow_context()) {
                                        let bits = ty.get_bits(builder.borrow_context());
                                        builder.borrow_context().get_float_constant(bits as u8, f)
                                    } else {
                                        todo!()
                                    }
                                }
                                OperandKind::Integer(i) => {
                                    if ty.is_float(builder.borrow_context()) {
                                        let bits = ty.get_bits(builder.borrow_context());
                                        builder
                                            .borrow_context()
                                            .get_float_constant(bits as u8, i as f64)
                                    } else if ty.is_int(builder.borrow_context()) {
                                        let bits = ty.get_bits(builder.borrow_context());
                                        builder
                                            .borrow_context()
                                            .get_int_constant(bits as u8, i as i64)
                                    } else if ty.is_uint(builder.borrow_context()) {
                                        let bits = ty.get_bits(builder.borrow_context());
                                        builder.borrow_context().get_uint_constant(bits as u8, i)
                                    } else {
                                        todo!()
                                    }
                                }
                                _ => panic!("Unhandled"),
                            }
                        } else {
                            todo!()
                        };

                        operand_stack.push(Operand {
                            location: range,
                            kind: OperandKind::Concrete(expr),
                        });
                    }
                    x if matches!(
                        x,
                        Token::Add
                            | Token::Sub
                            | Token::Mul
                            | Token::Div
                            | Token::Mod
                            | Token::Not
                            | Token::And
                            | Token::Or
                            | Token::Xor
                            | Token::Equals
                            | Token::NotEquals
                            | Token::LessThan
                            | Token::LessThanEquals
                            | Token::GreaterThan
                            | Token::GreaterThanEquals
                    ) =>
                    {
                        self.apply_if_lower_precedence_and_push_operator(
                            (token_loc.location, x),
                            &mut operand_stack,
                            &mut &mut operator_stack,
                            builder,
                        )?
                    }
                    x if matches!(x, Token::Identifier | Token::IdentifierWithNamespace) => {
                        let identifier = Name::new(self.file.clone(), token_loc.location);

                        // If we've got a parenthesis straight after our identifier, we've got a function call!
                        if self.bump_if_symbol(Token::LParen) {
                            let call = self.parse_called_function(identifier, builder)?;
                            operand_stack.push(Operand {
                                location: token_loc.location,
                                kind: OperandKind::Concrete(call),
                            });
                        } else if let Some((_, value, range)) = self.identifiers.get(&identifier) {
                            let location = self.get_location(*range, builder.borrow_context());
                            operand_stack.push(Operand {
                                location: token_loc.location,
                                kind: OperandKind::Pointer((*value, location)),
                            });
                        } else {
                            return Err(ParseError::UnknownIdentifier(token_loc.location));
                        }
                    }
                    Token::LBracket => {
                        let start_location = token_loc.location;

                        let u64_ty = builder.borrow_context().get_uint_type(64);
                        let index = self.parse_expression(u64_ty, builder)?;

                        let operand = operand_stack.pop().unwrap();

                        let operand = if let OperandKind::Pointer(ptr) = operand.kind {
                            ptr.0
                        } else {
                            todo!()
                        };

                        let end_location = self.expect_symbol(Token::RBracket)?;

                        let full_location = start_location.merge(end_location);

                        let location = self.get_location(full_location, builder.borrow_context());

                        let expr = builder.index_into(operand, &[index.0], location);

                        operand_stack.push(Operand {
                            location: full_location,
                            kind: OperandKind::Pointer((expr, location)),
                        });
                    }
                    _ => return Err(ParseError::UnknownIdentifier(token_loc.location)),
                }
            } else {
                todo!();
            }
        }

        // Handle the case where an expression is malformed like `foo=;`
        if operand_stack.is_empty() {
            let location = self
                .lexer
                .peek()
                .map_or(self.end_of_file_location(), |token_loc| token_loc.location);
            return Err(ParseError::InvalidExpression(location));
        }

        while !operator_stack.is_empty() {
            self.apply(&mut operand_stack, &mut operator_stack, builder)?;
        }

        let operand = operand_stack.pop().unwrap();

        let expr = match operand.kind {
            OperandKind::Pointer((value, location)) => builder.load(value, location),
            OperandKind::Concrete(v) => v,
            OperandKind::Float(f) => {
                if ty.is_float(builder.borrow_context()) {
                    let bits = ty.get_bits(builder.borrow_context());
                    builder.borrow_context().get_float_constant(bits as u8, f)
                } else {
                    todo!();
                }
            }
            OperandKind::Integer(i) => {
                if ty.is_float(builder.borrow_context()) {
                    let bits = ty.get_bits(builder.borrow_context());
                    builder
                        .borrow_context()
                        .get_float_constant(bits as u8, i as f64)
                } else if ty.is_int(builder.borrow_context()) {
                    let bits = ty.get_bits(builder.borrow_context());
                    builder
                        .borrow_context()
                        .get_int_constant(bits as u8, i as i64)
                } else if ty.is_uint(builder.borrow_context()) {
                    let bits = ty.get_bits(builder.borrow_context());
                    builder.borrow_context().get_uint_constant(bits as u8, i)
                } else {
                    todo!();
                }
            }
        };

        Ok((expr, operand.location))
    }

    fn push_scope(&mut self) {
        self.scoped_identifiers.push(Vec::new());
    }

    fn pop_scope(&mut self) {
        for identifier in self.scoped_identifiers.last().unwrap().iter() {
            self.identifiers.remove(identifier).unwrap();
        }

        self.scoped_identifiers.pop();
    }

    fn add_identifier(
        &mut self,
        identifier: Name,
        location: Location,
        ty: yair::Type,
        value: yair::Value,
    ) -> Result<(), ParseError> {
        if let Some(original) = self
            .identifiers
            .insert(identifier.clone(), (ty, value, location))
        {
            return Err(ParseError::IdentifierShadowsPreviouslyDeclareIdentifier(
                original.2,
                identifier.location,
            ));
        }

        self.scoped_identifiers.last_mut().unwrap().push(identifier);

        Ok(())
    }

    fn parse_if(
        &mut self,
        function: yair::Function,
        alloca_block: yair::Block,
        condition: yair::Value,
        block: yair::Block,
        context: &mut yair::Context,
    ) -> Result<yair::Block, ParseError> {
        let location = self.expect_symbol(Token::LCurly)?;

        let location = self.get_location(location, context);

        let mut exit_blocks = Vec::new();

        let (if_entry_block, if_exit_block) = self.parse_block(function, alloca_block, context)?;

        exit_blocks.push((if_exit_block, None));

        let false_block = if self.bump_if_symbol(Token::Else) {
            if self.bump_if_symbol(Token::If) {
                let else_if_block = function.create_block(context).build();

                let mut builder = else_if_block.create_instructions(context);

                let bool_ty = builder.borrow_context().get_bool_type();

                let expr = self.parse_expression(bool_ty, &mut builder)?;

                let exit_block = self.parse_if(
                    function,
                    alloca_block,
                    expr.0,
                    else_if_block,
                    builder.borrow_context(),
                )?;

                exit_blocks.push((exit_block, None));

                Some(else_if_block)
            } else {
                self.expect_symbol(Token::LCurly)?;

                let (else_entry_block, else_exit_block) =
                    self.parse_block(function, alloca_block, context)?;

                exit_blocks.push((else_exit_block, None));

                Some(else_entry_block)
            }
        } else {
            None
        };

        let merge_block = function.create_block(context).build();

        let false_block = false_block.map_or(merge_block, |block| block);

        block.create_instructions(context).conditional_branch(
            condition,
            if_entry_block,
            false_block,
            &[],
            &[],
            location,
        );

        for (exit_block, location) in exit_blocks {
            if !exit_block.has_terminator(context) {
                exit_block
                    .create_instructions(context)
                    .branch(merge_block, &[], location);
            }
        }

        Ok(merge_block)
    }

    fn parse_while(
        &mut self,
        function: yair::Function,
        alloca_block: yair::Block,
        block: yair::Block,
        context: &mut yair::Context,
    ) -> Result<yair::Block, ParseError> {
        let check_block = function.create_block(context).build();
        let body_block = function.create_block(context).build();
        let merge_block = function.create_block(context).build();

        self.merge_blocks.push(merge_block);
        self.continue_blocks.push(check_block);

        block
            .create_instructions(context)
            .branch(check_block, &[], None);

        let mut builder = check_block.create_instructions(context);

        let bool_ty = builder.borrow_context().get_bool_type();

        let (condition, location) = self.parse_expression(bool_ty, &mut builder)?;

        let location = self.get_location(location, builder.borrow_context());
        builder.conditional_branch(condition, body_block, merge_block, &[], &[], location);

        self.expect_symbol(Token::LCurly)?;

        let (while_entry_block, while_exit_block) =
            self.parse_block(function, alloca_block, context)?;

        body_block
            .create_instructions(context)
            .branch(while_entry_block, &[], None);

        if !while_exit_block.has_terminator(context) {
            while_exit_block
                .create_instructions(context)
                .branch(check_block, &[], None);
        }

        self.merge_blocks.pop();
        self.continue_blocks.pop();

        Ok(merge_block)
    }

    fn parse_called_function(
        &mut self,
        identifier: Name,
        builder: &mut InstructionBuilder,
    ) -> Result<yair::Value, ParseError> {
        // Need to look in our local functions first (most optimal)
        let called_function = if let Some(called_function) = self.functions.get(identifier.as_str())
        {
            *called_function
        } else {
            let exported_package_functions = self.exported_package_functions.borrow();

            let mut found_called_function = None;

            for import in &self.imports {
                if let Some(called_function) = exported_package_functions
                    .get(import)
                    .unwrap()
                    .get(identifier.as_str())
                {
                    found_called_function = Some(*called_function);
                    break;
                }
            }

            if let Some(called_function) = found_called_function {
                called_function
            } else {
                panic!("Could not find called function!");
            }
        };

        let num_args = called_function.get_num_args(builder.borrow_context());

        let mut call_args = Vec::new();

        for i in 0..num_args {
            // Anything except the first argument should have a comma before it.
            if i != 0 {
                self.expect_symbol(Token::Comma)?;
            }

            let arg = called_function.get_arg(builder.borrow_context(), i);
            let ty = arg.get_type(builder.borrow_context());

            let expression = self.parse_expression(ty, builder)?;

            call_args.push(expression.0);
        }

        self.expect_symbol(Token::RParen)?;

        let location = self.get_location(identifier.location, builder.borrow_context());

        Ok(builder.call(called_function, &call_args, location))
    }

    fn parse_block(
        &mut self,
        function: yair::Function,
        alloca_block: yair::Block,
        context: &mut yair::Context,
    ) -> Result<(yair::Block, yair::Block), ParseError> {
        self.push_scope();

        let entry_block = function.create_block(context).build();

        let mut current_block = entry_block;
        let mut builder = current_block.create_instructions(context);

        loop {
            let token_loc = self.lexer.next();

            if let Some(token_loc) = token_loc {
                match token_loc.token {
                    Token::LCurly => {
                        let location =
                            self.get_location(token_loc.location, builder.borrow_context());

                        let (sub_entry_block, sub_exit_block) =
                            self.parse_block(function, alloca_block, builder.borrow_context())?;

                        current_block
                            .create_instructions(builder.borrow_context())
                            .branch(sub_entry_block, &[], location);

                        current_block = function.create_block(builder.borrow_context()).build();

                        if !sub_exit_block.has_terminator(builder.borrow_context()) {
                            sub_exit_block
                                .create_instructions(builder.borrow_context())
                                .branch(current_block, &[], None);
                        }
                        builder = current_block.create_instructions(context);
                    }
                    Token::RCurly => {
                        self.pop_scope();
                        return Ok((entry_block, current_block));
                    }
                    x if matches!(x, Token::Identifier | Token::IdentifierWithNamespace) => {
                        let identifier = Name::new(self.file.clone(), token_loc.location);

                        if self.bump_if_symbol(Token::LParen) {
                            self.parse_called_function(identifier.clone(), &mut builder)?;
                            self.expect_symbol(Token::Semicolon)?;

                            // If we have a call here we're not capturing its return value.
                        } else {
                            let token_loc = self.lexer.next().unwrap();
                            match token_loc.token {
                                Token::Colon => {
                                    let location = self
                                        .get_location(token_loc.location, builder.borrow_context());

                                    let ty = self.parse_type(None, builder.borrow_context())?;

                                    let stack_alloc = {
                                        let mut builder = alloca_block
                                            .create_instructions(builder.borrow_context());

                                        let alloc =
                                            builder.stack_alloc(identifier.as_str(), ty, location);

                                        builder.pause_building();

                                        alloc
                                    };

                                    self.add_identifier(
                                        identifier,
                                        token_loc.location,
                                        ty,
                                        stack_alloc,
                                    )?;

                                    let (expr, location) = match self.next_token()? {
                                        Token::Assignment => (
                                            self.parse_expression(ty, &mut builder)?,
                                            self.get_location(
                                                token_loc.location,
                                                builder.borrow_context(),
                                            ),
                                        ),
                                        _ => todo!(),
                                    };

                                    self.expect_symbol(Token::Semicolon)?;

                                    builder.store(stack_alloc, expr.0, location);
                                }
                                Token::Assignment => {
                                    let location = self
                                        .get_location(token_loc.location, builder.borrow_context());

                                    let (ty, stack_alloc) = if let Some(identifier) =
                                        self.identifiers.get(&identifier)
                                    {
                                        (identifier.0, identifier.1)
                                    } else {
                                        return Err(ParseError::UnknownIdentifierUsedInAssignment(
                                            identifier.location,
                                        ));
                                    };

                                    let expr = self.parse_expression(ty, &mut builder)?;

                                    self.expect_symbol(Token::Semicolon)?;

                                    builder.store(stack_alloc, expr.0, location);
                                }
                                _ => todo!(),
                            }
                        }
                    }
                    Token::Return => {
                        let location =
                            self.get_location(token_loc.location, builder.borrow_context());

                        let return_ty = function.get_return_type(builder.borrow_context());

                        let expr = self.parse_expression(return_ty, &mut builder)?;

                        self.expect_symbol(Token::Semicolon)?;

                        builder.ret_val(expr.0, location);

                        return Ok((entry_block, current_block));
                    }
                    Token::If => {
                        let bool_ty = builder.borrow_context().get_bool_type();

                        let expr = self.parse_expression(bool_ty, &mut builder)?;

                        let exit_block = self.parse_if(
                            function,
                            alloca_block,
                            expr.0,
                            current_block,
                            builder.borrow_context(),
                        )?;

                        current_block = exit_block;
                        builder = current_block.create_instructions(context);
                    }
                    Token::While => {
                        let exit_block = self.parse_while(
                            function,
                            alloca_block,
                            current_block,
                            builder.borrow_context(),
                        )?;

                        current_block = exit_block;
                        builder = current_block.create_instructions(context);

                        /*
                        This is for a for statement

                        let identifier = self.get_current_str();
                        let identifier_range = self.get_current_range();

                        self.push_scope();

                        self.expect_symbol(Token::Colon)?;

                        let location = self.get_current_location(builder.borrow_context());

                        let ty = self.parse_type(None, builder.borrow_context())?;

                        let stack_alloc = {
                            let mut builder =
                                alloca_block.create_instructions(builder.borrow_context());

                            let alloc = builder.stack_alloc(identifier, ty, location);

                            builder.pause_building();

                            alloc
                        };

                        self.add_identifier(identifier, identifier_range, ty, stack_alloc)?;

                        self.expect_symbol(Token::In)?;

                        let start = self.parse_expression(ty, &mut builder)?;

                        self.expect_symbol(Token::Range)?;

                        let end = self.parse_expression(ty, &mut builder)?;

                        self.expect_symbol(Token::LCurly)?;*/
                    }
                    Token::Break => {
                        let location =
                            self.get_location(token_loc.location, builder.borrow_context());

                        let merge_block = if let Some(merge_block) = self.merge_blocks.last() {
                            *merge_block
                        } else {
                            panic!("Don't have a merge block (break wasn't in a loop).");
                        };

                        builder.branch(merge_block, &[], location);

                        self.expect_symbol(Token::Semicolon)?;

                        self.expect_symbol(Token::RCurly)?;

                        return Ok((entry_block, current_block));
                    }
                    Token::Continue => {
                        let location =
                            self.get_location(token_loc.location, builder.borrow_context());

                        let continue_block =
                            if let Some(continue_block) = self.continue_blocks.last() {
                                *continue_block
                            } else {
                                panic!("Don't have a continnue block (continue wasn't in a loop).");
                            };

                        builder.branch(continue_block, &[], location);

                        self.expect_symbol(Token::Semicolon)?;

                        self.expect_symbol(Token::RCurly)?;

                        return Ok((entry_block, current_block));
                    }
                    Token::Else => {
                        // We parse this in the if statement parsing, so if we find one here, its hanging around with no if!
                        return Err(ParseError::ElseStatementWithoutIf(token_loc.location));
                    }
                    token => panic!(
                        "Unhandled {:?} '{}'",
                        token,
                        self.file.source_slice(token_loc.location)
                    ),
                }
            } else {
                return Err(ParseError::UnexpectedEndOfFile);
            }
        }
    }

    fn parse_function(
        &mut self,
        identifier: &str,
        context: &mut yair::Context,
    ) -> Result<yair::Type, ParseError> {
        self.expect_symbol(Token::LParen)?;

        let mut args = Vec::new();

        let mut parsed_one_arg = false;

        loop {
            let token_loc = self.lexer.next();

            if let Some(token_loc) = token_loc {
                match token_loc.token {
                    Token::RParen => break,
                    Token::Identifier => {
                        let name = Name::new(self.file.clone(), token_loc.location);

                        self.expect_symbol(Token::Colon)?;

                        // TODO: We should check that we aren't parsing a function definition again here!
                        let ty = self.parse_type(None, context)?;

                        args.push((name, token_loc.location, ty));

                        parsed_one_arg = true;
                    }
                    Token::Comma => {
                        if !parsed_one_arg {
                            return Err(ParseError::ExpectedTokenNotFound(
                                Token::Comma,
                                token_loc.location,
                            ));
                        }
                    }
                    _ => return Err(ParseError::InvalidExpression(token_loc.location)),
                }
            } else {
                return Err(ParseError::UnexpectedEndOfFile);
            }
        }

        self.expect_symbol(Token::Colon)?;

        let return_ty = self.parse_type(None, context)?;

        let module = if let Some(module) = context
            .get_modules()
            .find(|m| m.get_name(context).as_str(context) == self.package)
        {
            module
        } else {
            context.create_module().with_name(&self.package).build()
        };

        let used_args: Vec<_> = args
            .iter()
            .map(|(name, _, ty)| (name.as_str().to_string(), ty))
            .collect();

        let mut function_builder = module
            .create_function(context)
            .with_name(identifier)
            .with_return_type(return_ty);

        for arg in used_args {
            function_builder = function_builder.with_arg(arg.0.as_str(), *arg.1);
        }

        let function = function_builder.build();

        self.functions.insert(identifier.to_string(), function);

        if self
            .lexer
            .peek()
            .map_or(Token::Error, |token_loc| token_loc.token)
            != Token::LCurly
        {
            panic!("TODO: Support function declarations!");
        }

        let arg_types: Vec<yair::Type> = function
            .get_args(context)
            .map(|v| v.get_type(context))
            .collect();

        let mut builder = function.create_block(context);

        for arg in arg_types {
            builder = builder.with_arg(arg);
        }

        let alloca_block = builder.build();

        self.push_scope();

        let mut entry_block_builder = alloca_block.create_instructions(context);

        for arg in args.iter().enumerate() {
            let (index, (name, range, ty)) = arg;

            let value = alloca_block.get_arg(entry_block_builder.borrow_context(), index);

            let location = value.get_location(entry_block_builder.borrow_context());
            let stack_alloc = entry_block_builder.stack_alloc(name.as_str(), *ty, location);

            entry_block_builder.store(stack_alloc, value, location);

            self.add_identifier(name.clone(), *range, *ty, stack_alloc)?;
        }

        entry_block_builder.pause_building();

        let return_is_void = return_ty.is_void(context);

        self.expect_symbol(Token::LCurly)?;
        let (entry_block, exit_block) = self.parse_block(function, alloca_block, context)?;

        if return_is_void {
            exit_block.create_instructions(context).ret(None);
        }

        alloca_block
            .create_instructions(context)
            .branch(entry_block, &[], None);

        Ok(function.get_type(context))
    }

    fn parse_identifier(&mut self) -> Result<String, ParseError> {
        let location = self.expect_symbol(Token::Identifier)?;
        Ok(self.file.source_slice(location).to_string())
    }

    fn parse_type(
        &mut self,
        name: Option<&str>,
        context: &mut yair::Context,
    ) -> Result<yair::Type, ParseError> {
        let identifier = self.parse_identifier()?;

        let mut result = match identifier.as_str() {
            "function" => self.parse_function(name.unwrap(), context)?,
            "void" => context.get_void_type(),
            "bool" => context.get_bool_type(),
            "i8" => context.get_int_type(8),
            "i16" => context.get_int_type(16),
            "i32" => context.get_int_type(32),
            "i64" => context.get_int_type(64),
            "u8" => context.get_uint_type(8),
            "u16" => context.get_uint_type(16),
            "u32" => context.get_uint_type(32),
            "u64" => context.get_uint_type(64),
            "f16" => context.get_float_type(16),
            "f32" => context.get_float_type(32),
            "f64" => context.get_float_type(64),
            _ => panic!("Unknown type identifier {}", identifier),
        };

        // If we've got an array, parse that now.
        while self.bump_if_symbol(Token::LBracket) {
            let location = self.expect_symbol(Token::Integer)?;

            let length = self.parse_integer(self.file.source_slice(location))?;

            result = context.get_array_type(result, length);

            self.expect_symbol(Token::RBracket)?;
        }

        Ok(result)
    }

    pub fn parse_package_and_import(&mut self) -> Result<(), ParseError> {
        if self.bump_if_symbol(Token::Package) {
            let location = self.expect_symbol(Token::String)?;

            let str = self.file.source_slice(location);
            self.package = str[1..(str.len() - 1)].to_string();
            // TODO: Check for bad package names?
        }

        while self.bump_if_symbol(Token::Import) {
            let location = self.expect_symbol(Token::String)?;

            let str = self.file.source_slice(location);
            self.imports.push(str[1..(str.len() - 1)].to_string());
            // TODO: Check for bad package names?
        }

        Ok(())
    }

    pub fn parse(&mut self, context: &mut yair::Context) -> Result<(), ParseError> {
        let identifier = match self.parse_identifier() {
            Ok(i) => i,
            Err(ParseError::UnexpectedEndOfFile) => return Ok(()),
            Err(e) => return Err(e),
        };

        self.expect_symbol(Token::Colon)?;

        let _ty = self.parse_type(Some(&identifier), context)?;

        Ok(())
    }

    pub fn display_error(
        &self,
        e: ParseError,
        context: &mut yair::Context,
        fmt: &mut std::io::Stderr,
    ) -> Result<(), io::Error> {
        let write_range = |fmt: &mut std::io::Stderr, span: Location| -> Result<(), io::Error> {
            let location = self.codemap.look_up_span(span);

            writeln!(
                fmt,
                "{}:{}:{}",
                location.file.name(),
                location.begin.line + 1,
                location.begin.column + 1
            )?;

            let pos = span.low();

            let line = self.file.find_line(pos);

            let str = self.file.source_line(line);

            writeln!(fmt, "  {}", str)?;

            let line_col = self.file.find_line_col(pos);

            writeln!(fmt, "  {}^", " ".repeat(line_col.column))
        };

        match e {
            ParseError::UnexpectedEndOfFile => {
                writeln!(fmt, "error: Unexpected end of file")?;
                write_range(fmt, self.end_of_file_location())?;
            }
            ParseError::ExpectedTokenNotFound(token, range) => {
                writeln!(fmt, "error: Expected token '{:?}' not found", token)?;
                write_range(fmt, range)?;
            }
            ParseError::InvalidExpression(range) => {
                writeln!(fmt, "error: Invalid expression")?;
                write_range(fmt, range)?;
            }
            ParseError::OperatorsInDifferentPrecedenceGroups(x_location, _, y_location, _) => {
                let x_str = self.file.source_slice(x_location);
                let y_str = self.file.source_slice(y_location);

                writeln!(
                    fmt,
                    "error: Operators '{}' and '{}' are in different precedence groups",
                    x_str, y_str
                )?;
                write_range(fmt, x_location)?;
                write_range(fmt, y_location)?;
            }
            ParseError::TypesDoNotMatch(x_range, x_ty, y_range, y_ty) => {
                writeln!(
                    fmt,
                    "error: Types '{}' and '{}' do not match",
                    x_ty.get_displayer(context),
                    y_ty.get_displayer(context),
                )?;
                write_range(fmt, x_range)?;
                write_range(fmt, y_range)?;
            }
            ParseError::InvalidNonConcreteConstantsUsed(x_range, y_range) => {
                writeln!(fmt, "error: Invalid non-concrete constant used")?;
                write_range(fmt, x_range)?;
                write_range(fmt, y_range)?;
            }
            ParseError::InvalidNonConcreteConstantUsed(range) => {
                writeln!(fmt, "error: Invalid non-concrete constant used")?;
                write_range(fmt, range)?;
            }
            ParseError::UnknownIdentifier(location) => {
                let str = self.file.source_slice(location);

                writeln!(fmt, "error: Unknown identifier '{}' used", str)?;
                write_range(fmt, location)?;
            }
            ParseError::ComparisonOperatorsAlwaysNeedParenthesis(x_location, _, y_location, _) => {
                let x_str = self.file.source_slice(x_location);
                let y_str = self.file.source_slice(y_location);

                writeln!(
                    fmt,
                    "error: Comparison operators always need parenthesis in expressions: '{}' and '{}'",
                    x_str, y_str
                )?;

                write_range(fmt, x_location)?;
                write_range(fmt, y_location)?;
            }
            ParseError::UnknownIdentifierUsedInAssignment(location) => {
                let str = self.file.source_slice(location);

                writeln!(
                    fmt,
                    "error: Unknown identifier '{}' used in assignment",
                    str
                )?;
                writeln!(
                    fmt,
                    "       Did you mean to declare the variable `{} : <type> =` instead?",
                    str
                )?;
                write_range(fmt, location)?;
            }
            ParseError::IdentifierShadowsPreviouslyDeclareIdentifier(x_location, y_location) => {
                let y_str = self.file.source_slice(y_location);

                writeln!(
                    fmt,
                    "error: Identifier '{}' shadows a previously declared identifier:",
                    y_str
                )?;

                write_range(fmt, y_location)?;
                write_range(fmt, x_location)?;
            }
            ParseError::ElseStatementWithoutIf(range) => {
                writeln!(fmt, "error: Else statement without an if")?;
                write_range(fmt, range)?;
            }
        }

        Ok(())
    }
}

fn main() {
    let yaml = load_yaml!("bootstrap.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let inputs = matches.values_of("input").unwrap();

    let mut parsers = Vec::new();

    let mut data = String::new();
    let mut slices = Vec::new();

    for input in inputs {
        let start = data.len();

        if input == "-" {
            io::stdin().read_to_string(&mut data).unwrap();
        } else {
            let mut file = if let Ok(file) = File::open(input) {
                file
            } else {
                panic!("Could not open file '{}'", input);
            };
            file.read_to_string(&mut data).unwrap();
        }

        let end = data.len();

        slices.push(start..end);
    }

    let exported_package_functions: ExportedPackageFunctions =
        Rc::new(RefCell::new(HashMap::new()));

    for (index, input) in matches.values_of("input").unwrap().enumerate() {
        parsers.push(Parser::new(
            exported_package_functions.clone(),
            input.to_string(),
            &data[slices[index].clone()],
        ));
    }

    let mut context = yair::Context::new();

    // Parse the packages and imports first
    for parser in &mut parsers {
        if let Err(e) = parser.parse_package_and_import() {
            parser
                .display_error(e, &mut context, &mut std::io::stderr())
                .unwrap();
            std::process::exit(1);
        }
    }

    // Sort the parsers now so that we process them in order
    parsers.sort();

    // Actually parse the package now
    for parser in &mut parsers {
        if let Err(e) = parser.parse(&mut context) {
            parser
                .display_error(e, &mut context, &mut std::io::stderr())
                .unwrap();
            std::process::exit(1);
        }

        if !parser.package.is_empty() {
            if !exported_package_functions
                .borrow()
                .contains_key(&parser.package)
            {
                exported_package_functions
                    .borrow_mut()
                    .insert(parser.package.clone(), HashMap::new());
            }

            for (name, function) in &parser.functions {
                let function_with_package_name = parser.package.to_string() + "::" + name;

                let mut borrow_mut = exported_package_functions.borrow_mut();

                let per_package_map = borrow_mut.get_mut(&parser.package).unwrap();

                if per_package_map
                    .insert(function_with_package_name, *function)
                    .is_some()
                {
                    // We had a package conflict (two files declaring the same package declared the same function!)
                    panic!("Two packages declared the same function!");
                }
            }
        }
    }

    if let Err(e) = context.verify() {
        eprintln!("Verification error: {}", e);
        std::process::exit(1);
    }

    let emit = matches.value_of("emit").unwrap();

    match emit {
        "yair" => {
            let output = matches.value_of("output").unwrap();

            if output == "-" {
                disassemble(&context, io::stdout().lock())
            } else {
                let file = File::create(output).unwrap();
                disassemble(&context, file)
            }
            .expect("Could not write data");
        }
        _ => {
            todo!()
        }
    }
}
