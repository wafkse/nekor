//! Register access gates.
//!
//! [`Gated`] combines a register value type, its numeric address, and two
//! compile-time access bits. It does not define how that address is accessed.

use core::marker;

use crate::address::Address;

/// A register description with compile-time read and write permissions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
// NOTE(invariant): `address` is the typed address associated with value type `T`
// while `READ` and `WRITE` remain compile-time access policy. Construction
// cannot change those type-level facts or attach runtime permission state.
pub struct Gated<T, A, const READ: bool, const WRITE: bool>
where
    A: Address,
{
    /// Numeric address naming the register.
    address: A,

    /// Register value-type marker without storage ownership.
    value: marker::PhantomData<fn() -> T>,
}

impl<T, A, const READ: bool, const WRITE: bool> Gated<T, A, READ, WRITE>
where
    A: Address,
{
    /// Construct a register description at `address`.
    #[inline]
    #[must_use]
    pub const fn register(address: A) -> Self {
        Self {
            address,
            value: marker::PhantomData,
        }
    }

    /// Return the register address in its native integer type.
    #[inline]
    #[must_use]
    pub const fn address(&self) -> A {
        let &Self { address, .. } = self;

        address
    }
}

/// A read-only register.
pub type Ro<T, A> = Gated<T, A, true, false>;

/// A read-write register.
pub type Rw<T, A> = Gated<T, A, true, true>;

/// A write-only register.
pub type Wo<T, A> = Gated<T, A, false, true>;

/// A described register unavailable through the current interface.
pub type Unaccessible<T, A> = Gated<T, A, false, false>;

#[cfg(test)]
mod tests {
    use super::{Rw, Unaccessible};

    #[test]
    fn register_preserves_its_numeric_address() {
        let register = Rw::<u64, u32>::register(0xc000_0080);

        assert_eq!(register.address(), 0xc000_0080);
    }

    #[test]
    fn inaccessible_register_still_has_an_address() {
        let register = Unaccessible::<u32, u16>::register(7);

        assert_eq!(register.address(), 7);
    }
}
