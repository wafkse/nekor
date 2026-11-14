//! Raw vector-register values and their architectural subregisters.
//!
//! Wider registers preserve the identity of their lower and upper portions. XSAVE stores
//! those portions in separate components, so each saved portion has its own representation.

use nekor_bitwise::prelude::{Counterpart, Field};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// XMM0 through XMM15 in the x86-64 legacy save region.
pub type XmmBank = [Xmm; 16];

/// Upper YMM0 through YMM15 portions in the AVX extended-state component.
pub type YmmHighBank = [YmmHigh; 16];

/// Two consecutive 128-bit portions covering bits 256 through 511 of a ZMM register.
pub type ZmmHighParts = [Xmm; 2];

/// The lower 64 bits of an XMM register.
pub type XmmLow64<'target_value> = Field<'target_value, 0, 63, u128>;

/// The upper 64 bits of an XMM register.
pub type XmmHigh64<'target_value> = Field<'target_value, 64, 127, u128>;

/// Mutable counterpart of [`XmmLow64`].
pub type XmmLow64Mut<'target_value> = <XmmLow64<'target_value> as Counterpart>::Mut;

/// Mutable counterpart of [`XmmHigh64`].
pub type XmmHigh64Mut<'target_value> = <XmmHigh64<'target_value> as Counterpart>::Mut;

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

/// The upper 128 bits of a YMM register, stored in the AVX XSAVE component.
#[derive(Debug, Clone, Copy, Eq, PartialEq, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(transparent)]
// NOTE(invariant): None.
pub struct YmmHigh(Xmm);

impl YmmHigh {
    /// Construct the upper portion from a 128-bit register image.
    #[inline]
    #[must_use]
    pub const fn new(target_value: Xmm) -> Self {
        Self(target_value)
    }

    /// Borrow the 128-bit portion.
    #[inline]
    #[must_use]
    pub const fn value(&self) -> &Xmm {
        let Self(target_value) = self;

        target_value
    }

    /// Mutably borrow the 128-bit portion.
    #[inline]
    pub const fn value_mut(&mut self) -> &mut Xmm {
        let Self(target_value) = self;

        target_value
    }
}

/// A raw 256-bit YMM register with its separately saved subregisters.
#[derive(Debug, Clone, Copy, Eq, PartialEq, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
// NOTE(invariant): None.
pub struct Ymm(Xmm, YmmHigh);

impl Ymm {
    /// Combine the lower XMM register and upper AVX portion.
    #[inline]
    #[must_use]
    pub const fn new(low: Xmm, high: YmmHigh) -> Self {
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

    /// Borrow the upper AVX portion.
    #[inline]
    #[must_use]
    pub const fn high(&self) -> &YmmHigh {
        let Self(.., high) = self;

        high
    }

    /// Mutably borrow the upper AVX portion.
    #[inline]
    pub const fn high_mut(&mut self) -> &mut YmmHigh {
        let Self(.., high) = self;

        high
    }
}

/// The upper 256 bits of a ZMM register, stored in the ZMM_Hi256 component.
#[derive(Debug, Clone, Copy, Eq, PartialEq, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(transparent)]
// NOTE(invariant): None.
pub struct ZmmHigh(ZmmHighParts);

impl ZmmHigh {
    /// Construct the high portion in increasing bit order.
    #[inline]
    #[must_use]
    pub const fn new(target_value: ZmmHighParts) -> Self {
        Self(target_value)
    }

    /// Borrow the two 128-bit portions in increasing bit order.
    #[inline]
    #[must_use]
    pub const fn value(&self) -> &ZmmHighParts {
        let Self(target_value) = self;

        target_value
    }

    /// Mutably borrow the two 128-bit portions.
    #[inline]
    pub const fn value_mut(&mut self) -> &mut ZmmHighParts {
        let Self(target_value) = self;

        target_value
    }
}

/// A raw 512-bit ZMM register with its separately saved portions.
#[derive(Debug, Clone, Copy, Eq, PartialEq, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
// NOTE(invariant): None.
pub struct Zmm(Ymm, ZmmHigh);

impl Zmm {
    /// Combine the lower YMM register and upper AVX-512 portion.
    #[inline]
    #[must_use]
    pub const fn new(low: Ymm, high: ZmmHigh) -> Self {
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

    /// Borrow the upper AVX-512 portion.
    #[inline]
    #[must_use]
    pub const fn high(&self) -> &ZmmHigh {
        let Self(.., high) = self;

        high
    }

    /// Mutably borrow the upper AVX-512 portion.
    #[inline]
    pub const fn high_mut(&mut self) -> &mut ZmmHigh {
        let Self(.., high) = self;

        high
    }
}

#[cfg(test)]
mod tests {
    use core::mem;

    use zerocopy::{FromBytes, IntoBytes};

    use super::{Xmm, XmmBank, Ymm, YmmHigh, YmmHighBank, Zmm, ZmmHigh};

    #[test]
    fn vector_portions_have_the_architectural_extents() {
        assert_eq!(mem::size_of::<Xmm>(), 16);
        assert_eq!(mem::size_of::<YmmHigh>(), 16);
        assert_eq!(mem::size_of::<Ymm>(), 32);
        assert_eq!(mem::size_of::<ZmmHigh>(), 32);
        assert_eq!(mem::size_of::<Zmm>(), 64);
        assert_eq!(mem::size_of::<XmmBank>(), 256);
        assert_eq!(mem::size_of::<YmmHighBank>(), 256);
    }

    #[test]
    fn named_fields_and_wide_portions_preserve_independent_values() {
        let mut xmm = Xmm::new(0);

        xmm.low64_mut().const_merge(u64::MAX);
        xmm.high64_mut().const_merge(0x0123_4567_89ab_cdef);

        assert_eq!(xmm.low64().const_value(), u64::MAX);
        assert_eq!(xmm.high64().const_value(), 0x0123_4567_89ab_cdef);

        let high = YmmHigh::new(Xmm::new(3));
        let ymm = Ymm::new(xmm, high);
        let high = ZmmHigh::new([Xmm::new(5), Xmm::new(7)]);
        let zmm = Zmm::new(ymm, high);
        let decoded = Zmm::read_from_bytes(zmm.as_bytes()).expect("raw register bytes");

        assert_eq!(decoded, zmm);
        assert_eq!(decoded.low().low(), &xmm);
        assert_eq!(decoded.low().high().value().raw(), 3);
    }
}
