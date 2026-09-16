//! Architectural x86 interrupt-vector values.

/// One x86 interrupt-vector number.
///
/// The architecture defines a complete eight-bit vector namespace. This type
/// preserves the semantic distinction between a vector and interrupt-controller
/// pins or lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar covers the complete architectural eight-bit vector domain.
pub struct InterruptVector(u8);

impl InterruptVector {
    /// Constructs one vector from its complete architectural number.
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

/// One x86 architectural exception-vector number.
///
/// The processor reserves vectors zero through 31 for architectural
/// exceptions. This type rejects values outside that domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar is always in the architectural exception-vector range 0..=31.
pub struct ExceptionVector(u8);

impl ExceptionVector {
    /// Create an architectural exception vector.
    ///
    /// Returns `None` when `vector` is outside the processor-reserved range
    /// zero through 31.
    #[inline]
    #[must_use]
    pub const fn new(vector: u8) -> Option<Self> {
        match vector {
            0..=31 => Some(Self(vector)),
            _ => None,
        }
    }

    /// Returns the architectural exception-vector number.
    #[inline]
    #[must_use]
    pub const fn get(self) -> u8 {
        let Self(vector) = self;

        vector
    }

    /// Return whether hardware exception delivery pushes an architectural error-code slot.
    ///
    /// This classification covers the architectural exception meanings for the selected vector.
    /// Software `INT n` delivery is a distinct event source and must not borrow this stack-shape
    /// proof merely because it targets the same vector number.
    #[inline]
    #[must_use]
    pub const fn pushes_error_code(self) -> bool {
        let Self(vector) = self;

        matches!(vector, 8 | 10 | 11 | 12 | 13 | 14 | 17 | 21 | 29 | 30)
    }
}

/// One raw x86 exception error-code image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar preserves the complete architectural 32-bit exception
// error-code image.
pub struct ExceptionErrorCode(u32);

impl ExceptionErrorCode {
    /// Constructs one complete architectural exception error-code image.
    #[inline]
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the complete architectural error-code image.
    #[inline]
    #[must_use]
    pub const fn get(self) -> u32 {
        let Self(value) = self;

        value
    }
}

/// One x86 startup-interrupt vector.
///
/// The vector selects one 4 KiB page in the first MiB for application-processor
/// startup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar preserves the complete architectural eight-bit startup-vector
// domain.
pub struct StartupVector(u8);

impl StartupVector {
    /// Constructs one startup vector.
    #[inline]
    #[must_use]
    pub const fn new(vector: u8) -> Self {
        Self(vector)
    }

    /// Returns the architectural startup-vector number.
    #[inline]
    #[must_use]
    pub const fn get(self) -> u8 {
        let Self(vector) = self;

        vector
    }
}

#[cfg(test)]
mod tests {
    use super::{ExceptionVector, StartupVector};

    #[test]
    fn exception_vector_rejects_non_exception_domain() {
        assert_eq!(ExceptionVector::new(31).map(ExceptionVector::get), Some(31));
        assert_eq!(ExceptionVector::new(32), None);
    }

    #[test]
    fn exception_error_code_shape_includes_current_architectural_vectors() {
        for vector in [8_u8, 10, 11, 12, 13, 14, 17, 21, 29, 30] {
            let vector = ExceptionVector::new(vector).expect("listed exception vector must be valid");

            assert!(vector.pushes_error_code());
        }

        for vector in [0_u8, 1, 2, 3, 4, 5, 6, 7, 9, 16, 18, 20, 28, 31] {
            let vector = ExceptionVector::new(vector).expect("listed exception vector must be valid");

            assert!(!vector.pushes_error_code());
        }
    }

    #[test]
    fn startup_vector_preserves_complete_byte_domain() {
        assert_eq!(StartupVector::new(u8::MAX).get(), u8::MAX);
    }
}
