//! x86 guest entry values.

use core::{convert::TryFrom, num::TryFromIntError};

use eustace_address::gpa::Gpa;

/// Guest instruction address representable by a real-mode instruction pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): Every 16-bit value is a valid real-mode instruction address.
pub struct RealModeEntryPoint(u16);

impl RealModeEntryPoint {
    /// Preserve a 16-bit real-mode instruction address.
    #[inline]
    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Return the real-mode instruction address.
    #[inline]
    #[must_use]
    pub const fn value(self) -> u16 {
        let Self(value) = self;

        value
    }
}

impl TryFrom<Gpa> for RealModeEntryPoint {
    type Error = TryFromIntError;

    #[inline]
    fn try_from(value: Gpa) -> Result<Self, Self::Error> {
        u16::try_from(value.value()).map(Self)
    }
}
