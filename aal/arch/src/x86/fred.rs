//! x86 Flexible Return and Event Delivery capability facts.

/// Proof that the current processor exposes `FRED` transitions and their
/// architectural MSRs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
// NOTE(invariant): Construction requires the caller to establish `FRED` support on the current CPU.
pub struct FredCapability(());

impl FredCapability {
    /// Create the `FRED` capability proof for the current processor.
    ///
    /// # Safety
    ///
    /// The current processor must report the `FRED` architectural feature.
    #[inline]
    #[must_use]
    pub const unsafe fn assume() -> Self {
        Self(())
    }
}

enumerate![
    /// A `FRED` supervisor stack level.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub enum FredLevel {
        /// Ordinary supervisor event stack level.
        Zero = 0,

        /// First machine-safety stack level.
        One = 1,

        /// Second machine-safety stack level.
        Two = 2,

        /// Third machine-safety stack level.
        Three = 3,
    } as u8
];

#[cfg(test)]
mod tests {
    use super::FredLevel;

    #[test]
    fn stack_levels_cover_the_complete_hardware_domain() {
        assert_eq!(FredLevel::lift(0), Some(FredLevel::Zero));
        assert_eq!(FredLevel::lift(3), Some(FredLevel::Three));
        assert_eq!(FredLevel::lift(4), None);
    }
}
