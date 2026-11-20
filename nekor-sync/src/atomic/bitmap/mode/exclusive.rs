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
    /// The transition was not performed, with its reason and an acquire snapshot.
    ///
    /// Contention returns the value observed by the failed compare-and-exchange. Other
    /// failures return the selection snapshot, which may already be stale.
    // NOTE: The last observed state must have been obtained with a
    // memory ordering of [`Acquire`] or stronger.
    Failure(Reason, usize),

    /// This operation changed the bit, with the word observed before that change.
    // NOTE: The last observed state must have been obtained with a
    // memory ordering of [`Acquire`] or stronger.
    Success(usize),
}

/// The failure reason for the [`Outcome::Failure`] signal enum variant.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Reason {
    /// The engaged bit was already in the target state.
    Unchanged,

    /// The bitmap word differed from the selection snapshot during compare-and-exchange.
    Contended,

    /// The operation was attempted too many times and has reached the imposed
    /// limit.
    Limited,
}

/// Change a selected bit only when the word still matches its acquire snapshot.
///
/// A successful compare-and-exchange uses acquire-release ordering. Failure observes the
/// word with acquire ordering and reports contention, even if another bit caused the
/// difference. An already matching bit reports `Unchanged`. The caller must select again
/// when retrying after contention.
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

                if bit_handle.state() == State::Set {
                    match target_atomic.compare_exchange(state_snapshot, bit_handle.cleared(), AcqRel, Acquire) {
                        Ok(successful_snapshot) => Outcome::Success(successful_snapshot),
                        Err(latest_snapshot) => {
                            backoff_cycle();

                            Outcome::Failure(Reason::Contended, latest_snapshot)
                        },
                    }
                } else {
                    Outcome::Failure(Reason::Unchanged, state_snapshot)
                }
            },
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

                if bit_handle.state() == State::Cleared {
                    match target_atomic.compare_exchange(state_snapshot, bit_handle.enabled(), AcqRel, Acquire) {
                        Ok(successful_snapshot) => Outcome::Success(successful_snapshot),
                        Err(latest_snapshot) => {
                            backoff_cycle();

                            Outcome::Failure(Reason::Contended, latest_snapshot)
                        },
                    }
                } else {
                    Outcome::Failure(Reason::Unchanged, state_snapshot)
                }
            },
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
