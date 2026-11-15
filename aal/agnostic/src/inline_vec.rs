//! Fixed-capacity vectors with inline storage.

use core::{
    fmt,
    mem::{self, MaybeUninit},
};

/// An inline vector of at most `N` elements. Unused slots are uninitialized.
// NOTE(invariant): The length bounds the backing's initialized prefix and never exceeds `N`.
// Each initialized element is owned exclusively by this vector.
pub struct InlineVec<T, const N: usize>(
    /// Inline backing including its unused, potentially uninitialized suffix.
    [MaybeUninit<T>; N],
    /// The number of initialized elements.
    usize,
);

impl<T, const N: usize> InlineVec<T, N> {
    /// Create an empty vector without initializing unused element storage.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self([const { MaybeUninit::uninit() }; N], usize::MIN)
    }

    /// Number of initialized elements.
    #[inline]
    #[must_use]
    pub const fn size(&self) -> usize {
        let &Self(.., target_size) = self;

        target_size
    }

    /// Whether this vector has no initialized elements.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.size() == usize::MIN
    }

    /// Maximum number of owned elements.
    #[inline]
    #[must_use]
    pub const fn capacity(&self) -> usize {
        N
    }

    /// Append an owned element or return it unchanged when the vector is full.
    #[inline]
    pub fn push(&mut self, target_value: T) -> Result<(), T> {
        let Self(target_storage, target_size) = self;

        match target_storage.get_mut(*target_size) {
            Some(target_slot) => {
                target_slot.write(target_value);

                *target_size += 1;

                Ok(())
            },
            None => Err(target_value),
        }
    }

    /// Remove and return the final owned element.
    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        let Self(storage, target_size) = self;

        match target_size.checked_sub(1) {
            Some(target_index) => {
                let target_slot = storage.get(target_index)?;

                *target_size = target_index;

                // SAFETY: This slot belonged to the initialized prefix before its length
                // was shortened. Removing it from that prefix transfers ownership to the
                // returned value, so the vector will not drop the same element again.
                Some(unsafe { target_slot.assume_init_read() })
            },
            None => None,
        }
    }

    /// Borrow the initialized prefix of the [`InlineVec`].
    #[inline]
    #[must_use]
    pub const fn as_slice(&self) -> &[T] {
        let Self(target_storage, target_slice) = self;

        let (target_slice, ..) = target_storage.split_at(*target_slice);

        // SAFETY: Every element below the stored length is initialized, as required by
        // the vector's invariant. The slice covers only those elements and borrows them
        // through the shared backing reference, so no uninitialized slot is exposed.
        unsafe { target_slice.assume_init_ref() }
    }

    /// Mutably borrow the initialized prefix of the [`InlineVec`].
    #[inline]
    pub const fn as_mut_slice(&mut self) -> &mut [T] {
        let Self(target_storage, target_size) = self;

        let (target_slice, ..) = target_storage.split_at_mut(*target_size);

        // SAFETY: Every element below the stored length is initialized, as required by
        // the vector's invariant. The exclusive backing borrow covers the returned slice,
        // which therefore exposes no uninitialized slot or overlapping mutable reference.
        unsafe { target_slice.assume_init_mut() }
    }

    /// Drop the initialized prefix and return the vector to its empty state.
    #[inline]
    pub fn clear(&mut self) {
        let Self(target_storage, target_size) = self;

        let target_size = mem::take(target_size);

        let (target_slice, ..) = target_storage.split_at_mut(target_size);

        // SAFETY: The slice contains exactly the initialized elements owned by the vector.
        // Its length has already been cleared, so a panicking element destructor cannot
        // leave those elements available for a second drop through the vector.
        unsafe { target_slice.assume_init_drop() };
    }
}

impl<T, const N: usize> Default for InlineVec<T, N> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> Drop for InlineVec<T, N> {
    #[inline]
    fn drop(&mut self) {
        Self::clear(self);
    }
}

impl<T, const N: usize> fmt::Debug for InlineVec<T, N>
where
    T: fmt::Debug,
{
    #[inline]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_list().entries(Self::as_slice(self)).finish()
    }
}

#[cfg(test)]
mod tests {
    //! Ownership and initialized-prefix checks for `InlineVec`.

    use core::cell::Cell;

    use crate::inline_vec::InlineVec;

    /// Count destruction of values whose ownership moves through the initialized prefix.
    // NOTE(invariant): None.
    struct Counted<'counter>(&'counter Cell<usize>);

    impl Drop for Counted<'_> {
        fn drop(&mut self) {
            let Self(counter) = self;

            counter.set(counter.get() + 1);
        }
    }

    #[test]
    fn capacity_rejection_and_prefix_operations_preserve_ownership() {
        let counter = Cell::new(0);
        let mut vector = InlineVec::<_, 2>::new();

        assert!(vector.push(Counted(&counter)).is_ok());
        assert!(vector.push(Counted(&counter)).is_ok());

        let rejected = vector.push(Counted(&counter));

        assert_eq!(counter.get(), 0);
        assert_eq!(vector.size(), 2);
        assert_eq!(vector.as_slice().len(), 2);

        drop(rejected);
        drop(vector.pop());

        assert_eq!(counter.get(), 2);
        assert_eq!(vector.size(), 1);

        vector.clear();

        assert_eq!(counter.get(), 3);
        assert!(vector.is_empty());
        assert!(vector.push(Counted(&counter)).is_ok());

        drop(vector);

        assert_eq!(counter.get(), 4);
    }

    #[test]
    fn zero_capacity_and_zero_sized_elements_have_valid_prefixes() {
        let mut empty = InlineVec::<(), 0>::new();

        assert_eq!(empty.push(()), Err(()));
        assert_eq!(empty.pop(), None);
        assert!(empty.as_mut_slice().is_empty());

        let mut zero_sized = InlineVec::<(), 3>::new();

        for _ in 0..zero_sized.capacity() {
            assert_eq!(zero_sized.push(()), Ok(()));
        }

        assert_eq!(zero_sized.as_slice().len(), 3);
        assert_eq!(zero_sized.pop(), Some(()));
        assert_eq!(zero_sized.size(), 2);
    }

    /// Count every destruction while one element interrupts a clear through unwinding.
    // NOTE(invariant): None.
    struct Panicking<'counter> {
        /// Shared number of elements destroyed.
        counter: &'counter Cell<usize>,

        /// Whether this element's destructor panics.
        panics: bool,
    }

    impl Drop for Panicking<'_> {
        fn drop(&mut self) {
            let Self { counter, panics } = self;

            counter.set(counter.get() + 1);
            assert!(!*panics, "fixture destructor panic");
        }
    }

    #[test]
    fn a_panicking_destructor_cannot_leave_an_initialized_prefix_to_drop_twice() {
        let counter = Cell::new(0);
        let mut vector = InlineVec::<_, 3>::new();

        for panics in [true, false, false] {
            let element = Panicking {
                counter: &counter,
                panics,
            };

            assert!(vector.push(element).is_ok());
        }

        let outcome = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| vector.clear()));

        assert!(outcome.is_err());
        assert!(vector.is_empty());
        assert_eq!(counter.get(), 3);

        drop(vector);

        assert_eq!(counter.get(), 3);
    }
}
