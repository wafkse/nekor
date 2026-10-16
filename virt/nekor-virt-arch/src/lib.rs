#![no_std]
#![doc = include_str!("../README.md")]

extern crate alloc;

pub use nekor_virt_arch_x86 as x86;

/// A marker that identifies a particular Instruction Set Architecture.
pub trait Arch: 'static {}

pub mod known {
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
}
