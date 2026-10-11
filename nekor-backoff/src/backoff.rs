//! Cycle-based backoff control.
//!
//! See the [`Backoff`] type for further information.

use core::num::NonZero;

/// The power-of-two base of a [`Backoff`] progression.
///
/// # Selection
///
/// Higher bases grow delays faster, trading retry latency for less busy-waiting.
///
/// - [`Self::Base2`] suits low contention and brief operations.
/// - [`Self::Base4`] suits moderate, predictable contention.
/// - [`Self::Base8`] suits high contention or longer critical sections.
/// - [`Self::Base16`] suits very high contention or expensive critical sections.
/// - [`Self::Base32`] favors avoiding busy-waiting under extreme contention.
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

impl ExponentialBase {
    /// Return the numeric base used by the exponential progression.
    #[inline]
    #[must_use]
    pub const fn fundamental(self) -> usize {
        self as usize
    }

    /// Whether the selected base exceeds the supplied exponential ceiling.
    #[inline]
    #[must_use]
    pub const fn exceeds(self, target_ceiling: NonZero<usize>) -> bool {
        target_ceiling.get() < self.fundamental()
    }
}

/// An exponential backoff strategy for contended operations.
///
/// This selects how quickly relaxation requests grow and how large they may
/// become. [`BackoffState`] tracks the progression for a retry sequence.
/// The strategy does not limit retry attempts. For that, see [`Retry`].
///
/// # Platform Wait Quanta
///
/// A *Platform Wait Quantum* is a platform-defined unit of requested processor
/// relaxation. Backoff delays are expressed as a count of these units, rather
/// than as a duration in nanoseconds or a number of failed attempts.
///
/// In other words, requesting more quanta asks for more relaxation, but does
/// not promise a particular elapsed time. A quantum is not a guaranteed minimum
/// wait, nor is its duration necessarily constant between calls or processors.
///
/// # Progression
///
/// The strategy has two parameters:
///
/// - The *Exponential Base*, selected through [`ExponentialBase`], determines how quickly the
///   requested quantum count grows.
/// - The *Exponential Ceiling* bounds the quantum count of each request. It does not bound the
///   number of retries or their accumulated delay.
///
/// The first cycle performs a single relaxation hint. Subsequent cycles request
/// `1`, `base`, `base²`, and so on, clamped to the ceiling. The cycle count
/// saturates at [`usize::MAX`] and the delay calculation uses saturating arithmetic.
///
/// For example, `Base4` with a ceiling of `10` performs a hint, then requests
/// `1`, `4`, and `10` quanta. All later requests remain at `10`.
///
/// [`Self::minimal`] uses `Base2` and a ceiling of `usize::BITS - 1`.
/// [`Self::capped`] accepts an explicit ceiling at least as large as the base.
///
/// # Platform Implementation
///
/// On `x86` and `x86_64`, each timed request estimates a quantum from the absolute
/// TSC change across a platform-configured chain of `pause` instructions. This
/// includes `rdtsc` overhead and does not serialize instruction execution.
/// The estimate is sampled again for each request, not calibrated once at startup.
///
/// The selected `Relaxation` delegate determines how that estimate is used:
///
/// - `Relaxation::Pause` polls the TSC between batches of `pause` chains. After sampling, it
///   performs at most the requested number of chains, even if the TSC stops advancing. It may
///   return sooner when the requested ticks have elapsed.
/// - `Relaxation::Tpause` uses the estimate to form a TSC deadline for `tpause`. This requires
///   `WAITPKG` support. The hardware may return before the deadline.
///
/// Both delegates cap the requested ticks at `Relaxation::TIMESTAMP_LIMIT`.
/// Timed requests require TSC access on every executing CPU. A hint only issues
/// `pause`, without sampling the TSC or entering a timed wait.
///
/// On other targets, and under Miri, a quantum is a call to [`core::hint::spin_loop`].
/// This fallback performs the requested number of hints without clock calibration.
///
/// # Guarantees
///
/// Requests after the initial hint have a positive quantum count no greater
/// than the strategy's ceiling. Increasing that count does not guarantee a
/// longer elapsed wait, since sampling and execution conditions may change.
///
/// Neither the ceiling nor the x86 tick cap establishes a wall-clock deadline.
/// Interrupts, descheduling, and virtualization may extend elapsed time. Hardware
/// wakeups, instruction reordering, and backward TSC movement may shorten it.
/// This is processor relaxation, not a precise sleep or a synchronization operation.
///
/// Backoff does not guarantee fairness, starvation freedom, or that the contended
/// operation will eventually succeed. A hardware monitor consuming [`BackoffState`]
/// may also ignore its requested delay. Such waits follow the monitor's contract,
/// not a timeout guaranteed by this strategy.
///
/// [`Retry`]: crate::retry::Retry
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
// NOTE(invariant): `exponential_ceiling` is at least the numeric value of `exponential_base`.
pub struct Backoff {
    /// The base of the delay progression.
    exponential_base: ExponentialBase,

    /// The maximum delay magnitude in platform wait quanta, not a retry count.
    exponential_ceiling: NonZero<usize>,
}

/// The [`Backoff::minimal`] strategy.
impl Default for Backoff {
    #[inline]
    fn default() -> Self {
        Self::minimal()
    }
}

impl Backoff {
    /// Use the slowest [`ExponentialBase`] and a word-size-derived ceiling.
    ///
    /// See [`Self::capped`] for an explicit ceiling.
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

    /// Issue a single spin-wait hint without timing or retaining state.
    ///
    /// Use [`BackoffState`] for repeated contention.
    #[inline]
    pub fn oneshot() {
        Self::relax(None);
    }

