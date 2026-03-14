//! Bit-wise state.

use core::ops::Not;

/// A bitwise state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum State {
    /// A non-set state, i.e. a bit is set to `0`.
    ///
    /// This is the default state.
    #[default]
    Cleared = 0b0000_0000,

    /// A set state, i.e. a bit is set to `1`.
    Set = 0b0000_0001,
}

impl State {
    /// Convert a [`State`] into a [`bool`].
    #[inline]
    #[must_use]
    pub const fn bool(self) -> bool {
        match self {
            Self::Set => true,
            Self::Cleared => false,
        }
    }

    /// Determine the complement of to the current [`State`].
    #[inline]
    #[must_use]
    pub const fn complement(&self) -> Self {
        match self {
            Self::Set => Self::Cleared,
            Self::Cleared => Self::Set,
        }
    }
}

impl Not for State {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        match self {
            Self::Set => Self::Cleared,
            Self::Cleared => Self::Set,
        }
    }
}
