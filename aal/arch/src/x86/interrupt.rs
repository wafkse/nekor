//! Architectural x86 interrupt-vector values.
//!
//! The processor exception assignments and IDT error-code rules follow
//! [Intel SDM, Volume 3A, Table 6-1][intel-sdm] and
//! [AMD64 APM, Volume 2, Section 8.2 and Table 8-1][amd-apm].
//!
//! [`RawInterruptVector`] preserves every vector number in the eight-bit namespace.
//! [`RawInterruptVector::normalize()`] maps the processor assignments modeled here to
//! [`InterruptVector`] variants, including AMD's assignments for vectors 28 through 30.
//!
//! Use [`exception::ErrorCode`] to test whether hardware exception delivery through
//! the IDT pushes an error-code slot for a vector.
//!
//! ```
//! use nekor_aal_arch::x86::interrupt::{InterruptVector, RawInterruptVector, exception};
//!
//! let vector = RawInterruptVector::new(14);
//! assert_eq!(vector.normalize(), Some(InterruptVector::PageFault));
//! assert!(vector.test::<exception::ErrorCode>());
//!
//! let vector = RawInterruptVector::FIRST_USER_DEFINED;
//! assert_eq!(vector.normalize(), None);
//! assert!(!vector.test::<exception::ErrorCode>());
//! ```
//!
//! [`StartupVector`] follows the start-up IPI vector definition in Intel SDM,
//! Volume 3A, Sections 8.4.4 and 11.6.1.
//!
//! [intel-sdm]: https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html
//! [amd-apm]: https://docs.amd.com/v/u/en-US/24593_3.44_APM_Vol2

/// A raw x86 interrupt-vector number.
///
/// The full eight-bit namespace includes reserved and user-defined vectors.
/// A vector number is distinct from an interrupt-controller pin or line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): None.
pub struct RawInterruptVector(u8);

impl RawInterruptVector {
    /// The first vector available for user-defined interrupts.
    pub const FIRST_USER_DEFINED: Self = Self(32);

    /// Tests whether this interrupt-vector number has the selected property.
    #[inline]
    #[must_use]
    pub fn test<N>(self) -> bool
    where
        N: Property,
    {
        <N as Property>::test(self)
    }
}

/// A raw x86 exception error-code newtype.
///
/// [`exception::ErrorCode`] describes when the error-code slot is present.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): None.
pub struct ExceptionErrorCode(u32);

/// An x86 start-up IPI vector.
///
/// The vector selects a 4-KiB page in the first MiB. Its physical start address
/// is the vector shifted left by 12 bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): None.
pub struct StartupVector(u8);

/// An architectural property of an x86 interrupt-vector number.
///
/// The architecture module defines the supported properties. Query them through
/// [`RawInterruptVector::test()`] or [`InterruptVector::test()`].
pub trait Property: detail::Sealed {
    /// Tests whether the vector number has this property.
    fn test(vector: RawInterruptVector) -> bool;
}

/// The properties of architectural exception vectors.
pub mod exception {
    /// The vector property for hardware exceptions that push an error-code slot
    /// when delivered through the IDT.
    ///
    /// Double Fault and Alignment Check push zero. Other exceptions that push an
    /// error code carry an exception-specific image described by the architecture manuals.
    /// Software `INT n` delivery follows its own stack rules even when it uses the same
    /// vector number.
    pub struct ErrorCode;
}

impl detail::Sealed for exception::ErrorCode {}

impl InterruptVector {
    /// Tests whether this vector has the selected property.
    #[inline]
    #[must_use]
    pub fn test<N>(self) -> bool
    where
        N: Property,
    {
        <N as Property>::test(Self::raw(self))
    }
}

