//! Architectural x86 interrupt-vector values.
//!
//! The processor exception assignments and error-code stack shapes follow
//! [Intel SDM, Volume 3A, Table 6-1][intel-sdm] and
//! [AMD64 APM, Volume 2, Section 8.2 and Table 8-1][amd-apm]. AMD assigns vectors 28 through
//! 30 to exceptions that Intel reserves. [`ExceptionVector`] retains the full
//! processor-reserved range so software can represent either architecture and
//! future assignments without losing the raw vector.
//!
//! [`StartupVector`] follows the start-up IPI vector definition in Intel SDM,
//! Volume 3A, Sections 8.4.4 and 11.6.1.
//!
//! [intel-sdm]: https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html
//! [amd-apm]: https://docs.amd.com/v/u/en-US/24593_3.44_APM_Vol2

/// An x86 interrupt-vector number.
///
/// The architecture defines an eight-bit vector namespace. This type preserves
/// the distinction between a vector and interrupt-controller pins or lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar covers the architectural eight-bit vector domain.
pub struct InterruptVector(u8);

impl InterruptVector {
    /// First vector available for user-defined interrupts.
    pub const FIRST_USER_DEFINED: Self = Self(32);

    /// Constructs a vector from its architectural number.
    #[inline]
    #[must_use]
    pub const fn new(vector: u8) -> Self {
        Self(vector)
    }

    /// Returns the architectural vector number.
    #[inline]
    #[must_use]
    pub const fn get(self) -> u8 {
        let Self(vector) = self;

        vector
    }
}

/// A vector in the processor-reserved exception range.
///
/// Values in this range do not all identify exceptions. Vector 2 is the NMI
/// vector, while vectors 9, 15, 22 through 27, and 31 are currently reserved.
/// Vectors 28 through 30 have AMD-defined meanings and remain reserved by
/// Intel. The named constants identify every currently assigned meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar is always in the processor-reserved range 0..=31.
pub struct ExceptionVector(u8);

impl ExceptionVector {
    /// Alignment Check (`#AC`).
    pub const ALIGNMENT_CHECK: Self = Self(17);
    /// Bound Range Exceeded (`#BR`).
    pub const BOUND_RANGE_EXCEEDED: Self = Self(5);
    /// Breakpoint (`#BP`).
    pub const BREAKPOINT: Self = Self(3);
    /// Control Protection Exception (`#CP`).
    pub const CONTROL_PROTECTION: Self = Self(21);
    /// Debug (`#DB`).
    pub const DEBUG: Self = Self(1);
    /// Device Not Available (`#NM`).
    pub const DEVICE_NOT_AVAILABLE: Self = Self(7);
    /// Divide Error (`#DE`).
    pub const DIVIDE_ERROR: Self = Self(0);
    /// Double Fault (`#DF`).
    pub const DOUBLE_FAULT: Self = Self(8);
    /// General Protection (`#GP`).
    pub const GENERAL_PROTECTION: Self = Self(13);
    /// AMD Hypervisor Injection Exception (`#HV`).
    pub const HYPERVISOR_INJECTION: Self = Self(28);
    /// Invalid Opcode (`#UD`).
    pub const INVALID_OPCODE: Self = Self(6);
    /// Invalid TSS (`#TS`).
    pub const INVALID_TSS: Self = Self(10);
    /// Machine Check (`#MC`).
    pub const MACHINE_CHECK: Self = Self(18);
    /// Highest processor-reserved vector.
    pub const MAX: Self = Self(31);
    /// Lowest processor-reserved vector.
    pub const MIN: Self = Self(0);
    /// Non-maskable interrupt (NMI).
    pub const NON_MASKABLE_INTERRUPT: Self = Self(2);
    /// Overflow (`#OF`).
    pub const OVERFLOW: Self = Self(4);
    /// Page Fault (`#PF`).
    pub const PAGE_FAULT: Self = Self(14);
    /// AMD Security Exception (`#SX`).
    pub const SECURITY: Self = Self(30);
    /// Segment Not Present (`#NP`).
    pub const SEGMENT_NOT_PRESENT: Self = Self(11);
    /// SIMD floating-point exception (`#XM` or `#XF`).
    pub const SIMD_FLOATING_POINT: Self = Self(19);
    /// Stack-Segment Fault (`#SS`).
    pub const STACK_SEGMENT_FAULT: Self = Self(12);
    /// Virtualization Exception (`#VE`).
    pub const VIRTUALIZATION: Self = Self(20);
    /// AMD VMM Communication Exception (`#VC`).
    pub const VMM_COMMUNICATION: Self = Self(29);
    /// Assigned exception vectors whose hardware delivery pushes an error code.
    ///
    /// Double Fault and Alignment Check push zero. Other entries carry an
    /// exception-specific image described by the architecture manuals.
    pub const WITH_ERROR_CODE: &'static [Self] = &[
        Self::DOUBLE_FAULT,
        Self::INVALID_TSS,
        Self::SEGMENT_NOT_PRESENT,
        Self::STACK_SEGMENT_FAULT,
        Self::GENERAL_PROTECTION,
        Self::PAGE_FAULT,
        Self::ALIGNMENT_CHECK,
        Self::CONTROL_PROTECTION,
        Self::VMM_COMMUNICATION,
        Self::SECURITY,
    ];
    /// x87 floating-point error (`#MF`).
    pub const X87_FLOATING_POINT: Self = Self(16);

    /// Constructs a vector in the processor-reserved range.
    ///
    /// Returns `None` for external and software-defined vectors.
    #[inline]
    #[must_use]
    pub const fn new(vector: u8) -> Option<Self> {
        if vector <= Self::MAX.get() {
            Some(Self(vector))
        } else {
            None
        }
    }

    /// Returns the architectural vector number.
    #[inline]
    #[must_use]
    pub const fn get(self) -> u8 {
        let Self(vector) = self;

        vector
    }

    /// Returns whether hardware delivery pushes an architectural error-code slot.
    ///
    /// Software `INT n` delivery is a distinct event source and does not gain
    /// this stack shape merely by targeting the same vector number.
    #[inline]
    #[must_use]
    pub const fn pushes_error_code(self) -> bool {
        let mut index = 0;

        while index < Self::WITH_ERROR_CODE.len() {
            if self.get() == Self::WITH_ERROR_CODE[index].get() {
                return true;
            }

            index += 1;
        }

        false
    }

    /// Widens this processor-reserved vector into the full vector namespace.
    #[inline]
    #[must_use]
    pub const fn interrupt(self) -> InterruptVector {
        InterruptVector::new(self.get())
    }
}

