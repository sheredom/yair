use crate::*;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Location {
    pub(crate) filename: Name,
    pub(crate) start: (usize, usize),
    pub(crate) end: (usize, usize),
}