detail::vector!(
    /// A named x86 processor interrupt vector.
    ///
    /// Use [`RawInterruptVector`] to preserve unassigned and user-defined vector numbers.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub enum InterruptVector {
        /// Divide Error (`#DE`).
        DivideError = 0,

        /// Debug (`#DB`).
        Debug = 1,

        /// Non-maskable interrupt (NMI).
        NonMaskableInterrupt = 2,

        /// Breakpoint (`#BP`).
        Breakpoint = 3,

        /// Overflow (`#OF`).
        Overflow = 4,

        /// Bound Range Exceeded (`#BR`).
        BoundRangeExceeded = 5,

        /// Invalid Opcode (`#UD`).
        InvalidOpcode = 6,

        /// Device Not Available (`#NM`).
        DeviceNotAvailable = 7,

        /// Double Fault (`#DF`).
        DoubleFault = 8,

        /// Invalid TSS (`#TS`).
        InvalidTss = 10,

        /// Segment Not Present (`#NP`).
        SegmentNotPresent = 11,

        /// Stack-Segment Fault (`#SS`).
        StackSegmentFault = 12,

        /// General Protection (`#GP`).
        GeneralProtection = 13,

        /// Page Fault (`#PF`).
        PageFault = 14,

        /// x87 floating-point error (`#MF`).
        X87FloatingPoint = 16,

        /// Alignment Check (`#AC`).
        AlignmentCheck = 17,

        /// Machine Check (`#MC`).
        MachineCheck = 18,

        /// SIMD floating-point exception (`#XM` or `#XF`).
        SimdFloatingPoint = 19,

        /// Virtualization Exception (`#VE`).
        Virtualization = 20,

        /// Control Protection Exception (`#CP`).
        ControlProtection = 21,

        /// AMD-specific Hypervisor Injection Exception (`#HV`).
        HypervisorInjection = 28,

        /// AMD-specific VMM Communication Exception (`#VC`).
        VmmCommunication = 29,

        /// AMD-specific Security Exception (`#SX`).
        Security = 30,
    } as u8;

    impl Property for exception::ErrorCode in InterruptVector {
        DoubleFault | InvalidTss | SegmentNotPresent | StackSegmentFault | GeneralProtection
            | PageFault | AlignmentCheck | ControlProtection | VmmCommunication | Security
    }

    impl RawInterruptVector for u8;
    impl ExceptionErrorCode for u32;
    impl StartupVector for u8;
);

/// The expansions and property seal for interrupt vectors.
mod detail {
    /// Restricts interrupt-vector properties to this module.
    pub trait Sealed {}

