//! `static`-ness domains for multiple instantiation of statics of type `T`.
//!
//! This module exposes the trait [`Domain`], which, in short terms, *is a way
//! to constrict the domain a generic static value of `T` belongs to*.
//!
//! This distinction is quite neat, as it allows for types to have utmost
//! control over their self-declared static variables.
//!
//! In the case where many crates may want to share a static variable, the
//! [`Preset`] domain is the *default* if no other [`Domain`] is explicitly
//! declared.
//!
//! [`Preset`]: preset::Preset

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
