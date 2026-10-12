//! Processor relaxation for `x86` and `x86_64` backoff.

use core::{convert::Infallible, num::NonZero};

use nekor_aal_arch::x86::instruction::{
    info,
    relax::{self, Cstate},
};
use nekor_aal_feature::{
    arch::x86::qualified::Waitpkg,
    prelude::{Feature, Present, Signal},
};
use nekor_aal_hotpatch::prelude::{Chosen, Delegated, Delegator, Eventual};
use nekor_platform::var;

/// A [`Delegator`] selecting processor relaxation for backoff or power saving.
///
/// # Installation
///
/// Installation must admit instruction support and TSC access on every CPU
/// that can execute the delegate. [`Self::supported`] checks only the current CPU.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub enum Relaxation {
    /// Poll the TSC between configured chains of `pause` instructions.
    ///
    /// Older CPUs may treat `pause` as `nop`. [`Delay::Hint`] issues a single `pause`.
    #[default]
    Pause,

    /// Wait with `tpause` using a quantum-derived TSC deadline.
    ///
    /// [`Delay::Hint`] uses `pause` because `tpause` requires a deadline.
    Tpause,
}

/// A relaxation request, independent of the selected [`Relaxation`] implementation.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub enum Delay {
    /// Perform a single `pause` instruction without reading the TSC.
    #[default]
    Hint,

    /// Request a delay in platform wait quanta.
    ///
    /// # Timing
    ///
    /// Each request samples the absolute TSC change across a configured `pause` chain.
    /// The sample includes `RDTSC` overhead without serialization. It is an estimate,
    /// not a minimum duration, as waits may return early, including on backward TSC movement.
    ///
    /// # Bounds
    ///
    /// Both modes clamp the requested ticks to [`Relaxation::TIMESTAMP_LIMIT`].
    /// [`Relaxation::Pause`] executes at most the requested chain count after sampling,
    /// even if the TSC stops.
    Timed(NonZero<usize>),
}

impl Delay {
    /// Convert a quantum count and sample to TSC ticks, capped at [`Relaxation::TIMESTAMP_LIMIT`].
    ///
    /// A zero-length sample is treated as one tick. Multiplication saturates.
    #[inline]
    #[must_use]
    pub fn ticks(target_quantum_count: NonZero<usize>, target_quantum: u64) -> u64 {
        // NOTE: Virtualized or coarse counters may sample zero.
        // `x86` and `x86_64` words fit in `u64`.
        target_quantum
            .max(1)
            .saturating_mul(target_quantum_count.get() as u64)
            .min(Relaxation::TIMESTAMP_LIMIT)
    }
}

impl Relaxation {
    /// Maximum number of `pause` chains performed between TSC checks.
    ///
    /// Zero skips further polling after measuring the wait quantum.
    pub const CHAIN_COUNT: usize = var::integer!("NEKOR_BACKOFF_X86_CHAINS_PER_CHECK");
    /// Number of consecutive `pause` instructions in a platform wait quantum.
    pub const PAUSE_COUNT: usize = var::integer!("NEKOR_BACKOFF_X86_PAUSES_PER_CHAIN");
    /// Maximum requested x86 wait duration in TSC ticks.
    ///
    /// Hardware may return from a wait earlier than requested.
    pub const TIMESTAMP_LIMIT: u64 = var::integer!("NEKOR_BACKOFF_X86_MAX_DELAY_TICKS");

    /// Check instruction support on the current CPU, not TSC access or other CPUs.
    ///
    /// See [`Relaxation`] for installation requirements.
    #[inline]
    #[must_use]
    pub fn supported(self) -> bool {
        match self {
            Self::Pause => true,
            Self::Tpause => Signal::is(&Waitpkg::supported(), Present::Yes),
        }
    }
}

// SAFETY: Selection maps fixed modes without side effects. Installing `Tpause` requires `WAITPKG`
// on every possible CPU. Both delegates require per-CPU `RDTSC` admission for `Delay::Timed`.
unsafe impl Delegator for Relaxation {
    type Target = Pause;
    type Value = Self;

