//! The default, global `static`-ness domain.
//!
//! See [`Preset`] for further information.

use crate::store::Store;

use super::{Adapter, Domain};

/// The default implicit [`Domain`] for all type `T`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Preset {}

impl Domain for Preset {
    type Adapter<T>
        = Included<T>
    where
        T: Store;
}

/// The adaptor type for the [`Preset`] domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Included<T>(T)
where
    T: Store;

/// SAFETY: [`Included`] is `repr(transparent)` over [`Adapter::Target`].
unsafe impl<T> Adapter for Included<T>
where
    T: Store,
{
    type Target = T;
}
