//! Host monitor and backoff benchmarks.
//!
//! # Requirements
//!
//! Every worker CPU must support the selected instructions and permit `RDTSC`.
//! Detection checks only the current CPU.

use core::iter;

use nekor_aal_hotpatch::{
    cmc::Publish,
    prelude::{Chosen, Delegator},
};
use nekor_aal_signal::arch::x86::monitor::{Dispatch, Operation};
use nekor_backoff::arch::x86::{Delay, Relaxation};

/// Backoff without hardware monitoring.
static BACKOFF: Dispatch = Dispatch::Backoff;
/// User-level `UMONITOR`/`UMWAIT`, requiring `WAITPKG`.
static UMONITOR: Dispatch = Dispatch::UmonitorMwait;
/// AMD `MONITORX`/`MWAITX`, available at any privilege level.
static MONITORX: Dispatch = Dispatch::MonitorMwaitX;

/// The `PAUSE` backoff delegate.
static PAUSE: Relaxation = Relaxation::Pause;
/// The `TPAUSE` delegate, requiring `WAITPKG`.
static TPAUSE: Relaxation = Relaxation::Tpause;

/// A benchmark monitor and backoff configuration.
#[derive(Clone, Copy)]
// NOTE(invariant): Both static selections are supported on the benchmark host.
pub struct Configuration {
    /// The [`Dispatch`] used for engagement and waiting.
    monitor: &'static Dispatch,

    /// The backoff [`Relaxation`] delegate.
    backoff: &'static Relaxation,

    /// The monitor's benchmark label.
    pub monitor_name: &'static str,

    /// The backoff's benchmark label.
    pub backoff_name: &'static str,
}

/// Supported usermode configurations, including the backoff baseline.
pub fn configurations() -> Vec<Configuration> {
    let monitors = [
        (&BACKOFF, "backoff"),
        (&UMONITOR, "umonitor_umwait"),
        (&MONITORX, "monitorx_mwaitx"),
    ];
    let backoffs = [(&PAUSE, "pause"), (&TPAUSE, "tpause")];
    let mut configurations = Vec::new();

    for (monitor, monitor_name) in monitors {
        if monitor.supported() {
            for (backoff, backoff_name) in backoffs {
                if backoff.supported() {
                    configurations.push(Configuration {
                        monitor,
                        backoff,
                        monitor_name,
                        backoff_name,
                    });
                }
            }
        }
    }

    configurations
}

impl Configuration {
    /// Install both delegates between worker groups.
    ///
    /// Workers must be joined before installation and created after publication.
    /// [`Publish::now`] serializes the local CPU only.
    pub fn install(self) {
        let Self { monitor, backoff, .. } = self;

        let target_monitor = Dispatch::choose(monitor);
        let target_backoff = Relaxation::choose(backoff);

        // SAFETY: The host admits both delegates on all worker CPUs and permits
        // `RDTSC`. Patch storage is writable and uses the small code model.
        // No workers exist during patching or publication.
        unsafe {
            assert!(
                Chosen::<Dispatch, Operation, ()>::patchsite()
                    .stale(target_monitor)
                    .is_ok(),
                "monitor dispatch patchsite changed concurrently"
            );
            assert!(
                Chosen::<Relaxation, Delay, ()>::patchsite()
                    .stale(target_backoff)
                    .is_ok(),
                "backoff delay patchsite changed concurrently"
            );
        }

        Publish::now(iter::empty());
    }
}
