//! Architectural peripheral operations and their input and outcome types.

use core::error;

use nekor_virt_hypervisor::baseline::Backend;
use nekor_virt_model::prelude::Arch;

use crate::{
    effect::Effect,
    mapped::{Map, Mapping},
};

/// An ISA-defined interface for handling guest peripheral operations.
///
/// Peripherals include processor-integrated facilities and external devices defined by the
/// guest platform. The execution backend supplies an architectural operation, such as an
/// intercepted `OUT` instruction or x2APIC register access, which [`Mapping`] executes against
/// transaction-owned state. The corresponding [`Map`] selects the implementation for the
/// peripheral, backend, and architecture, allowing the same interface to use backend-specific
/// state representations. Multiple interfaces may share architectural state where the guest
/// architecture requires it.
///
/// Although the mapping performs the operation, the caller controls transaction publication.
/// The caller must supply provisional state, and the mapping must keep its changes provisional
/// until [`Commit::commit`](crate::effect::transact::Commit::commit). Isolation therefore depends
/// on the caller and mapping implementation upholding this contract.
///
/// Because both successful and unsuccessful operations can produce guest-visible effects, the
/// caller must stage either outcome through [`Effect::stage`].
pub trait Peripheral<A>
where
    A: Arch,
    Self: Sized,
{
    /// May borrow decoded operation data for `'a`.
    type Input<'a>;

    type Output: Effect<A>;

    /// A guest architectural error, such as a request to inject an exception.
    ///
    /// Failure to stage this outcome is reported separately through
    /// [`Stage::Error`](crate::effect::Stage::Error).
    type Error: error::Error + Effect<A>;

    /// Calls [`Mapping::delegate`] for backend `B` under the transaction contract of
    /// [`Peripheral`].
    #[inline]
    fn action<'a, B>(
        peripheral_state: &mut <Map<Self, B, A> as Mapping<Self, A>>::Mapped,
        target_input: Self::Input<'a>,
    ) -> Result<Self::Output, Self::Error>
    where
        B: Backend<A>,
        Map<Self, B, A>: Mapping<Self, A>,
    {
        <Map<Self, B, A> as Mapping<Self, A>>::delegate(peripheral_state, target_input)
    }
}
