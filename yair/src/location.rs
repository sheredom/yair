use crate::*;

#[derive(Clone, Copy, Debug)]
pub struct Filename(pub(crate) generational_arena::Index);

#[derive(Clone, Copy, Debug)]
pub struct Location {
    pub(crate) filename: Filename,
    pub(crate) start: (usize, usize),
    pub(crate) end: (usize, usize),
}
