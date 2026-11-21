use core::{
    marker,
    mem::MaybeUninit,
    num::NonZero,
    ops::{Deref, DerefMut},
};

use nekor_aal::cache::prelude::CachePadded;
use nekor_domain::zeroed::Zeroable;

use crate::{limit::Cores, prelude::CoreId};

/// A distinct value type used by processor-local APIs.
///
/// Access borrows the stored value. This type does not select a processor or prevent thread
/// migration, so the surrounding API must establish any processor-local access rules.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
#[repr(transparent)]
// NOTE(invariant): None.
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

/// A cache-padded storage slot for each configured processor identifier.
///
/// Slots contain [`MaybeUninit`] because this interface does not provide initialized `T`
/// references. [`Self::mine`] selects a slot through the current processor's installed
/// [`CoreId`]. [`Self::core`] selects it explicitly.
#[derive(Debug)]
#[repr(transparent)]
// NOTE(invariant): Each array position is distinct per-CPU storage and may remain uninitialized.
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
        Self([const { CachePadded::new(MaybeUninit::<T>::zeroed()) }; NonZero::get(Cores::maximum())])
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
        let Self(target_storage) = self;

        // SAFETY: `core_id` is always in the range `0..Cores::maximum()`, and
        // therefore always in range.
        unsafe { target_storage.get_unchecked(*target_core) }
    }
}

// SAFETY: This is safe, as `PerCpu` never relies on the value of the
// `MaybeUninit` array.
unsafe impl<T> Zeroable for PerCpu<T> where T: Zeroable {}
