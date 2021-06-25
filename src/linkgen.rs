use std::io::{Read, Seek, Write};
use std::str::FromStr;

#[derive(Copy, Clone)]
pub enum LinkGenOutput {
    Dynamic,
}

impl FromStr for LinkGenOutput {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "Dynamic" => Ok(LinkGenOutput::Dynamic),
            _ => Err(()),
        }
    }
}

pub trait LinkGen {
    type Error;

    fn generate<R: Read, W: Seek + Write>(
        reader: &R,
        output: LinkGenOutput,
        writer: &mut W,
    ) -> Result<(), Self::Error>;
}