impl From<ExceptionVector> for InterruptVector {
    #[inline]
    fn from(vector: ExceptionVector) -> Self {
        vector.interrupt()
    }
}

impl TryFrom<InterruptVector> for ExceptionVector {
    type Error = InterruptVector;

    #[inline]
    fn try_from(vector: InterruptVector) -> Result<Self, Self::Error> {
        match Self::new(vector.get()) {
            Some(vector) => Ok(vector),
            None => Err(vector),
        }
    }
}

/// A raw x86 exception error-code image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar preserves the architectural 32-bit exception error-code
// image.
pub struct ExceptionErrorCode(u32);

impl ExceptionErrorCode {
    /// Constructs an architectural exception error-code image.
    #[inline]
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the architectural error-code image.
    #[inline]
    #[must_use]
    pub const fn get(self) -> u32 {
        let Self(value) = self;

        value
    }
}

/// An x86 start-up IPI vector.
///
/// The vector selects a 4-KiB page in the first MiB. Its physical start address
/// is the vector shifted left by 12 bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar preserves the architectural eight-bit SIPI vector domain.
pub struct StartupVector(u8);

impl StartupVector {
    /// Constructs a start-up IPI vector.
    #[inline]
    #[must_use]
    pub const fn new(vector: u8) -> Self {
        Self(vector)
    }

    /// Returns the IPI vector field.
    #[inline]
    #[must_use]
    pub const fn get(self) -> u8 {
        let Self(vector) = self;

        vector
    }
}

#[cfg(test)]
mod tests {
    use super::{ExceptionVector, InterruptVector, StartupVector};

    #[test]
    fn exception_domain_ends_before_user_defined_vectors() {
        assert_eq!(
            ExceptionVector::new(ExceptionVector::MAX.get()),
            Some(ExceptionVector::MAX)
        );
        assert_eq!(ExceptionVector::new(InterruptVector::FIRST_USER_DEFINED.get()), None);
    }

    #[test]
    fn assigned_vector_names_match_the_manual_tables() {
        assert_eq!(ExceptionVector::DOUBLE_FAULT.get(), 8);
        assert_eq!(ExceptionVector::PAGE_FAULT.get(), 14);
        assert_eq!(ExceptionVector::CONTROL_PROTECTION.get(), 21);
        assert_eq!(ExceptionVector::VMM_COMMUNICATION.get(), 29);
    }

    #[test]
    fn error_code_shape_distinguishes_named_exceptions() {
        assert!(ExceptionVector::DOUBLE_FAULT.pushes_error_code());
        assert!(ExceptionVector::PAGE_FAULT.pushes_error_code());
        assert!(ExceptionVector::VMM_COMMUNICATION.pushes_error_code());
        assert!(!ExceptionVector::DEBUG.pushes_error_code());
        assert!(!ExceptionVector::MACHINE_CHECK.pushes_error_code());
        assert!(!ExceptionVector::HYPERVISOR_INJECTION.pushes_error_code());
    }
}
