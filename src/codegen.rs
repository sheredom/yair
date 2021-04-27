use crate::Library;
use std::io::{Seek, Write};

pub enum CodeGenPlatform {
    MacOsAppleSilicon,
}

pub trait CodeGen {
    type Error;

    fn generate<W: Seek + Write>(
        library: &Library,
        platform: CodeGenPlatform,
        writer: &mut W,
    ) -> Result<(), Self::Error>;
}
