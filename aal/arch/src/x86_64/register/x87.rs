//! 80-bit x87 register values split into significand and exponent/sign portions.

use core::fmt;

use nekor_aal_agnostic::partitioned::Partitioned;
use nekor_bitwise::prelude::{Bit, Counterpart, Field, State};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unalign, Unaligned};

/// The significand portion of an x87 register image.
pub type Significand = Partitioned<0, 63, u128>;

/// The exponent and sign portion of an x87 register image.
pub type SignExponent = Partitioned<64, 79, u128>;

/// The biased exponent in the upper register portion.
pub type Exponent<'a> = Field<'a, 0, 14, u16>;

/// The sign bit in the upper register portion.
pub type Sign<'a> = Bit<'a, u16, 15>;

/// Mutable counterpart of [`Exponent`].
pub type ExponentMut<'a> = <Exponent<'a> as Counterpart>::Mut;

/// Mutable counterpart of [`Sign`].
pub type SignMut<'a> = <Sign<'a> as Counterpart>::Mut;

/// An 80-bit x87 register image with byte alignment and no save-slot padding.
///
/// Mutable access uses [`Unalign`] because either portion may be at an unaligned address.
#[derive(Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout, Unaligned)]
#[repr(C)]
// NOTE(invariant): None.
pub struct Sti(Unalign<Significand>, Unalign<SignExponent>);

impl Sti {
    /// Construct a register image from its architectural portions.
    #[inline]
    #[must_use]
    pub const fn new(significand: Significand, sign_exponent: SignExponent) -> Self {
        let significand = Unalign::new(significand);
        let sign_exponent = Unalign::new(sign_exponent);

        Self(significand, sign_exponent)
    }

    /// Read the significand without requiring aligned backing.
    #[inline]
    #[must_use]
    pub const fn significand(&self) -> Significand {
        let &Self(target_value, ..) = self;

        target_value.into_inner()
    }

    /// Mutably borrow the byte-aligned significand storage.
    #[inline]
    pub const fn significand_mut(&mut self) -> &mut Unalign<Significand> {
        let Self(target_value, ..) = self;

        target_value
    }

    /// Read the exponent and sign without requiring aligned backing.
    #[inline]
    #[must_use]
    pub const fn sign_exponent(&self) -> SignExponent {
        let &Self(.., target_value) = self;

        target_value.into_inner()
    }

    /// Mutably borrow the byte-aligned exponent and sign storage.
    #[inline]
    pub const fn sign_exponent_mut(&mut self) -> &mut Unalign<SignExponent> {
        let Self(.., target_value) = self;

        target_value
    }

    /// Borrow the biased exponent field.
    #[inline]
    #[must_use]
    pub const fn exponent(&self) -> u16 {
        let target_value = self.sign_exponent();

        Exponent::wrap(target_value.value()).const_value()
    }

    /// Sign bit.
    #[inline]
    #[must_use]
    pub const fn sign(&self) -> State {
        let target_value = self.sign_exponent();

        Sign::wrap(target_value.value()).const_state()
    }
}

impl PartialEq for Sti {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.significand() == other.significand() && self.sign_exponent() == other.sign_exponent()
    }
}

impl Eq for Sti {}

impl fmt::Debug for Sti {
    #[inline]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Sti")
            .field("significand", &self.significand())
            .field("sign_exponent", &self.sign_exponent())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use core::mem;

    use nekor_bitwise::prelude::State;
    use zerocopy::{FromBytes, IntoBytes};

    use crate::x86_64::register::x87::{ExponentMut, SignExponent, SignMut, Significand, Sti};

    /// Place a ten-byte architectural value at an address unaligned for either word.
    #[repr(C, align(8))]
    // NOTE(invariant): None.
    struct Misaligned {
        /// Sentinel preceding the register image.
        prefix: u8,

        /// Byte-aligned architectural image.
        register: Sti,
    }

    #[test]
    fn architectural_register_has_no_save_slot_padding() {
        let significand = Significand::raw(0x0123_4567_89ab_cdef);
        let sign_exponent = SignExponent::raw(0);
        let mut register = Sti::new(significand, sign_exponent);

        register.sign_exponent_mut().update(|target_value| {
            ExponentMut::wrap(target_value.value_mut()).const_merge(0x2345);
            SignMut::wrap(target_value.value_mut()).const_set(State::Set);
        });

        assert_eq!(mem::size_of::<Sti>(), 10);
        assert_eq!(mem::align_of::<Sti>(), 1);
        assert_eq!(*register.significand().value(), 0x0123_4567_89ab_cdef);
        assert_eq!(register.exponent(), 0x2345);
        assert_eq!(register.sign(), State::Set);
        assert_eq!(
            Sti::read_from_bytes(register.as_bytes()).expect("raw x87 value"),
            register
        );
    }
}
