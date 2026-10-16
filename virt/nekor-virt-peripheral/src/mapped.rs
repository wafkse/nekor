//! Backend-specific state selection and dispatch for peripheral operations.
//!
//! A [`Mapping`] implementation on a [`Map`] key selects the state and operation delegate at
//! compile time. In this serial example, the delegate records a pending byte in caller-owned
//! provisional state, leaving publication to the caller.
//!
//! ```
//! use core::convert::Infallible;
//!
//! use nekor_virt_hypervisor::baseline::Backend;
//! use nekor_virt_model::prelude::Arch;
//! use nekor_virt_peripheral::{
//!     mapped::{Map, Mapping},
//!     peripheral::Peripheral,
//! };
//!
//! # struct Guest;
//! # impl Arch for Guest {}
//! # struct ExampleBackend;
//! # impl Backend<Guest> for ExampleBackend {}
//! struct Serial;
//!
//! impl Peripheral<Guest> for Serial {
//!     type Error = Infallible;
//!     type Input<'a> = u8;
//!     type Output = ();
//! }
//!
//! // NOTE(invariant): None.
//! struct SerialState {
//!     pending_byte: Option<u8>,
//! }
//!
//! impl Mapping<Serial, Guest> for Map<Serial, ExampleBackend, Guest> {
//!     type Mapped = SerialState;
//!
//!     fn delegate<'a>(state: &mut Self::Mapped, input: u8) -> Result<(), Infallible> {
//!         state.pending_byte = Some(input);
//!         Ok(())
//!     }
//! }
//!
//! type State = <Map<Serial, ExampleBackend, Guest> as Mapping<Serial, Guest>>::Mapped;
//!
//! let mut provisional_state = State { pending_byte: None };
//! let outcome = Serial::action::<ExampleBackend>(&mut provisional_state, b'A');
//!
//! assert_eq!(outcome, Ok(()));
//! assert_eq!(provisional_state.pending_byte, Some(b'A'));
//! ```

use core::{convert, fmt, marker};

use nekor_virt_hypervisor::baseline::Backend;
use nekor_virt_model::prelude::Arch;

use crate::peripheral::Peripheral;

/// Selects the state representation and operation implementation for a peripheral.
///
/// Implement this trait on [`Map<P, B, A>`](Map) to provide the behavior for backend `B`.
/// [`Peripheral::action`] then uses that implementation to execute an input.
pub trait Mapping<P, A>
where
    P: Peripheral<A>,
    A: Arch,
{
    type Mapped;

    /// Executes an input against provisional transaction state.
    ///
    /// Implementations must keep state changes provisional so the caller can stage the returned
    /// architectural outcome before publication, as required by [`Peripheral`].
    fn delegate<'a>(state: &mut Self::Mapped, input: P::Input<'a>) -> Result<P::Output, P::Error>;
}

/// A type-level key selecting a peripheral's backend-specific implementation.
///
/// Because this type is uninhabited, callers work with [`Mapping::Mapped`] state and invoke
/// its delegate through [`Peripheral::action`].
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
// NOTE(invariant): None.
pub struct Map<P, B, A>(convert::Infallible, marker::PhantomData<fn() -> (P, B, A)>)
where
    P: Peripheral<A>,
    B: Backend<A>,
    A: Arch;

impl<P, B, A> fmt::Display for Map<P, B, A>
where
    P: Peripheral<A>,
    B: Backend<A>,
    A: Arch,
{
    #[inline]
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}
