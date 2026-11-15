//! XMM, YMM and ZMM register values with access to their lower and upper halves.

use nekor_bitwise::prelude::{Counterpart, Field};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// The lower 64 bits of an XMM register.
pub type XmmLow64<'a> = Field<'a, 0, 63, u128>;

/// The upper 64 bits of an XMM register.
pub type XmmHigh64<'a> = Field<'a, 64, 127, u128>;

/// Mutable counterpart of [`XmmLow64`].
pub type XmmLow64Mut<'a> = <XmmLow64<'a> as Counterpart>::Mut;

/// Mutable counterpart of [`XmmHigh64`].
pub type XmmHigh64Mut<'a> = <XmmHigh64<'a> as Counterpart>::Mut;

/// A raw 128-bit XMM register.
#[derive(Debug, Clone, Copy, Eq, PartialEq, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(transparent)]
// NOTE(invariant): None.
pub struct Xmm(u128);

impl Xmm {
    /// Construct an XMM register image from its scalar bits.
    #[inline]
    #[must_use]
    pub const fn new(target_value: u128) -> Self {
        Self(target_value)
    }

    /// Return the scalar register image.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u128 {
        let Self(target_value) = self;

        target_value
    }

    /// Borrow the lower 64-bit field.
    #[inline]
    #[must_use]
    pub const fn low64(&self) -> XmmLow64<'_> {
        let Self(target_value) = self;

        XmmLow64::wrap(target_value)
    }

    /// Mutably borrow the lower 64-bit field.
    #[inline]
    pub const fn low64_mut(&mut self) -> XmmLow64Mut<'_> {
        let Self(target_value) = self;

        XmmLow64Mut::wrap(target_value)
    }

    /// Borrow the upper 64-bit field.
    #[inline]
    #[must_use]
    pub const fn high64(&self) -> XmmHigh64<'_> {
        let Self(target_value) = self;

        XmmHigh64::wrap(target_value)
    }

    /// Mutably borrow the upper 64-bit field.
    #[inline]
    pub const fn high64_mut(&mut self) -> XmmHigh64Mut<'_> {
        let Self(target_value) = self;

        XmmHigh64Mut::wrap(target_value)
    }
}

/// A 256-bit YMM register image.
#[derive(Debug, Clone, Copy, Eq, PartialEq, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
// NOTE(invariant): None.
pub struct Ymm(Xmm, Xmm);

impl Ymm {
    /// Combine the lower and upper 128-bit portions.
    #[inline]
    #[must_use]
    pub const fn new(low: Xmm, high: Xmm) -> Self {
        Self(low, high)
    }

    /// Borrow the lower XMM subregister.
    #[inline]
    #[must_use]
    pub const fn low(&self) -> &Xmm {
        let Self(low, ..) = self;

        low
    }

    /// Mutably borrow the lower XMM subregister.
    #[inline]
    pub const fn low_mut(&mut self) -> &mut Xmm {
        let Self(low, ..) = self;

        low
    }

    /// Borrow the upper 128-bit portion.
    #[inline]
    #[must_use]
    pub const fn high(&self) -> &Xmm {
        let Self(.., high) = self;

        high
    }

    /// Mutably borrow the upper 128-bit portion.
    #[inline]
    pub const fn high_mut(&mut self) -> &mut Xmm {
        let Self(.., high) = self;

        high
    }
}

/// A 512-bit ZMM register image.
#[derive(Debug, Clone, Copy, Eq, PartialEq, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
// NOTE(invariant): None.
pub struct Zmm(Ymm, Ymm);

impl Zmm {
    /// Combine the lower and upper 256-bit portions.
    #[inline]
    #[must_use]
    pub const fn new(low: Ymm, high: Ymm) -> Self {
        Self(low, high)
    }

    /// Borrow the lower YMM subregister.
    #[inline]
    #[must_use]
    pub const fn low(&self) -> &Ymm {
        let Self(low, ..) = self;

        low
    }

    /// Mutably borrow the lower YMM subregister.
    #[inline]
    pub const fn low_mut(&mut self) -> &mut Ymm {
        let Self(low, ..) = self;

        low
    }

    /// Borrow the upper 256-bit portion.
    #[inline]
    #[must_use]
    pub const fn high(&self) -> &Ymm {
        let Self(.., high) = self;

        high
    }

    /// Mutably borrow the upper 256-bit portion.
    #[inline]
    pub const fn high_mut(&mut self) -> &mut Ymm {
        let Self(.., high) = self;

        high
    }
}

#[cfg(test)]
mod tests {
    use core::mem;

    use zerocopy::{FromBytes, IntoBytes};

    use crate::x86_64::register::vector::{Xmm, Ymm, Zmm};

    #[test]
    fn registers_have_their_architectural_extents() {
        assert_eq!(mem::size_of::<Xmm>(), 16);
        assert_eq!(mem::size_of::<Ymm>(), 32);
        assert_eq!(mem::size_of::<Zmm>(), 64);
    }

    #[test]
    fn named_fields_and_wide_portions_preserve_independent_values() {
        let mut xmm = Xmm::new(0);

        xmm.low64_mut().const_merge(u64::MAX);
        xmm.high64_mut().const_merge(0x0123_4567_89ab_cdef);

        assert_eq!(xmm.low64().const_value(), u64::MAX);
        assert_eq!(xmm.high64().const_value(), 0x0123_4567_89ab_cdef);

        let high = Xmm::new(3);
        let ymm = Ymm::new(xmm, high);
        let high = Ymm::new(Xmm::new(5), Xmm::new(7));
        let zmm = Zmm::new(ymm, high);
        let decoded = Zmm::read_from_bytes(zmm.as_bytes()).expect("raw register bytes");

        assert_eq!(decoded, zmm);
        assert_eq!(decoded.low().low(), &xmm);
        assert_eq!(decoded.low().high().raw(), 3);
    }
}
