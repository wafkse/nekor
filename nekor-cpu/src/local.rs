use core::{
    marker,
    mem::MaybeUninit,
    num::NonZero,
    ops::{Deref, DerefMut},
};

use nekor_domain::zeroed::Zeroable;

use nekor_aal::cache::prelude::CachePadded;

use crate::{limit::Cores, prelude::CoreId};

/// A transparent wrapper over a value that is known to be cpu-local.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
#[repr(transparent)]
pub struct Local<T>(T, marker::PhantomData<fn() -> *mut T>);

impl<T> Deref for Local<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        let Self(target_value, ..) = self;

        target_value
    }
}

impl<T> DerefMut for Local<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        let &mut Self(ref mut target_value, ..) = self;

        target_value
    }
}

/// A type to englobe all per-cpu data structures.
///
/// This is a low-level type with no initialization guarantees. Use the
/// [`Local`] type.
#[derive(Debug)]
#[repr(transparent)]
pub struct PerCpu<T>([CachePadded<MaybeUninit<T>>; NonZero::get(Cores::maximum())])
where
    T: Zeroable;

impl<T> PerCpu<T>
where
    T: Zeroable,
{
    /// Construct a new per-cpu array for `T`.
    #[inline]
    #[must_use]
    pub const fn array() -> Self {
        Self(
            [const { CachePadded::new(MaybeUninit::<T>::zeroed()) };
                NonZero::get(Cores::maximum())],
        )
    }

    /// Retrieve the maybe-uninitialized per-CPU value `T` for this core.
    #[inline]
    pub fn mine(&self) -> &CachePadded<MaybeUninit<T>> {
        Self::core(self, CoreId::mine())
    }

    /// Retrieve the maybe-uninitialized per-CPU value `T` for the provided
    /// [`CoreId`].
    #[inline]
    pub fn core(&self, target_core: CoreId) -> &CachePadded<MaybeUninit<T>> {
        let Self(processor_array) = self;

        // SAFETY: `core_id` is always in the range `0..Cores::maximum()`, and
        // therefore always in range.
        unsafe { processor_array.get_unchecked(*target_core) }
    }
}

// SAFETY: This is safe, as `PerCpu` never relies on the value of the
// `MaybeUninit` array.
unsafe impl<T> Zeroable for PerCpu<T> where T: Zeroable {}
