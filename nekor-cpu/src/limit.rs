//! Compile-time capacity for processor-local storage.
//!
//! Enable a `maximum-core-count-N` feature to choose the number of slots. When several
//! presets are enabled, the largest takes precedence. The default is 16 slots, and selecting
//! no preset fails constant evaluation.

use core::num::NonZero;

/// Select the first enabled core-count preset from the descending list.
macro_rules! size {
    () => {};
    ($(($target_size:literal, $feature:literal)),+ $(,)?) => {
        const {
            $(
                if cfg!(feature = $feature) {
                    $target_size
                } else
            )+
            {
                panic!(concat!(
                    "No selected maximal processor core count, please enable one of the following:",
                    $(" maximum-core-count-", $feature),+
                ))
            }
        }
    };
}

/// An uninhabited type that serves as an umbrella type to denote the maximum
/// processor cores that the kernel can manage simultaneously.
pub enum Cores {}

impl Cores {
    /// Determine the maximum number of distinct processor cores that the kernel
    /// can drive simultaneously.
    #[inline]
    #[must_use]
    pub const fn maximum() -> NonZero<usize> {
        const {
            NonZero::new(size!(
                (8192, "maximum-core-count-8192"),
                (4096, "maximum-core-count-4096"),
                (2048, "maximum-core-count-2048"),
                (1024, "maximum-core-count-1024"),
                (512, "maximum-core-count-512"),
                (256, "maximum-core-count-256"),
                (128, "maximum-core-count-128"),
                (64, "maximum-core-count-64"),
                (32, "maximum-core-count-32"),
                (16, "maximum-core-count-16"),
                (8, "maximum-core-count-8"),
                (4, "maximum-core-count-4"),
                (2, "maximum-core-count-2"),
                (1, "maximum-core-count-1"),
            ))
            .expect("bad core count")
        }
    }
}
