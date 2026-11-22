//! Self-contained processor state management.

use core::{cell::UnsafeCell, num::NonZero};

use nekor_domain::{
    domain::arbitrary::Tiable,
    prelude::Arbitrary,
    zeroed::{Zeroable, Zeroed},
};

/// An allocated processor stack.
#[cfg_attr(any(target_arch = "x86", target_arch = "x86_64"), repr(C, align(16)))]
// NOTE(invariant): The non-Freeze byte array remains exclusively owned by its active stack user.
pub struct Stack<const N: usize>(
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
// NOTE(invariant): The non-Freeze byte array remains exclusively owned by its active shadow-stack
// user.
pub struct ShadowStack<const N: usize>(
    // Hardware writes use interior-mutable backing without exposing shared byte references.
    UnsafeCell<[u8; N]>,
);

// SAFETY: A shadow stack can always be zero-initialized, as it is a sequence of
// bytes.
unsafe impl<const N: usize> Zeroable for ShadowStack<N> {}

// SAFETY: This has the same soundness justification as the `Sync`
// implementation for `Stack`
unsafe impl<const N: usize> Sync for ShadowStack<N> {}

/// A processor stack-area.
#[repr(C)]
// NOTE(invariant): The nonempty area exclusively owns usable execution-stack backing, with
// guard geometry or a proved execution bound established by its unsafe admission.
pub struct StackArea(NonZero<usize>, &'static UnsafeCell<[u8]>);

impl StackArea {
    /// Take exclusive ownership of static writable stack backing.
    ///
    /// Empty storage is rejected. The entry layer accounts for alignment before using the area.
    ///
    /// # Safety
    ///
    /// * The backing must be admitted for every execution placed on it. Guard mappings or a proved
    ///   bound must prevent stack use from reaching memory outside the supplied storage.
    /// * Asynchronous delivery on the stack must fit the same admission or use separate backing.
    #[inline]
    pub const unsafe fn slice(storage: &'static mut [u8]) -> Option<Self> {
        match NonZero::new(storage.len()) {
            Some(stack_size) => {
                let storage = UnsafeCell::from_mut(storage);

                Some(Self(stack_size, storage))
            },
            None => None,
        }
    }

    /// Allocate a `N`-byte processor stack, tying it to a target pseudo-Domain.
    ///
    /// `N` must be nonzero. A zero value fails constant evaluation when this generic instance is
    /// code-generated.
    ///
    /// # Safety
    ///
    /// * The storage identity must not be accessed by another stack user while the area is live.
    /// * The storage must stay writable and satisfy the execution and asynchronous-delivery
    ///   admission requirements of [`Self::slice`].
    #[inline]
    #[must_use]
    pub unsafe fn new<const N: usize, T>() -> Self
    where
        T: Tiable,
    {
        // NOTE: const-eval will raise a compile-time error if `N = 0`.
        let stack_size = const { NonZero::<usize>::new(N).expect("invariant: stack size is nonzero") };

        let &Stack(ref target_value) = Zeroed::explicit_in::<Stack<N>, Arbitrary<T>>();

        Self(stack_size, target_value)
    }

    /// Return the size of this stack area, in bytes.
    #[inline]
    #[must_use]
    pub const fn size(&self) -> usize {
        let &Self(stack_size, ..) = self;

        stack_size.get()
    }
}
