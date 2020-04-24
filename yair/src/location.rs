use crate::*;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Location {
    pub(crate) filename: Name,
    pub(crate) start: (usize, usize),
    pub(crate) end: (usize, usize),
}

impl Location {
    pub fn get_start(&self) -> (usize, usize) {
        self.start
    }

    pub fn get_end(&self) -> (usize, usize) {
        self.end
    }
}

impl Named for Location {
    fn get_name<'a>(&self, library: &'a Library) -> &'a str {
        &library.names[self.filename.0]
    }
}
