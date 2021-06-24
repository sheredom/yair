use crate::linkgen::*;
use std::io::{Read, Seek, Write};

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
}

impl From<std::io::Error> for Error {
    fn from(io_error: std::io::Error) -> Self {
        Error::Io(io_error)
    }
}

pub struct Lld {}

impl LinkGen for Lld {
    type Error = Error;

    fn generate<R: Read, W: Seek + Write>(
        _: &R,
        _: LinkGenOutput,
        _: &mut W,
    ) -> Result<(), Self::Error> {
        todo!()
    }
}
