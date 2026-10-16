//! Architectural effects and transaction-specific staging.
//!
//! Peripheral outcomes describe guest-visible consequences through [`Effect`]. A transaction
//! handles those consequences through [`Stage`] before following the preparation and publication
//! lifecycle defined in [`transact`].

use core::{convert, error};

use nekor_virt_model::prelude::Arch;

pub mod transact;

/// An architectural consequence staged as part of a transaction.
///
/// Effects can target state beyond the originating peripheral. For example, a timer expiration
/// may request an interrupt through an architectural interrupt controller. The target's [`Stage`]
/// implementation determines how to record that consequence in provisional state, allowing the
/// same effect type to be handled by transactions with different state representations.
///
/// Staging may combine consequences from several operations, but their publication belongs to
/// [`Commit::commit`](transact::Commit::commit). The transaction must therefore preserve
/// authoritative state until commit and when abandoned, as specified in [`transact`].
pub trait Effect<A: Arch>: Sized {
    /// Delegates to [`Stage::stage`] for the target transaction.
    #[inline]
    fn stage<T>(self, target_value: &mut T) -> Result<(), T::Error>
    where
        T: Stage<A, Self>,
    {
        Stage::stage(target_value, self)
    }
}

/// Unit outcomes still require a target-specific [`Stage<A, ()>`](Stage) implementation, even
/// though they carry no additional effect data.
impl<A: Arch> Effect<A> for () {}

impl<A: Arch> Effect<A> for convert::Infallible {}

/// Allows an operation's result to use a result-specific staging handler.
///
/// The target must implement [`Stage<A, Result<O, E>>`](Stage) and handle the selected variant,
/// even when it also implements staging for `O` and `E` individually.
impl<A, O, E> Effect<A> for Result<O, E>
where
    A: Arch,
    O: Effect<A>,
    E: Effect<A>,
{
}

/// Prepares an effect's changes within a transaction.
///
/// An implementation may validate the effect, reserve resources, and update provisional state
/// shared by several peripherals or logical processors. Because later staging or preparation
/// may fail, authoritative changes and irreversible host I/O must be deferred until
/// [`Commit::commit`](transact::Commit::commit).
///
/// If staging fails, the implementation must record that failure and prevent preparation until
/// explicit recovery restores the transaction's consistency. This is an implementation
/// obligation, so merely returning an error is insufficient to uphold the contract.
pub trait Stage<A, E>
where
    A: Arch,
    E: Effect<A>,
{
    /// A failure to prepare an effect, distinct from any guest architectural error carried by
    /// the effect itself.
    type Error: error::Error;

    fn stage(&mut self, effect: E) -> Result<(), Self::Error>;
}
