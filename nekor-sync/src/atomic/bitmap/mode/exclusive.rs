//! The *exclusive* [`AtomicBitmap`] engagement [`Mode`].
//!
//! [`AtomicBitmap`]: crate::atomic::bitmap::AtomicBitmap
//! [`Mode`]: crate::atomic::bitmap::mode::Mode

use core::{
    fmt::{self, Display},
    ops::ControlFlow,
    sync::atomic::Ordering::{AcqRel, Acquire},
};

use nekor_backoff::prelude::Retry;

use nekor_bitwise::prelude::{BitDyn, Selected, State};

use crate::atomic::bitmap::{
    at::{At, Disengage},
    mode::{InMode, Mode},
};

/// A signal outcome for the [`Exclusive`] mode.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Outcome {
    /// A failure occurred.
    ///
    /// This variant is accompanied with the latest bitmap state snapshot
    /// observed. This can be in turn used to accelerate retries in specific
    /// scenarios. Particularly, a [`Reason::ConcurrentModification`] failure
    /// reason corresponds to the [`Acquire`] failure ordering of the
    /// compare-exchange operation, this can be used to omit an aditional atomic
    /// load if the backoff state was not used.
    ///
    /// This generally corresponds to the operation not being able to be
    /// completed with [`Exclusive`]-level guarantees.
    // NOTE(invariant): The last observed state must have been obtained with a
    // memory ordering of [`Acquire`] or stronger.
    Failure(Reason, usize),

    /// The operation was successful.
    ///
    /// This outcome is guaranteed to be returned if the operation was
    /// successful, as all [`Exclusive`] mode guarantees are upheld.
    ///
    /// This is accompanied by the latest observed value of the
    /// [`AtomicBitmap`].
    // NOTE(invariant): The last observed state must have been obtained with a
    // memory ordering of [`Acquire`] or stronger.
    Success(usize),
}

/// The failure reason for the [`Outcome::Failure`] signal enum variant.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Reason {
    /// The engaged bit was already in the target state.
    Unchanged,

    /// The engaged bit was modified concurrently.
    Contended,

    /// The operation was attempted too many times and has reached the imposed
    /// limit.
    Limited,
}

/// An exclusive engagement [`Mode`].
///
/// # Provided Guarantees
///
/// This engagement mode provides *exclusive* guarantees regarding the Finalizer
/// Operation, in other words, this mode ensures that only one contender may
/// change the state of the bit successfully.
///
/// The exclusivity guarantee can be relied upon in unsafe code, it is a hard
/// invariant.
// NOTE: Auto-derive all the following traits to allow types with generic
// [`Mode`] parameters to derive them too without relaxing generic bounds.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Exclusive {}

// SAFETY: The (exclusive) guarantees of this engagement mode are known and
// well-documented, furthermore, the implementation of each finalizer is atomic
// and thread-safe.
unsafe impl Mode for Exclusive {
    type Signal = Outcome;

    type State = Retry;

    #[inline]
    fn zero_with(at: At<'_>, target_state: &mut Self::State, in_mode: &InMode) -> Self::Signal {
        let Disengage(target_atomic, state_snapshot, bit_indice) = At::disengage(at, in_mode);

        match Retry::attempt(target_state) {
            ControlFlow::Continue(backoff_cycle) => {
                let bit_indice = Selected::new(bit_indice);

                let bit_handle = BitDyn::wrap(&state_snapshot, bit_indice);

                if let State::Set = bit_handle.state() {
                    match target_atomic.compare_exchange(
                        state_snapshot,
                        bit_handle.cleared(),
                        AcqRel,
                        Acquire,
                    ) {
                        Ok(successful_snapshot) => Outcome::Success(successful_snapshot),
                        Err(latest_snapshot) => {
                            let _ = backoff_cycle();

                            Outcome::Failure(Reason::Contended, latest_snapshot)
                        }
                    }
                } else {
                    Outcome::Failure(Reason::Unchanged, state_snapshot)
                }
            }
            ControlFlow::Break(..) => Outcome::Failure(Reason::Limited, state_snapshot),
        }
    }

    #[inline]
    fn one_with(at: At<'_>, target_state: &mut Self::State, in_mode: &InMode) -> Self::Signal {
        let Disengage(target_atomic, state_snapshot, bit_indice) = At::disengage(at, in_mode);

        match Retry::attempt(target_state) {
            ControlFlow::Continue(backoff_cycle) => {
                let bit_indice = Selected::new(bit_indice);

                let bit_handle = BitDyn::wrap(&state_snapshot, bit_indice);

                if let State::Cleared = bit_handle.state() {
                    match target_atomic.compare_exchange(
                        state_snapshot,
                        bit_handle.enabled(),
                        AcqRel,
                        Acquire,
                    ) {
                        Ok(successful_snapshot) => Outcome::Success(successful_snapshot),
                        Err(latest_snapshot) => {
                            let _ = backoff_cycle();

                            Outcome::Failure(Reason::Contended, latest_snapshot)
                        }
                    }
                } else {
                    Outcome::Failure(Reason::Unchanged, state_snapshot)
                }
            }
            ControlFlow::Break(..) => Outcome::Failure(Reason::Limited, state_snapshot),
        }
    }
}

impl Display for Exclusive {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        // NOTE: `Exclusive` is uninhabited

        Ok(())
    }
}

#[cfg(all(test, usermode))]
mod tests {}
