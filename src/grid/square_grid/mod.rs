use super::{Size, WrapFlags};

pub mod square;

#[derive(Clone, Copy)]
pub struct SquareGrid {
    pub size: Size,
    pub wrap_flags: WrapFlags,
}
