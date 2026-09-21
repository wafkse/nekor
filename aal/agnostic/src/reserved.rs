//! Reserved storage for low-level representations.

use core::mem::MaybeUninit;

/// Storage for an architecturally reserved field.
///
/// The wrapper deliberately does not assign meaning to the stored bytes. It can
/// carry initialized bytes verbatim or remain uninitialized when the enclosing
/// representation permits that state.
#[derive(Debug)]
#[repr(transparent)]
pub struct Reserved<S>(MaybeUninit<S>);

impl<S> Copy for Reserved<S> where S: Copy {}

impl<S> Clone for Reserved<S>
where
    S: Copy,
{
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Reserved<S> {
    /// Constructs reserved storage from an initialized value.
    #[inline]
    #[must_use]
    pub const fn new(target_value: S) -> Self {
        Self(MaybeUninit::new(target_value))
    }

    /// Constructs uninitialized reserved storage.
    #[inline]
    #[must_use]
    pub const fn uninit() -> Self {
        Self(MaybeUninit::uninit())
    }

    /// Constructs zeroed reserved storage.
    #[inline]
    #[must_use]
    pub const fn zeroed() -> Self {
        Self(MaybeUninit::zeroed())
    }

    /// Constructs zeroed reserved storage.
    ///
    /// This name is retained for compatibility with the original API.
    #[inline]
    #[must_use]
    pub const fn field() -> Self {
        Self::zeroed()
    }

    /// Wraps existing reserved storage without changing its bytes.
    #[inline]
    #[must_use]
    pub const fn wrap(storage: MaybeUninit<S>) -> Self {
        Self(storage)
    }

    /// Returns the underlying reserved storage.
    #[inline]
    #[must_use]
    pub const fn storage(&self) -> &MaybeUninit<S> {
        let &Self(ref storage) = self;

        storage
    }

    /// Returns mutable access to the underlying reserved storage.
    #[inline]
    pub const fn storage_mut(&mut self) -> &mut MaybeUninit<S> {
        let &mut Self(ref mut storage) = self;

        storage
    }

    /// Unwraps the reserved storage without reading it.
    #[inline]
    #[must_use]
    pub const fn into_storage(self) -> MaybeUninit<S> {
        let Self(storage) = self;

        storage
    }
}

impl<S> Default for Reserved<S> {
    #[inline]
    fn default() -> Self {
        Self::zeroed()
    }
}

#[cfg(test)]
mod tests {
    use core::{mem, mem::MaybeUninit};

    use super::Reserved;

    #[test]
    fn reserved_storage_preserves_initialized_bytes() {
        let reserved = Reserved::new(0x89ab_cdef_u32);
        let wrapped = Reserved::wrap(MaybeUninit::new(0x7654_3210_u32));

        // SAFETY: Both values were constructed from initialized u32 values.
        assert_eq!(unsafe { reserved.into_storage().assume_init() }, 0x89ab_cdef);
        // SAFETY: Both values were constructed from initialized u32 values.
        assert_eq!(unsafe { wrapped.into_storage().assume_init() }, 0x7654_3210);
    }

    #[test]
    fn reserved_storage_matches_the_wrapped_layout() {
        assert_eq!(mem::size_of::<Reserved<u32>>(), mem::size_of::<u32>());
        assert_eq!(mem::align_of::<Reserved<u32>>(), mem::align_of::<u32>());
    }
}