    #[inline]
    fn choose(target_value: &'static Self::Value) -> Chosen<Self, Delay, ()> {
        match *target_value {
            Self::Pause => Chosen::delegated::<Pause>(),
            Self::Tpause => Chosen::delegated::<Tpause>(),
        }
    }
}

// SAFETY: `Pause` and `Tpause` retain no caller state. Calls through earlier
// diversions can finish alongside new calls while their code remains resident.
unsafe impl Eventual for Relaxation {}

/// Execute a single `pause`, or poll the TSC after configured chains of `pause` hints.
#[derive(Clone, Copy)]
pub enum Pause {
    /// Prevent construction of the delegate namespace.
    __Variant(Infallible),
}

impl Pause {
    /// Measure the absolute TSC change across the platform's configured pause chain.
    ///
    /// Includes `RDTSC` overhead without serializing instruction execution.
    ///
    /// # Safety
    ///
    /// The execution environment must permit `RDTSC` at the current privilege level.
    #[inline]
    #[must_use]
    pub unsafe fn measure() -> u64 {
        // SAFETY: The caller admits `RDTSC` at the current privilege level.
        let target_start_time = unsafe { info::rdtsc() };

        relax::chain::<{ Relaxation::PAUSE_COUNT }, 1>();

        // SAFETY: `RDTSC` admission holds through the measurement.
        let target_end_time = unsafe { info::rdtsc() };

        target_end_time.abs_diff(target_start_time)
    }

    /// Poll the supplied counter after full batches, then execute any remaining chains.
    #[inline]
    fn wait<const PAUSE_COUNT: usize, const CHAIN_COUNT: usize>(
        target_quanta: NonZero<usize>,
        target_desire: u64,
        mut target_count: impl FnMut() -> u64,
    ) {
        if CHAIN_COUNT == 0 {
            return;
        }

        let batch_count = target_quanta.get() / CHAIN_COUNT;
        let remainder_count = target_quanta.get() % CHAIN_COUNT;
        let start_time = target_count();

        for _ in 0..batch_count {
            relax::chain::<PAUSE_COUNT, CHAIN_COUNT>();

            // NOTE: `abs_diff` treats backward TSC movement as progress.
            let target_elapsed = target_count().abs_diff(start_time);

            if target_elapsed >= target_desire {
                return;
            }
        }

        // NOTE: Exhausting the chain budget needs no further TSC read.
        for _ in 0..remainder_count {
            relax::chain::<PAUSE_COUNT, 1>();
        }
    }
}

// SAFETY: `PAUSE` needs no feature detection. `Delay::Timed` requires `RDTSC` admission.
unsafe impl Delegated for Pause {
    type Input = Delay;
    type Output = ();

    #[inline]
    fn implementation(target_delay: Delay) {
        match target_delay {
            Delay::Hint => relax::pause(),
            Delay::Timed(target_quantum_count) => {
                // SAFETY: The backoff environment admits `RDTSC`.
                let target_quantum = unsafe { Self::measure() };
                let target_delay = Delay::ticks(target_quantum_count, target_quantum);

                Self::wait::<{ Relaxation::PAUSE_COUNT }, { Relaxation::CHAIN_COUNT }>(
                    target_quantum_count,
                    target_delay,
                    || {
                        // SAFETY: `RDTSC` admission holds through this wait.
                        unsafe { info::rdtsc() }
                    },
                );
            },
        }
    }
}

/// Wait with WAITPKG using a TSC deadline, or issue a single `pause`.
#[derive(Clone, Copy)]
pub enum Tpause {
    /// Prevent construction of the delegate namespace.
    __Variant(Infallible),
}

// SAFETY: The installer admits `WAITPKG` support and TSC access on every CPU
// that can execute `Tpause`.
unsafe impl Delegated for Tpause {
    type Input = Delay;
    type Output = ();

