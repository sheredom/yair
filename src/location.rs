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

    pub fn get_displayer<'a>(&self, library: &'a Library) -> LocationDisplayer<'a> {
        LocationDisplayer {
            loc: *self,
            library,
        }
    }
}

impl Named for Location {
    fn get_name(&self, _: &Library) -> Name {
        self.filename
    }
}

pub struct LocationDisplayer<'a> {
    pub(crate) loc: Location,
    pub(crate) library: &'a Library,
}

impl<'a> std::fmt::Display for LocationDisplayer<'a> {
    fn fmt(
        &self,
        writer: &mut std::fmt::Formatter<'_>,
    ) -> std::result::Result<(), std::fmt::Error> {
        write!(
            writer,
            " !{}:{:?}:{:?}",
            self.loc.get_name(self.library).get_displayer(self.library),
            self.loc.get_line(),
            self.loc.get_column(),
        )
    }
}