    /// Construct a [`Backoff`] with the supplied base and delay ceiling.
    ///
    /// # Failure
    ///
    /// Returns [`None`] if the ceiling is smaller than the base, without adjusting it.
    #[inline]
    #[must_use]
    pub const fn capped(exponential_base: ExponentialBase, exponential_ceiling: NonZero<usize>) -> Option<Self> {
        if exponential_base.exceeds(exponential_ceiling) {
            None
        } else {
            Some(Self {
                exponential_base,
                exponential_ceiling,
            })
        }
    }

    /// Return the base of the delay progression.
    #[inline]
    #[must_use]
    pub const fn base(self) -> ExponentialBase {
        let Self { exponential_base, .. } = self;

        exponential_base
    }

    /// Return the maximum delay magnitude in platform wait quanta.
    #[inline]
    #[must_use]
    pub const fn ceiling(self) -> NonZero<usize> {
        let Self {
            exponential_ceiling, ..
        } = self;

        exponential_ceiling
    }

    /// Start a [`BackoffState`] for this strategy.
    #[inline]
    #[must_use]
    pub const fn state(self) -> BackoffState {
        BackoffState {
            backoff_strategy: self,
            cycle_count: None,
        }
    }

    /// Advance the [`BackoffState`] and execute its requested relaxation.
    ///
    /// See [`BackoffState::advance`] for the delay progression.
    #[inline]
    pub fn cycle(target_state: &mut BackoffState) {
        let target_delay = target_state.advance();

        Self::relax(target_delay);
    }

    /// Execute a hint or a bounded number of platform wait quanta.
    #[inline]
    fn relax(target_delay: Option<NonZero<usize>>) {
        cfg_select! {
            all(any(target_arch = "x86", target_arch = "x86_64"), not(miri)) => {
                use nekor_aal_hotpatch::prelude::Patch;

                use crate::arch::x86::{Delay, Relaxation};

                let target_delay = target_delay.map_or(Delay::Hint, Delay::Timed);

                Patch::<Relaxation, Relaxation>::run(target_delay);
            },
            _ => {
                let target_hint_count = target_delay.map_or(1, NonZero::get);

                for _ in 0..target_hint_count {
                    core::hint::spin_loop();
                }
            },
        }
    }
}

/// A [`Backoff`] strategy and its cycle count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
// NOTE(invariant): None.
pub struct BackoffState {
    /// The delay strategy.
    backoff_strategy: Backoff,

    /// The count of attempted backoff cycles for this state.
    cycle_count: Option<NonZero<usize>>,
}

impl BackoffState {
    /// Advance the count without waiting and return the requested platform wait quanta.
    ///
    /// Returns [`None`] for the initial hint, then `1`, `base`, `base²`, and further
    /// quanta capped by the ceiling. The count saturates at [`usize::MAX`].
    /// [`Backoff::cycle`] also executes the relaxation.
    #[inline]
    #[must_use]
    pub fn advance(&mut self) -> Option<NonZero<usize>> {
        let &mut Self {
            backoff_strategy: target_strategy,
            cycle_count: ref mut target_cycle_count,
        } = self;

        let target_delay = match *target_cycle_count {
            None => None,
            Some(target_count) => {
                // NOTE: Exponents above `usize::BITS` exceed every representable ceiling.
                let target_exponent = (target_count.get() - 1).min(usize::BITS as usize) as u32;
                let target_magnitude = target_strategy.base().fundamental().saturating_pow(target_exponent);
                let target_magnitude = target_magnitude.min(target_strategy.ceiling().get());

                NonZero::new(target_magnitude)
            },
        };

        *target_cycle_count = Some(match *target_cycle_count {
            None => NonZero::<usize>::MIN,
            Some(target_count) => target_count.saturating_add(1),
        });

        target_delay
    }

    /// Return the [`Backoff`] strategy.
    #[inline]
    #[must_use]
    pub const fn strategy(&self) -> Backoff {
        let &Self { backoff_strategy, .. } = self;

        backoff_strategy
    }

    /// Return the cycle count, absent before the first advance.
    #[inline]
    #[must_use]
    pub const fn cycle(&self) -> Option<NonZero<usize>> {
        let &Self { cycle_count, .. } = self;

        cycle_count
    }
}

#[cfg(test)]
mod tests {
    use core::num::NonZero;

    use super::{Backoff, BackoffState, ExponentialBase};

    #[test]
    fn progression() {
        let target_strategy = Backoff::capped(ExponentialBase::Base4, NonZero::new(10).expect("nonzero ceiling"))
            .expect("ceiling admits the base");
        let mut target_state = target_strategy.state();

        for (target_index, target_magnitude) in [0, 1, 4, 10, 10].into_iter().enumerate() {
            assert_eq!(target_state.advance(), NonZero::new(target_magnitude));
            assert_eq!(target_state.cycle(), NonZero::new(target_index + 1));
        }
    }

    #[test]
    fn saturation() {
        let target_strategy =
            Backoff::capped(ExponentialBase::Base32, NonZero::<usize>::MAX).expect("ceiling admits the base");
        let mut target_state = BackoffState {
            backoff_strategy: target_strategy,
            cycle_count: Some(NonZero::<usize>::MAX),
        };

        for _ in 0..2 {
            assert_eq!(target_state.advance(), Some(NonZero::<usize>::MAX));
            assert_eq!(target_state.cycle(), Some(NonZero::<usize>::MAX));
        }
    }

    #[test]
    fn dispatch() {
        let mut target_state = Backoff::minimal().state();

        Backoff::oneshot();

        for target_count in 1..=3 {
            Backoff::cycle(&mut target_state);
            assert_eq!(target_state.cycle(), NonZero::new(target_count));
        }
    }
}
