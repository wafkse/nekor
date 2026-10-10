//! Architecture-specific code for the `x86` architecture.
//!
//! # Compatibility
//!
//! Nekor does not support either *Real Mode* or *Protected Mode*, but modeling
//! and incomplete support for protected mode `x86` is done due to its
//! similarities with *long mode*.

/// Define an integer-backed enum with generated representation conversions.
macro_rules! enumerate {
    (
        $(#[$target_meta:meta])*
        $target_vis:vis enum $target_name:ident {
            $(
                $(#[$variant_meta:meta])*
                $variant_ident:ident $(= $variant_discriminant:expr)?
            ),+ $(,)?
        } as $target_repr:ty
    ) => {
        $(#[$target_meta])*
        #[repr($target_repr)]
        $target_vis enum $target_name {
            $(
                $(#[$variant_meta])*
                $variant_ident $(= $variant_discriminant)?,
            )+
        }

        impl $target_name {
            /// Attempt to lift a bare-reprvalue into an enumeration variant.
            #[inline]
            #[must_use]
            $target_vis const fn lift(target_value: $target_repr) -> Option<Self> {
                match target_value {
                    $(target_value if target_value == Self::$variant_ident as $target_repr => Some(Self::$variant_ident),)+
                    _ => None,
                }
            }

            /// Determine the bare-integer representation of this identity.
            #[inline]
            #[must_use]
            $target_vis const fn raw(self) -> $target_repr {
                self as $target_repr
            }
        }
    };
}

pub mod mode;

pub mod msr;

pub mod cpuid;

pub mod fred;

pub mod interrupt;

pub mod xstate;

pub mod instruction;

pub mod address;

pub mod segmentation;

pub mod privilege;

pub mod descriptor;

pub mod gate;

pub mod serialize;
