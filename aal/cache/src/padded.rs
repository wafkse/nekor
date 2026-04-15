use core::{
    fmt,
    ops::{Deref, DerefMut},
};

/// A new-type wrapper that forces alignment to a single cache line.
///
/// This is required for concurrent lock-free code (and all other code which
/// does not wish to deal with cache coherence-induced slowdown), as [false
/// sharing] can effectively erase any gained performance benefit.
///
/// ## Cache Line Sizes by Architecture
///
/// Cache line size is majorly architecture-dependant, therefore, it must be
/// defined on a per-architecture basis.
///
/// | Architecture | Cache Line Size | Notes |
/// |--------------|-----------------|-------|
/// | `x86_64` | 64 bytes | x86-64 architecture |
/// | aarch64 | 128 bytes | ARM `big.LITTLE` "big" cores have 128-byte cache lines |
/// | powerpc64 | 128 bytes | Standard cache line size |
/// | arm | 32 bytes | `ARMv7` and earlier |
/// | mips | 32 bytes | MIPS32 architecture |
/// | mips64 | 32 bytes | MIPS64 architecture |
/// | s390x | 256 bytes | IBM System z architecture |
/// | x86 | 64 bytes | Standard x86 cache line size |
/// | riscv | 64 bytes | RISC-V architecture |
/// | wasm | 64 bytes | WebAssembly matches the `x86` architecture |
/// | Others | 64 bytes | Fallback assumed size |
///
/// ## References
///
/// - [Intel 64 and IA-32 Architectures Optimization Reference Manual](https://www.intel.com/content/dam/www/public/us/en/documents/manuals/64-ia-32-architectures-optimization-manual.pdf)
/// - [Facebook Folly alignment implementation](https://github.com/facebook/folly/blob/1b5288e6eea6df074758f877c849b6e73bbb9fbb/folly/lang/Align.h#L107)
/// - [ARM64 instruction cache behavior](https://www.mono-project.com/news/2016/09/12/arm64-icache/)
/// - [Go runtime CPU constants](https://github.com/golang/go/tree/master/src/internal/cpu)
/// - [Linux RISC-V cache definitions](https://github.com/torvalds/linux/blob/3516bd729358a2a9b090c1905bd2a3fa926e24c6/arch/riscv/include/asm/cache.h#L10)
///
/// [false sharing]: https://en.wikipedia.org/wiki/False_sharing
#[derive(Clone, Copy, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(target_arch = "x86_64", repr(align(64)))]
#[cfg_attr(
    any(target_arch = "aarch64", target_arch = "powerpc64",),
    repr(align(128))
)]
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
pub struct CachePadded<T>(T);

impl<T> CachePadded<T> {
    /// Wrap a `T` to be effectively aligned to a single cache line size.
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
        let Self(target_value) = self;

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
        let Self(target_value) = self;

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
        let Self(target_value) = self;

        fmt::Debug::fmt(target_value, f)
    }
}

impl<T> fmt::Display for CachePadded<T>
where
    T: fmt::Display,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self(target_value) = self;

        fmt::Display::fmt(target_value, f)
    }
}
