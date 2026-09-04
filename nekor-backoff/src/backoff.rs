//! Cycle-based backoff control.
//!
//! See the [`Backoff`] type for further information.

use core::num::NonZero;

/// The exponential base for backoff delay calculation.
///
/// This enum restricts base values to efficient powers-of-2 that optimize to
/// shift operations rather than multiplication.
///
/// # Selection Guide
///
/// Choose your base according to contention characteristics:
///
/// - [`Base2`]: Low contention, fast resolution needed (e.g., brief atomic operations)
/// - [`Base4`]: Moderate contention with predictable access patterns
/// - [`Base8`]: High contention or longer critical sections
/// - [`Base16`]: Very high contention or expensive critical sections
/// - [`Base32`]: Extreme contention, prefer avoiding CPU busy-waiting
///
/// Higher bases cause backoff to grow more aggressively, reducing CPU usage
/// but potentially increasing latency for uncontended cases.
///
/// # Delay Progression Example
///
/// With iteration `i` and ceiling ∞:
///
/// | Base | i=0 | i=1 | i=2 | i=3 | i=4 |
/// |------|-----|-----|-----|-----|-----|
/// | 2    | 1   | 2   | 4   | 8   | 16  |
/// | 4    | 1   | 4   | 16  | 64  | 256 |
/// | 8    | 1   | 8   | 64  | 512 | 4K  |
///
/// [`Base2`]: ExponentialBase::Base2
/// [`Base4`]: ExponentialBase::Base4
/// [`Base8`]: ExponentialBase::Base8
/// [`Base16`]: ExponentialBase::Base16
/// [`Base32`]: ExponentialBase::Base32
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
#[repr(u8)]
#[non_exhaustive /* for additional exponential bases to be added */]
pub enum ExponentialBase {
    /// Exponential base of 2. Slowest backoff growth.
    Base2 = 2,

    /// Exponential base of 4. Moderate backoff growth.
    Base4 = 4,

    /// Exponential base of 8. Aggressive backoff growth.
    Base8 = 8,

    /// Exponential base of 16. Very aggressive backoff growth.
    Base16 = 16,

    /// Exponential base of 32. Most aggressive backoff growth.
    Base32 = 32,
}

/// An exponential backoff algorithm implementation.
///
/// This makes use of architecture-specific features to provide a more efficient
/// implementation.
/// For instance, in *x86* (and *`x86_64`*), instructions such as `pause`,
/// `tpause`, `rdtsc`, and `monitorx` (AMD-specific) are used.
///
/// # The Algorithm
///
/// The algorithm is rather simple, as we posess two main components:
///
/// - The *Exponential Base*: The base value used for the exponential backoff. This is not a
///   discrete value, but a selection from predefined values. See the [`ExponentialBase`] enum.
///
/// - The *Exponential Ceiling*: The maximum backoff value, i.e. the maximum numeric delay that the
///   backoff can reach. Any delay value will be clamped to the `[0, c]` interval, where `c` is the
///   *Exponential Ceiling*.
///
/// The backoff delay is generated through a sequential counter, starting from
/// `0`. This allows for small contention to be resolved quickly, without having
/// to wait for the initial case.
///
/// The *exponential ceiling*, if unspecified, it will be equal to the maximum
/// value of the [`usize`] type (see [`usize::MAX`]).
///
/// ## Remarks
///
/// Your choice of *Exponential Base* depends on *how granular* the contended
/// resource is. For instance, it is better to assign a higher base value for
/// resources that are more likely to be contended, as this will result in a
/// higher-backoff delay, possibly increasing throughput across the critical
/// section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Backoff {
    /// The exponential base used for the exponential backoff.
    exponential_base: ExponentialBase,

    /// The exponential ceiling used for the exponential backoff.
    ///
    /// This is specified in *retry cycles*, not as an absolute backoff delay.
    exponential_ceiling: NonZero<usize>,
}

/// The default [`Backoff`] strategy.
///
/// The default strategy corresponds to the same strategy as
/// [`Backoff::minimal`].
impl Default for Backoff {
    #[inline]
    fn default() -> Self {
        Self::minimal()
    }
}

impl Backoff {
    /// Determine the minimal [`Backoff`] configuration.
    #[inline]
    #[must_use]
    pub const fn minimal() -> Self {
        let exponential_base = ExponentialBase::Base2;

        let exponential_ceiling = const {
            match NonZero::<usize>::new((usize::BITS - 1) as usize) {
                Some(target_value) => target_value,
                None => unreachable!(),
            }
        };

        Self {
            exponential_base,
            exponential_ceiling,
        }
    }

    /// Determine the *exponential base* used for this [`Backoff`].
    #[inline]
    #[must_use]
    pub const fn base(self) -> ExponentialBase {
        let Self { exponential_base, .. } = self;

        exponential_base
    }

    /// Determine the *exponential ceiling* used for this [`Backoff`].
    #[inline]
    #[must_use]
    pub const fn ceiling(self) -> NonZero<usize> {
        let Self {
            exponential_ceiling, ..
        } = self;

        exponential_ceiling
    }

    /// Engage into a controlled backoff cycle with the specified backoff
    /// strategy.
    #[inline]
    #[must_use]
    pub const fn state(self) -> BackoffState {
        BackoffState {
            backoff_strategy: self,
            cycle_count: None,
        }
    }

    /// Perform a single backoff cycle using the engaged strategy.
    #[inline]
    pub const fn cycle(backoff_state: &mut BackoffState) {
        let &mut BackoffState {
            ref mut cycle_count, ..
        } = backoff_state;

        // TODO: Use the hotpatch mechanism to choose the best method here.
        //
        // Use tpause, pause in a loop with rdtsc, whatever, just make it wait.

        match cycle_count.as_mut() {
            Some(cycle_count) => *cycle_count = cycle_count.saturating_add(1),
            None => *cycle_count = Some(NonZero::<usize>::MIN),
        }
    }
}

/// The associated state of an engaged [`Backoff`] strategy.
///
/// This is used to keep track of the performed backoff cycles and determined
/// strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct BackoffState {
    /// The [`Backoff`] strategy that is engaged.
    backoff_strategy: Backoff,

    /// The count of attempted backoff cycles for this state.
    // FIXME: Remove TSC-quanta assumption, rely on "the highest wait
    // granularity available by the processor.". Its vague, but OOE and
    // superscalar processors don't help us.
    cycle_count: Option<NonZero<usize>>,
}

impl BackoffState {
    /// Determine the [`Backoff`] strategy that is used for this state.
    #[inline]
    #[must_use]
    pub const fn strategy(&self) -> Backoff {
        let &Self { backoff_strategy, .. } = self;

        backoff_strategy
    }

    /// Determine the count of performed backoff cycles.
    #[inline]
    #[must_use]
    pub const fn cycle(&self) -> Option<NonZero<usize>> {
        let &Self { cycle_count, .. } = self;

        cycle_count
    }
}
