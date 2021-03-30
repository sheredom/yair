use crate::*;
use std::io::{Seek, Write};

pub struct Llvm {}

pub enum Error {}

impl CodeGen for Llvm {
    type Error = Error;

    fn generate<W: Seek + Write>(_library: &Library, _writer: &mut W) -> Result<(), Self::Error> {
        todo!();
    }
}
