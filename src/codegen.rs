use crate::Library;
use std::io::{Seek, Write};
use std::str::FromStr;

#[derive(Copy, Clone)]
pub enum CodeGenPlatform {
    Windows64Bit,
    MacOsAppleSilicon,
}

impl FromStr for CodeGenPlatform {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "Windows64Bit" => Ok(CodeGenPlatform::Windows64Bit),
            "MacOsAppleSilicon" => Ok(CodeGenPlatform::MacOsAppleSilicon),
            _ => Err(()),
        }
    }
}

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
        library: &Library,
        platform: CodeGenPlatform,
        output: CodeGenOutput,
        writer: &mut W,
    ) -> Result<(), Self::Error>;
}
