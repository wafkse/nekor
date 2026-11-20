use core::{
    fmt,
    ops::{Deref, DerefMut},
};

/// Storage aligned to the cache-padding size chosen for the target architecture.
///
/// Separating independently updated values can reduce false sharing when the hardware's
/// cache lines fit within that padding. The selected size is a target assumption, not a
/// measurement of the running processor, so it does not guarantee a performance improvement.
///
/// # Representation
///
/// | Target | Minimum alignment |
/// | --- | --- |
/// | `aarch64`, `powerpc64` | 128 bytes. |
/// | `arm`, `mips`, `mips64` | 32 bytes. |
/// | `s390x` | 256 bytes. |
/// | Other targets, including x86-64 | 64 bytes. |
///
/// A value with a larger natural alignment keeps that requirement. Large values can occupy
/// several cache lines. This type does not limit their size to the selected padding.
#[derive(Clone, Copy, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(target_arch = "x86_64", repr(align(64)))]
#[cfg_attr(any(target_arch = "aarch64", target_arch = "powerpc64",), repr(align(128)))]
#[cfg_attr(
    any(target_arch = "arm", target_arch = "mips", target_arch = "mips64"),
    repr(align(32))
)]
#[cfg_attr(target_arch = "s390x", repr(align(256)))]
#[cfg_attr(
    not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "powerpc64",
        target_arch = "arm",
        target_arch = "mips",
        target_arch = "mips64",
        target_arch = "s390x",
    )),
    repr(align(64))
)]
// NOTE(invariant): The wrapped value starts at the target cache-line alignment selected above.
pub struct CachePadded<T>(T);

impl<T> CachePadded<T> {
    /// Place the value in storage with the target cache-padding alignment.
    #[inline]
    pub const fn new(target_value: T) -> Self {
        Self(target_value)
    }

    /// Unwrap the target `T` from this [`CachePadded`] wrapper.
    #[inline]
    // FIXME(const): Make this a const fn once destructors for generic types can
    // be ran in const fn.
    pub fn unwrap(self) -> T {
        let Self(target_value) = self;

        target_value
    }

    /// Reference the target `T` from this [`CachePadded`] wrapper.
    #[inline]
    pub const fn as_ref(&self) -> &T {
        let &Self(ref target_value) = self;

        target_value
    }

    /// Mutably reference the target `T` from this [`CachePadded`] wrapper.
    #[inline]
    pub const fn as_mut(&mut self) -> &mut T {
        let &mut Self(ref mut target_value) = self;

        target_value
    }
}

impl<T> Deref for CachePadded<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        let &Self(ref target_value) = self;

        target_value
    }
}

impl<T> DerefMut for CachePadded<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        let &mut Self(ref mut target_value) = self;

        target_value
    }
}

impl<T> fmt::Debug for CachePadded<T>
where
    T: fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let &Self(ref target_value) = self;

        fmt::Debug::fmt(target_value, f)
    }
}

impl<T> fmt::Display for CachePadded<T>
where
    T: fmt::Display,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let &Self(ref target_value) = self;

        fmt::Display::fmt(target_value, f)
    }
}