    /// Expand each declaration before processing the remaining input.
    ///
    /// An `impl Property` declaration lists the enum variants with that property.
    /// A variant may have any number of properties.
    macro_rules! vector {
        () => {};
        (
            $(
                #[$target_meta:meta]
            )*

            $target_vis:vis enum $target_name:ident { $($target_variants:tt)* } as $target_repr:ty;

            $($target_rest:tt)*
        ) => {
            tokel::stream!(
                detail::vector!(
                    $(#[$target_meta])*
                    $target_vis enum $target_name for [< Raw $target_name >]:concatenate {
                        $($target_variants)*
                    } as $target_repr;
                );
            );

            detail::vector!($($target_rest)*);
        };
        (
            $(
                #[$target_meta:meta]
            )*

            $target_vis:vis enum $target_name:ident for $target_raw:ident {
                $(
                    $(#[$variant_meta:meta])*
                    $variant_name:ident = $variant_number:literal
                ),+ $(,)?
            } as $target_repr:ty;
        ) => {
            $(#[$target_meta])*
            #[repr($target_repr)]
            $target_vis enum $target_name {
                $(
                    $(#[$variant_meta])*
                    $variant_name = $variant_number,
                )+
            }

            impl $target_raw {
                /// Returns the named processor vector when this module models its assignment.
                ///
                /// Returns `None` for vector numbers with no corresponding enum variant,
                /// including unassigned and user-defined vectors.
                #[inline]
                #[must_use]
                $target_vis const fn normalize(self) -> Option<$target_name> {
                    match <$target_raw>::get(self) {
                        $($variant_number => Some($target_name::$variant_name),)+
                        _ => None,
                    }
                }
            }

            impl $target_name {
                /// Returns the raw interrupt-vector number.
                #[inline]
                #[must_use]
                $target_vis const fn raw(self) -> $target_raw {
                    <$target_raw>::new(self as $target_repr)
                }
            }

            impl From<$target_name> for $target_raw {
                #[inline]
                fn from(vector: $target_name) -> Self {
                    <$target_name>::raw(vector)
                }
            }

            impl TryFrom<$target_raw> for $target_name {
                type Error = $target_raw;

                #[inline]
                fn try_from(vector: $target_raw) -> Result<Self, Self::Error> {
                    Option::<Self>::ok_or::<$target_raw>(<$target_raw>::normalize(vector), vector)
                }
            }

        };
        (
            impl $target_name:ident for $target_repr:ty;

            $($target_rest:tt)*
        ) => {
            impl $target_name {
                /// Constructs an architectural value from its raw representation.
                #[inline]
                #[must_use]
                pub const fn new(value: $target_repr) -> Self {
                    Self(value)
                }

                /// Returns the underlying architectural value.
                #[inline]
                #[must_use]
                pub const fn get(self) -> $target_repr {
                    let Self(value) = self;

                    value
                }
            }

            detail::vector!($($target_rest)*);
        };
        (
            impl Property for $($target_property:ident)::+ in $target_enum:ident {
                $($target_variant:ident)|+
            }

            $($target_rest:tt)*
        ) => {
            tokel::stream!(
                impl Property for $($target_property)::+ {
                    #[inline]
                    fn test(vector: [< Raw $target_enum >]:concatenate) -> bool {
                        $(
                            const [< $target_variant >]:case[[snake]]:case[[upper]]:push_right[[:]]
                                [< Raw $target_enum >]:concatenate =
                                <$target_enum>::raw($target_enum::$target_variant);
                        )+

                        match vector {
                            $([< $target_variant >]:case[[snake]]:case[[upper]])|+ => true,
                            _ => false,
                        }
                    }
                }
            );

            detail::vector!($($target_rest)*);
        };
    }

    pub(in crate::x86::interrupt) use vector;
}

#[cfg(test)]
mod tests {
    use super::{InterruptVector, Property, RawInterruptVector, detail, exception};

    /// A test namespace with members on either side of the error-code classification.
    struct OverlappingNamespace;

    impl detail::Sealed for OverlappingNamespace {}

    detail::vector!(
        impl Property for OverlappingNamespace in InterruptVector {
            Debug | PageFault
        }
    );

    #[test]
    fn assigned_vector_names_match_the_manual_tables() {
        assert_eq!(InterruptVector::DoubleFault.raw().get(), 8);
        assert_eq!(InterruptVector::GeneralProtection.raw().get(), 13);
        assert_eq!(InterruptVector::PageFault.raw().get(), 14);
        assert_eq!(InterruptVector::ControlProtection.raw().get(), 21);
        assert_eq!(InterruptVector::VmmCommunication.raw().get(), 29);
    }

    #[test]
    fn normalization_preserves_assigned_values_and_rejects_unassigned_values() {
        const GENERAL_PROTECTION: Option<InterruptVector> = RawInterruptVector::new(13).normalize();

        assert_eq!(GENERAL_PROTECTION, Some(InterruptVector::GeneralProtection));
        assert_eq!(RawInterruptVector::FIRST_USER_DEFINED.get(), 32);

        let assigned = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 13, 14, 16, 17, 18, 19, 20, 21, 28, 29, 30,
        ];

        for number in u8::MIN..=u8::MAX {
            let raw = RawInterruptVector::new(number);
            let named = raw.normalize();

            assert_eq!(named.is_some(), assigned.contains(&number), "vector {number}");
            assert_eq!(named.map(RawInterruptVector::from).unwrap_or(raw), raw);
            assert_eq!(InterruptVector::try_from(raw), named.ok_or(raw));
        }
    }

    #[test]
    fn error_code_shape_matches_the_architectural_vectors() {
        for number in u8::MIN..=u8::MAX {
            let raw = RawInterruptVector::new(number);
            let expected = matches!(number, 8 | 10 | 11 | 12 | 13 | 14 | 17 | 21 | 29 | 30);

            assert_eq!(raw.test::<exception::ErrorCode>(), expected, "vector {number}");
            assert_eq!(
                raw.test::<OverlappingNamespace>(),
                matches!(number, 1 | 14),
                "vector {number}"
            );

            if let Some(vector) = raw.normalize() {
                assert_eq!(vector.test::<exception::ErrorCode>(), expected, "vector {number}");
            }
        }
    }
}
