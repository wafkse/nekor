//! Distinct static storage for the same value type.
//!
//! A [`Domain`] supplies a transparent [`Adapter`] whose type selects the storage symbol.
//! Two domains can therefore store independent instances of the same `T`. Calls without an
//! explicit domain use [`preset::Preset`], while [`arbitrary::Arbitrary`] uses another type
//! as the storage identity.

use crate::store::Store;

pub mod preset;

pub mod arbitrary;

/// An adapter over a type `T` that represents inclusion in some [`Domain`].
///
/// # Safety
///
/// - The adaptor must be `repr(transparent)` over [`Adapter::Target`], i.e., have an identical
///   layout and ABI constraints.
pub unsafe trait Adapter: Store {
    /// The target type of this adapter.
    type Target;
}

/// A 'static'-ness domain that encompasses a set of unique static instances of
/// arbitrary types.
pub trait Domain: 'static {
    /// The adapter type over a target type `T` that represents the current
    /// domain.
    type Adapter<T>: Adapter<Target = T>
    where
        T: Store;
}
