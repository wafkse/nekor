//! Self-contained processor state management.

use core::{cell::UnsafeCell, num::NonZero};

use nekor_domain::{
    domain::arbitrary::Tiable,
    prelude::Arbitrary,
    zeroed::{Zeroable, Zeroed},
};

/// A processor context.
pub struct CpuContext {}

/// An allocated processor stack.
#[cfg_attr(any(target_arch = "x86", target_arch = "x86_64"), repr(C, align(16)))]
pub struct Stack<const N: usize>(
    // NOTE: Need to make the actual stack area non-`Freeze`.
    UnsafeCell<[u8; N]>, /* Add page-sized guard to protect against stack overflow.
                          * BoundaryGuard */
);

// SAFETY: A stack can always be zero-initialized, as it is a sequence of bytes.
unsafe impl<const N: usize> Zeroable for Stack<N> {}

// SAFETY: A stack is always only used by a single processor core at any
// particular time, as architectural mechanisms prevent the stack bytes from
// being used when they are logically owned by another thread of execution.
// Furthermore, the stack bytes are non-`Freeze` and they are private
// to this module, maintaining any invariants upheld.
unsafe impl<const N: usize> Sync for Stack<N> {}

/// A shadow stack.
#[cfg_attr(target_arch = "x86", repr(C, align(4)), /* alignment to return address */)]
#[cfg_attr(target_arch = "x86_64", repr(C, align(8)), /* alignment to return address */)]
pub struct ShadowStack<const N: usize>(
    // NOTE: Need to make the actual shadow stack area non-`Freeze`.
    UnsafeCell<[u8; N]>,
);

// SAFETY: A shadow stack can always be zero-initialized, as it is a sequence of
// bytes.
unsafe impl<const N: usize> Zeroable for ShadowStack<N> {}

// SAFETY: This posseses the same soundness justification as the `Sync`
// implementation for `Stack`
unsafe impl<const N: usize> Sync for ShadowStack<N> {}

/// A processor stack-area.
#[repr(C)]
pub struct StackArea(NonZero<usize>, &'static UnsafeCell<[u8]>);

impl StackArea {
    /// Allocate a `N`-byte processor stack, tying it to a target pseudo-Domain.
    ///
    /// # Safety
    ///
    /// The created [`StackArea`] must be used in a sound and thread-safe
    /// manner.
    #[inline]
    #[must_use]
    pub unsafe fn new<const N: usize, T>() -> Self
    where
        T: Tiable,
    {
        let Stack(target_value) = Zeroed::explicit_in::<Stack<N>, Arbitrary<T>>();

        Self(
            const { NonZero::<usize>::new(N).expect("cannot have zero-sized stack") },
            target_value,
        )
    }
}