    #[inline]
    fn implementation(target_delay: Delay) {
        match target_delay {
            Delay::Hint => relax::pause(),
            Delay::Timed(target_quantum_count) => {
                // SAFETY: The backoff environment admits `RDTSC`.
                let target_quantum = unsafe { Pause::measure() };
                let target_delay = Delay::ticks(target_quantum_count, target_quantum);

                // SAFETY: The execution environment admits `RDTSC`.
                let target_deadline = unsafe { info::rdtsc() }.saturating_add(target_delay);

                // SAFETY: Installing `Tpause` admits `WAITPKG` support.
                unsafe { relax::tpause(Cstate::C0_1, target_deadline) };
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use core::{hint::black_box, num::NonZero};

    use nekor_aal_hotpatch::prelude::{Delegated, Delegator};

    use super::{Delay, Pause, Relaxation, Tpause};

    #[test]
    fn selection() {
        static PAUSE: Relaxation = Relaxation::Pause;
        static TPAUSE: Relaxation = Relaxation::Tpause;

        black_box(Relaxation::choose(&PAUSE));
        black_box(Relaxation::choose(&TPAUSE));
        black_box(<Pause as Delegated>::implementation as fn(Delay));
        black_box(<Tpause as Delegated>::implementation as fn(Delay));
    }

    #[test]
    fn conversion() {
        assert_eq!(Delay::ticks(NonZero::<usize>::MIN, 0), 1);
        assert_eq!(
            Delay::ticks(NonZero::<usize>::MAX, u64::MAX),
            Relaxation::TIMESTAMP_LIMIT
        );
    }

    #[test]
    #[cfg(not(miri))]
    fn hint() {
        <Pause as Delegated>::implementation(Delay::Hint);
        <Tpause as Delegated>::implementation(Delay::Hint);
    }

    #[test]
    #[cfg(not(miri))]
    fn timed() {
        // SAFETY: The host test environment admits `RDTSC`.
        black_box(unsafe { Pause::measure() });

        <Pause as Delegated>::implementation(Delay::Timed(NonZero::<usize>::MIN));
    }

    #[test]
    #[cfg(not(miri))]
    fn stalled() {
        let quantum_count = NonZero::<usize>::new(4).expect("nonzero quantum count");
        let mut sample_count = 0;

        Pause::wait::<1, 3>(quantum_count, 1, || {
            sample_count += 1;

            42
        });

        assert_eq!(sample_count, 2, "the remainder needs no further counter read");
    }

    #[test]
    #[cfg(not(miri))]
    fn remainder() {
        let quantum_count = NonZero::<usize>::new(2).expect("nonzero quantum count");
        let mut sample_count = 0;

        Pause::wait::<1, 3>(quantum_count, 1, || {
            sample_count += 1;

            42
        });

        assert_eq!(sample_count, 1, "a partial batch exhausts the budget without polling");
    }

    #[test]
    #[cfg(not(miri))]
    fn elapsed() {
        let quantum_count = NonZero::<usize>::new(10).expect("nonzero quantum count");
        let mut target_timestamp = [100, 101, 104].into_iter();

        Pause::wait::<1, 3>(quantum_count, 4, || {
            target_timestamp.next().expect("the wait must stop at its target")
        });

        assert_eq!(target_timestamp.next(), None);
    }

    #[test]
    #[cfg(not(miri))]
    fn backward() {
        let quantum_count = NonZero::<usize>::new(4).expect("nonzero quantum count");
        let mut sample_count = 0;

        Pause::wait::<1, 3>(quantum_count, 10, || {
            sample_count += 1;

            if sample_count == 1 { 100 } else { 80 }
        });

        assert_eq!(sample_count, 2, "absolute counter movement must satisfy the target");
    }

    #[test]
    #[cfg(not(miri))]
    fn zero() {
        let mut sample_count = 0;

        Pause::wait::<1, 0>(NonZero::<usize>::MIN, 1, || {
            sample_count += 1;

            42
        });

        assert_eq!(sample_count, 0);
    }
}
