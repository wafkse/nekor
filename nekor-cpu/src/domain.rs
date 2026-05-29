//! The [`Domain`] exclusive to the `nekor-cpu` crate.
//!
//! See [`Cpu`] for further information.

use nekor_domain::{
    domain::{Adapter, Domain},
    store::Store,
};

/// A [`Domain`] for cpu-related structures and per-cpu data.
pub enum Cpu {}

impl Domain for Cpu {
    type Adapter<T>
        = InCpu<T>
    where
        T: Store;
}

/// An [`Adapter`] for the [`Cpu`]-local domain.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
#[repr(transparent)]
pub struct InCpu<T>(T)
where
    T: Store;

// SAFETY: `InCpu` is `repr(transparent)` over `T`.
unsafe impl<T> Adapter for InCpu<T>
where
    T: Store,
{
    type Target = T;
}
