//! Dense position newtype.
//!
//! A position is a `0..n` index into a problem's city list — a matrix row, a point index.
//! City IDs are external identifiers and may be sparse (TSPLIB 1-based, arbitrary API input).
//! The two spaces are both `usize` at the storage layer, so the newtype is what keeps one
//! from being passed where the other is expected.

/// Dense `0..n` index into a problem's city list.
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Position(usize);

impl Position {
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    pub const fn into_inner(self) -> usize {
        self.0
    }
}

impl From<usize> for Position {
    fn from(index: usize) -> Self {
        Self(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_is_memory_neutral() {
        assert_eq!(
            core::mem::size_of::<Position>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::align_of::<Position>(),
            core::mem::align_of::<usize>()
        );
    }

    #[test]
    fn position_round_trips() {
        let pos = Position::new(7);
        assert_eq!(pos.into_inner(), 7);
        assert_eq!(Position::from(7), pos);
        assert!(Position::new(1) < Position::new(2));
    }
}
