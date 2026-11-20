//! x86 monitor delegates selected through an *Eventual Patchsite*.
//!
//! [`Dispatch`] selects the backend. [`Operation`] arms a monitor or waits with backoff.

use core::{convert::Infallible, ptr::NonNull};

use nekor_aal_feature::{
    arch::x86::qualified,
    prelude::{Feature, Present, Signal},
};
use nekor_aal_hotpatch::prelude::{Chosen, Delegated, Delegator, Eventual, Patch};
use nekor_backoff::{
    arch::x86::{Delay, Pause, Relaxation},
    backoff::Backoff,
};

use crate::arch::x86::{self, Cstate};

/// The patch selector for monitor engagement and waiting.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Dispatch {
    /// The privileged `monitor`/`mwait` pair.
    #[cfg(not(usermode))]
    #[cfg_attr(all(target_os = "none", not(usermode)), default)]
    MonitorMwait,

    /// Backoff without hardware monitoring.
    #[cfg_attr(any(usermode, not(target_os = "none")), default)]
    Backoff,

    /// The AMD `monitorx`/`mwaitx` pair.
    MonitorMwaitX,

    /// The user-level `umonitor`/`umwait` pair.
    UmonitorMwait,
}

impl Dispatch {
    /// Check instruction support on the current CPU.
    ///
    /// # Remarks
    ///
    /// A diversion requires support on every executing CPU. [`MonitorMwait`] also
    /// requires *CPL0*. Timed [`MonitorMwaitX`] and [`UmonitorMwait`] waits require *TSC* access.
    #[inline]
    #[must_use]
    pub fn supported(self) -> bool {
        match self {
            Self::Backoff => true,
            Self::MonitorMwaitX => Signal::is(&qualified::MonitorX::supported(), Present::Yes),
            Self::UmonitorMwait => Signal::is(&qualified::Waitpkg::supported(), Present::Yes),
            #[cfg(not(usermode))]
            Self::MonitorMwait => Signal::is(&qualified::Monitor::supported(), Present::Yes),
        }
    }
}

/// An operation through the shared monitor patchsite.
#[derive(Clone, Copy)]
pub enum Operation {
    /// Arm observation of a live value's address.
    Arm(NonNull<u8>),

    /// Wait for a write or wake event using platform wait quanta.
    /// [`Delay::Hint`] relaxes once without a hardware wait.
    Wait(Delay),
}

// SAFETY: Selection maps a static `Dispatch` value. The kernel default requires admitted
// `monitor`/`mwait` at CPL0. Optional delegates require admission on every executing CPU.
unsafe impl Delegator for Dispatch {
    #[cfg(all(target_os = "none", not(usermode)))]
    type Target = MonitorMwait;
    #[cfg(any(not(target_os = "none"), usermode))]
    type Target = ManualBackoff;
    type Value = Self;

    #[inline]
    fn choose(target_value: &'static Self::Value) -> Chosen<Self, Operation, ()> {
        match *target_value {
            Self::Backoff => Chosen::delegated::<ManualBackoff>(),
            Self::MonitorMwaitX => Chosen::delegated::<MonitorMwaitX>(),
            Self::UmonitorMwait => Chosen::delegated::<UmonitorMwait>(),
            #[cfg(not(usermode))]
            Self::MonitorMwait => Chosen::delegated::<MonitorMwait>(),
        }
    }
}

// SAFETY: Delegates share `Operation` and remain resident. Old and new delegates must be admitted
// on every executing CPU while a diversion is stale. Callers recheck after engagement and waking.
// Switching delegates between engagement and waiting may cause spurious or lost monitor events.
unsafe impl Eventual for Dispatch {}

/// Backoff without hardware monitoring.
#[derive(Clone, Copy)]
pub enum ManualBackoff {
    /// Keep the delegate namespace uninhabited.
    __Variant(Infallible),
}

// SAFETY: Arming is a no-op. Waiting delegates to `Relaxation`.
unsafe impl Delegated for ManualBackoff {
    type Input = Operation;
    type Output = ();

    #[inline]
    fn implementation(target_operation: Operation) {
        match target_operation {
            Operation::Arm(..) => (),
            Operation::Wait(target_delay) => Patch::<Relaxation, Relaxation>::run(target_delay),
        }
    }
}

/// The privileged `monitor`/`mwait` delegate.
#[derive(Clone, Copy)]
pub enum MonitorMwait {
    /// Keep the delegate namespace uninhabited.
    __Variant(Infallible),
}

// SAFETY: Callers admit `monitor`/`mwait` at CPL0. Waiting follows engagement on the same CPU.
unsafe impl Delegated for MonitorMwait {
    type Input = Operation;
    type Output = ();

