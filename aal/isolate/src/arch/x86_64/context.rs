//! Low-level architectural entry context for Context Switch code.

use core::marker;

use nekor_aal_agnostic::prelude::{Contextual, LowLevel};

/// A newtype wrapper over the [`LowLevel`] type that is context-specific that
/// describes low-level context-save logic. `CsEntry` stands for `Context-save
/// Entry`.
#[repr(transparent)]
pub struct CsEntry<T>(
    pub LowLevel<T>,
    // NOTE: This does not have an effect on variance, and is only required as a ZST.
    marker::PhantomData<Self>,
);

// SAFETY: This is being implemented for a `LowLevel` wrapper type.
unsafe impl<T> Contextual for CsEntry<T> {}
