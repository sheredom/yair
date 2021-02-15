use crate::*;

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
#[cfg_attr(feature = "io", derive(Serialize, Deserialize))]
pub struct Location {
    pub(crate) filename: Name,
    pub(crate) line: usize,
    pub(crate) column: usize,
}

impl Location {
    pub fn get_line(&self) -> usize {
        self.line
    }

    pub fn get_column(&self) -> usize {
        self.column
    }
}

impl Named for Location {
    fn get_name<'a>(&self, library: &'a Library) -> &'a str {
        &library.names[self.filename.0]
    }
}
