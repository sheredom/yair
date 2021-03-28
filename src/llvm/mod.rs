use crate::*;
use std::io::{Seek, Write};

#[allow(clippy::upper_case_acronyms)]
pub struct LLVM {}

pub enum Error {}

impl CodeGen for LLVM {
    type Error = Error;

    fn generate<W: Seek + Write>(_library: &Library, _writer: &mut W) -> Result<(), Self::Error> {
        todo!();
    }
}
