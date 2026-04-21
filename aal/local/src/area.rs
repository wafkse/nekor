//! Mandatory per-CPU [`Area`].

use crate::arch;

/// The mandatory per-CPU area.
///
/// Each online processor core must be able to access this structure at all
/// times, and at reasonable efficiency.
#[repr(transparent)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Area(usize);

impl Area {
    /// Instantiate a new local per-CPU area.
    #[inline]
    #[must_use]
    pub const fn local() -> Self {
        Self(usize::MIN)
    }

    /// Determine the implementation-defined pointer-sized opaque [`usize`]
    /// value contained within this [`Area`].
    #[inline]
    #[must_use]
    pub const fn value(self) -> usize {
        let Self(raw_value) = self;

        raw_value
    }

    /// Construct a new [`Area`] from a raw pointer-sized value.
    #[inline]
    #[must_use]
    pub const fn raw(raw_value: usize) -> Self {
        Self(raw_value)
    }
}

impl Area {
    /// Read the per-CPU designated area.
    ///
    /// This will read the [`Area`] from architecture- and platform-dependent
    /// storage.
    #[inline]
    #[must_use]
    pub fn read() -> Self {
        #[cfg(any(target_arch = "x86"))]
        return arch::x86::read_area();

        #[cfg(any(target_arch = "x86_64"))]
        return arch::x86_64::read_area();
    }

    /// Write to the per-CPU designated area.
    ///
    /// This will write the [`Area`] to architecture- and platform-dependent
    /// storage.
    #[inline]
    pub fn write(self) {
        #[cfg(any(target_arch = "x86"))]
        return arch::x86::write_area(self);

        #[cfg(any(target_arch = "x86_64"))]
        return arch::x86_64::write_area(self);
    }
}