    #[inline]
    fn implementation(target_operation: Operation) {
        match target_operation {
            Operation::Arm(target_address) => {
                // SAFETY: The kernel admits `monitor` at CPL0. A live `Monitor` supplies the
                // address.
                unsafe { x86::monitor(target_address) };
            },
            Operation::Wait(Delay::Hint) => Backoff::oneshot(),
            Operation::Wait(Delay::Timed(..)) => {
                // SAFETY: The kernel admits `mwait` at CPL0 on this CPU.
                unsafe { x86::mwait() };
            },
        }
    }
}

/// The AMD `monitorx`/`mwaitx` delegate.
#[derive(Clone, Copy)]
pub enum MonitorMwaitX {
    /// Keep the delegate namespace uninhabited.
    #[doc(hidden)]
    __Variant(Infallible),
}

// SAFETY: Installation admits `monitorx`/`mwaitx` and TSC access on every executing CPU.
// Waiting follows engagement on the same CPU.
unsafe impl Delegated for MonitorMwaitX {
    type Input = Operation;
    type Output = ();

    #[inline]
    fn implementation(target_operation: Operation) {
        match target_operation {
            Operation::Arm(target_address) => {
                // SAFETY: Installation admits `monitorx` on this CPU. A live `Monitor` supplies the
                // address.
                unsafe { x86::monitorx(target_address) };
            },
            Operation::Wait(Delay::Hint) => Backoff::oneshot(),
            Operation::Wait(Delay::Timed(target_quantum_count)) => {
                // SAFETY: The platform admits TSC access for `Pause::measure`.
                let target_quantum = unsafe { Pause::measure() };
                let target_delay = Delay::ticks(target_quantum_count, target_quantum);

                // SAFETY: Installation admits `mwaitx` on this CPU after matching `monitorx`
                // engagement.
                unsafe { x86::mwaitx(u32::try_from(target_delay).unwrap_or(u32::MAX)) };
            },
        }
    }
}

/// The user-level *WAITPKG* delegate.
#[derive(Clone, Copy)]
pub enum UmonitorMwait {
    /// Keep the delegate namespace uninhabited.
    __Variant(Infallible),
}

// SAFETY: Installation admits WAITPKG and TSC access on every executing CPU.
// Waiting follows engagement on the same CPU.
unsafe impl Delegated for UmonitorMwait {
    type Input = Operation;
    type Output = ();

    #[inline]
    fn implementation(target_operation: Operation) {
        match target_operation {
            Operation::Arm(target_address) => {
                // SAFETY: Installation admits `umonitor` on this CPU. A live `Monitor` supplies the
                // address.
                unsafe { x86::umonitor(target_address) };
            },
            Operation::Wait(Delay::Hint) => Backoff::oneshot(),
            Operation::Wait(Delay::Timed(target_quantum_count)) => {
                // SAFETY: Installation admits TSC access for `Pause::measure` on this CPU.
                let target_quantum = unsafe { Pause::measure() };
                let target_delay = Delay::ticks(target_quantum_count, target_quantum);

                // SAFETY: Installation admits `rdtsc` access on this CPU.
                let target_deadline = unsafe { x86::rdtsc() }.saturating_add(target_delay);

                // SAFETY: Installation admits `umwait` and TSC access after matching `umonitor`
                // engagement.
                unsafe { x86::umwait(Cstate::C0_1, target_deadline) };
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use core::hint::black_box;

    use nekor_aal_hotpatch::prelude::{Delegated, Delegator};

    use super::{Dispatch, ManualBackoff, MonitorMwait, MonitorMwaitX, Operation, UmonitorMwait};

    #[test]
    fn each_backend_can_be_chosen() {
        static BACKOFF: Dispatch = Dispatch::Backoff;
        #[cfg(not(usermode))]
        static MONITOR: Dispatch = Dispatch::MonitorMwait;
        static MONITORX: Dispatch = Dispatch::MonitorMwaitX;
        static UMONITOR: Dispatch = Dispatch::UmonitorMwait;

        black_box(Dispatch::choose(&BACKOFF));
        #[cfg(not(usermode))]
        black_box(Dispatch::choose(&MONITOR));
        black_box(Dispatch::choose(&MONITORX));
        black_box(Dispatch::choose(&UMONITOR));
    }

    #[test]
    fn hardware_backends_are_linked() {
        black_box(<ManualBackoff as Delegated>::implementation as fn(Operation));
        black_box(<MonitorMwait as Delegated>::implementation as fn(Operation));
        black_box(<MonitorMwaitX as Delegated>::implementation as fn(Operation));
        black_box(<UmonitorMwait as Delegated>::implementation as fn(Operation));
    }
}
