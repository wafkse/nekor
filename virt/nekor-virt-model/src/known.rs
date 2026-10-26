//! Type-level architecture markers for well-known architectures.

use core::convert;

/// A macro to define a type for a well-known architecture.
///
/// This automatically implements the marker [`Arch`] trait.
///
/// [`Arch`]: super::Arch
macro_rules! known {
    () => {};
    ($(#[$target_meta:meta])* $target_vis:vis for $target_ident:ident) => {
        $(#[$target_meta])*
        $target_vis struct $target_ident(convert::Infallible);

        impl $crate::baseline::Arch for $target_ident { }
    };
}

known!(
    /// The marker type for the `x86_64` architecture.
    #[derive(Debug, Copy, Clone)]
    pub for X86_64
);

known!(
    /// The marker type for the `aarch64` architecture.
    #[derive(Debug, Copy, Clone)]
    pub for Aarch64
);
