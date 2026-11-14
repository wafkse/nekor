//! x87 data-register images in their architectural 16-byte save slots.
//!
//! The low 80 bits describe the extended-precision value. The remaining six bytes belong
//! to the save slot and carry no floating-point meaning.

use nekor_aal_agnostic::partitioned::Partitioned;
use nekor_bitwise::prelude::{Bit, Counterpart, Field};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// The six architecturally reserved bytes following an 80-bit x87 save-slot value.
type X87SlotReserved = [u8; 6];

/// The eight x87 data-register slots in a legacy save image.
pub type X87Bank = [X87; 8];

/// The significand portion of an x87 register image.
pub type Significand = Partitioned<0, 63, u128>;

/// The exponent and sign portion of an x87 register image.
pub type SignExponent = Partitioned<64, 79, u128>;

/// The biased exponent in the upper register portion.
pub type Exponent<'target_value> = Field<'target_value, 0, 14, u16>;

/// The sign bit in the upper register portion.
pub type Sign<'target_value> = Bit<'target_value, u16, 15>;

/// Mutable counterpart of [`Exponent`].
pub type ExponentMut<'target_value> = <Exponent<'target_value> as Counterpart>::Mut;

/// Mutable counterpart of [`Sign`].
pub type SignMut<'target_value> = <Sign<'target_value> as Counterpart>::Mut;

/// An x87 register in its 16-byte FXSAVE or XSAVE slot.
#[derive(Debug, Clone, Copy, Eq, PartialEq, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
// NOTE(invariant): None.
pub struct X87(Significand, SignExponent, X87SlotReserved);

impl X87 {
    /// Construct an image from its architectural register portions.
    #[inline]
    #[must_use]
    pub const fn new(significand: Significand, exponent: SignExponent) -> Self {
        Self(significand, exponent, [0; core::mem::size_of::<X87SlotReserved>()])
    }

    /// Borrow the significand portion.
    #[inline]
    #[must_use]
    pub const fn significand(&self) -> &Significand {
        let Self(target_value, ..) = self;

        target_value
    }

    /// Mutably borrow the significand portion.
    #[inline]
    pub const fn significand_mut(&mut self) -> &mut Significand {
        let Self(target_value, ..) = self;

        target_value
    }

    /// Borrow the biased exponent field.
    #[inline]
    #[must_use]
    pub const fn exponent(&self) -> Exponent<'_> {
        let Self(_, target_value, ..) = self;

        Exponent::wrap(target_value.value())
    }

    /// Mutably borrow the biased exponent field.
    #[inline]
    pub const fn exponent_mut(&mut self) -> ExponentMut<'_> {
        let Self(_, target_value, ..) = self;

        ExponentMut::wrap(target_value.value_mut())
    }

    /// Borrow the sign field.
    #[inline]
    #[must_use]
    pub const fn sign(&self) -> Sign<'_> {
        let Self(_, target_value, ..) = self;

        Sign::wrap(target_value.value())
    }

    /// Mutably borrow the sign field.
    #[inline]
    pub const fn sign_mut(&mut self) -> SignMut<'_> {
        let Self(_, target_value, ..) = self;

        SignMut::wrap(target_value.value_mut())
    }
}

#[cfg(test)]
mod tests {
    use core::mem;

    use nekor_bitwise::prelude::State;
    use zerocopy::{FromBytes, IntoBytes};

    use super::{SignExponent, Significand, X87, X87Bank};

    #[test]
    fn x87_partitions_preserve_the_saved_register_slot() {
        let significand = Significand::raw(0x0123_4567_89ab_cdef);
        let exponent = SignExponent::raw(0);
        let mut register = X87::new(significand, exponent);

        register.exponent_mut().const_merge(0x2345);
        register.sign_mut().const_set(State::Set);

        assert_eq!(mem::size_of::<X87>(), 16);
        assert_eq!(mem::size_of::<X87Bank>(), 128);
        assert_eq!(*register.significand().value(), 0x0123_4567_89ab_cdef);
        assert_eq!(register.exponent().const_value(), 0x2345);
        assert_eq!(register.sign().const_state(), State::Set);
        assert_eq!(
            X87::read_from_bytes(register.as_bytes()).expect("raw x87 slot"),
            register
        );
    }
}
