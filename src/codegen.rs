use crate::Library;
use std::io::{Seek, Write};

pub trait CodeGen {
    type Error;

    fn generate<W: Seek + Write>(library: &Library, writer: &mut W) -> Result<(), Self::Error>;
}
