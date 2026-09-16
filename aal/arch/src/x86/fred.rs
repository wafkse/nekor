//! x86 Flexible Return and Event Delivery capability facts.

/// Proof that the current processor exposes FRED transitions and their
/// architectural MSRs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fred(());

impl Fred {
    /// Create the FRED capability proof for the current processor.
    ///
    /// # Safety
    ///
    /// The current processor must report the FRED architectural feature.
    #[inline]
    #[must_use]
    pub const unsafe fn assume() -> Self {
        Self(())
    }
}

enumerate![
    /// FRED supervisor stack level.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub enum Level {
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
    use super::Level;

    #[test]
    fn stack_levels_cover_the_complete_hardware_domain() {
        assert_eq!(Level::lift(0), Some(Level::Zero));
        assert_eq!(Level::lift(3), Some(Level::Three));
        assert_eq!(Level::lift(4), None);
    }
}
