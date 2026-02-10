//! Retry mechanism based on controlled backoff and a bounded limit.

use core::{num::NonZero, ops::ControlFlow};

use crate::{
    backoff::{Backoff, BackoffState},
    limit::Limit,
};

/// A controlled retry state.
///
/// This incorporates the functionality of [`Limit`] and [`Backoff`] to form an
/// unified interface for controlled limit and backoff policies.
#[derive(Debug)]
pub struct Retry(Option<Limit>, BackoffState);

impl Retry {
    /// Construct a new [`Retry`] state using the selected backoff strategy but
    /// with a singular attempt.
    #[inline]
    #[must_use]
    pub const fn oneshot(backoff_strategy: Backoff) -> Self {
        Self::bare((Some(Limit::one()), backoff_strategy.state()))
    }

    /// Construct a new [`Retry`] state with a singular attempt and a target
    /// existing backoff state.
    #[inline]
    #[must_use]
    pub const fn oneshot_with(backoff_state: BackoffState) -> Self {
        Self::bare((Some(Limit::one()), backoff_state))
    }

    /// Construct a new [`Retry`] state using the selected backoff strategy but
    /// with unlimited attempts.
    #[inline]
    #[must_use]
    pub const fn unlimited(backoff_strategy: Backoff) -> Self {
        Self::bare((None, backoff_strategy.state()))
    }

    /// Construct a new [`Retry`] state with unlimited attempts and a target
    /// existing backoff state.
    #[inline]
    #[must_use]
    pub const fn unlimited_with(backoff_state: BackoffState) -> Self {
        Self::bare((None, backoff_state))
    }

    /// Construct a new [`Retry`] state from a bare [`Option<Limit>`] and
    /// [`BackoffState`] 2-tuple.
    #[inline]
    #[must_use]
    pub const fn bare((limit_state, backoff_state): (Option<Limit>, BackoffState)) -> Self {
        Self(limit_state, backoff_state)
    }

    /// Determine the [`Limit`] structure used in this [`Retry`].
    #[inline]
    #[must_use]
    pub const fn limit(&self) -> Option<&Limit> {
        let Self(target_limit, ..) = self;

        target_limit.as_ref()
    }

    /// Determine the [`Backoff`] strategy used in this [`Retry`].
    #[inline]
    #[must_use]
    pub const fn backoff(&self) -> Backoff {
        let Self(.., target_state) = self;

        target_state.strategy()
    }

    /// Attempt some operation, tracking it into this [`Retry`].
    ///
    /// This yields either:
    ///
    /// - [`ControlFlow::Break`] if the operation cannot be retried further.
    /// - [`ControlFlow::Continue`] if the operation can indeed continue
    ///   further. The closure embedded within the continue variant must be used
    ///   to engage in backoff.
    #[inline]
    #[must_use]
    pub fn attempt<'a>(
        &'a mut self,
    ) -> ControlFlow<NonZero<usize>, impl FnOnce() -> Option<NonZero<usize>> + use<'a>> {
        let Self(limit_state, backoff_state) = self;

        let target_closure = || -> Option<NonZero<usize>> {
            Backoff::cycle(backoff_state);

            BackoffState::cycle(backoff_state)
        };

        if let Some(target_outcome) = limit_state.as_mut().map(Limit::actuate) {
            match target_outcome {
                ControlFlow::Continue(..) => ControlFlow::Continue(target_closure),
                ControlFlow::Break(target_count) => ControlFlow::Break(target_count),
            }
        } else {
            ControlFlow::Continue(target_closure)
        }
    }
}
