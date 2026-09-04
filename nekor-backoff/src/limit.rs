//! A limit for contended operations.
//!
//! See the [`Limit`] type for further information.

use core::{num::NonZero, ops::ControlFlow};

/// A self-imposed limit.
///
/// This can be useful to directly bail if such lock-free operation is to be
/// done opportunistically under some pretense, but is not required.
///
/// This exactly encompasses the imposed limit and the associated actuation
/// count.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Limit(NonZero<usize>, Option<NonZero<usize>>);

impl Limit {
    /// Construct a one-attempt limit.
    #[inline]
    #[must_use]
    pub const fn one() -> Self {
        Self(NonZero::<usize>::MIN, None)
    }

    /// Construct a one-attempt limit.
    #[inline]
    #[must_use]
    pub const fn one_with(actuation_count: Option<NonZero<usize>>) -> Self {
        Self(NonZero::<usize>::MIN, actuation_count)
    }

    /// Construct a limit for a specific number of attempts.
    #[inline]
    #[must_use]
    pub const fn of(target_value: NonZero<usize>) -> Self {
        Self(target_value, None)
    }

    /// Construct a limit for a specific number of attempts, with a specified
    /// actuation count.
    #[inline]
    #[must_use]
    pub const fn of_with(imposed_limit: NonZero<usize>, actuation_count: Option<NonZero<usize>>) -> Self {
        Self(imposed_limit, actuation_count)
    }

    /// Determine the imposed attempt limit.
    #[inline]
    #[must_use]
    pub const fn imposed(&self) -> NonZero<usize> {
        let &Self(target_value, ..) = self;

        target_value
    }

    /// Determine the count of actuations performed with this limit.
    #[inline]
    #[must_use]
    pub const fn count(&self) -> Option<NonZero<usize>> {
        let &Self(.., target_value) = self;

        target_value
    }

    /// Reset the actuation counter in this limit.
    #[inline]
    #[must_use]
    pub const fn reset(self) -> Self {
        let Self(target_value, ..) = self;

        Self(target_value, None)
    }

    /// Perform a single actuation for this imposed limit.
    ///
    /// In the case the limit has been exceeded, a [`ControlFlow::Break`] is
    /// returned.
    #[inline]
    pub const fn actuate(&mut self) -> ControlFlow<NonZero<usize>> {
        let &mut Self(imposed_limit, ref mut actuation_count) = self;

        if let Some(target_count) = actuation_count.as_mut() {
            if target_count.get() >= imposed_limit.get() {
                ControlFlow::Break(*target_count)
            } else {
                *target_count = target_count.saturating_add(1);

                ControlFlow::Continue(())
            }
        } else {
            *actuation_count = Some(NonZero::<usize>::MIN);

            ControlFlow::Continue(())
        }
    }
}
