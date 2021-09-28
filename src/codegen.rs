use crate::Context;
use std::io::{Seek, Write};
use std::str::FromStr;

#[derive(Copy, Clone)]
pub enum CodeGenOutput {
    Object,
    Assembly,
    Intermediate,
}

impl FromStr for CodeGenOutput {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "Object" => Ok(CodeGenOutput::Object),
            "Assembly" => Ok(CodeGenOutput::Assembly),
            "Intermediate" => Ok(CodeGenOutput::Intermediate),
            _ => Err(()),
        }
    }
}

pub trait CodeGen {
    type Error;

    fn generate<W: Seek + Write>(
        context: &Context,
        triple: &str,
        output: CodeGenOutput,
        writer: &mut W,
    ) -> Result<(), Self::Error>;
}
