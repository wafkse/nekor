//! A [`Domain`] that ties static storage to some arbitrary type.
//!
//! This happens to be useful when delaing with *unnameable types*, such as `fn
//! items`.

use core::marker;

use crate::{
    domain::{Adapter, Domain},
    prelude::Store,
};

/// A marker trait to determine whether a target type can be tied a domain to.
pub trait Tiable: 'static {}

/// Blanket implementation for all automatically-[`Tiable`] types.
impl<T> Tiable for T where T: ?Sized + 'static {}

/// A [`Domain`] that ties static storage to the `T` type.
#[repr(transparent)]
pub struct Arbitrary<T>(marker::PhantomData<fn() -> T>)
where
    T: Tiable;

impl<T> Domain for Arbitrary<T>
where
    T: Tiable,
{
    type Adapter<S>
        = Arbitrarily<T, S>
    where
        S: Store;
}

/// The [`Adapter`] pertinent to the [`Arbitrary`] [`Domain`].
#[repr(transparent)]
pub struct Arbitrarily<T, S>(
    // NOTE(invariant): Must be `repr(transparent)` over `S`.
    S,
    marker::PhantomData<fn() -> T>,
)
where
    T: Tiable,
    S: Store;

// SAFETY: `Arbitrarily` is `repr(transparent)` over `S`.
unsafe impl<T, S> Adapter for Arbitrarily<T, S>
where
    T: Tiable,
    S: Store,
{
    type Target = S;
}
