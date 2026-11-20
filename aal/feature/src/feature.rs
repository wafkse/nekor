//! Architecture-agnostic Processor Feature inquiries.

use nekor_bitwise::prelude::State;

/// A *feature support signal*.
///
/// The source distinguishes a compile-time assumption from a runtime result.
/// Neither source establishes the execution context needed to use an instruction.
#[derive(Debug, Clone, Hash, Copy, PartialEq, PartialOrd, Ord, Eq)]
pub enum Signal {
    /// The *Processor Feature* was hardcoded at compile-time.
    Guaranteed(Present),

    /// The *Processor Feature* was detected at runtime.
    Detected(Present),
}

impl Signal {
    /// Force a signal state at compile-time.
    #[inline]
    #[must_use]
    pub const fn forced<const S: bool>() -> Self {
        let target_state = if S { Present::Yes } else { Present::No };

        Self::Guaranteed(target_state)
    }

    /// Report absence when no baseline or detection result is available.
    #[inline]
    #[must_use]
    pub const fn unknown() -> Self {
        Self::Detected(Present::No)
    }

    /// Determine a [`Signal`] from a bitwise [`State`].
    #[inline]
    #[must_use]
    pub const fn state(target_state: State) -> Self {
        let target_state = match target_state {
            State::Set => Present::Yes,
            State::Cleared => Present::No,
        };

        Self::Detected(target_state)
    }

    /// Combine support requirements. The result is guaranteed only when both
    /// inputs are guaranteed. Otherwise its source remains runtime detection.
    #[inline]
    #[must_use]
    pub const fn and(&self, target_signal: Self) -> Self {
        match (*self, target_signal) {
            (Self::Guaranteed(present_left), Self::Guaranteed(present_right)) => {
                Self::Guaranteed(Present::and(&present_left, present_right))
            },
            (Self::Guaranteed(present_left) | Self::Detected(present_left), Self::Detected(present_right))
            | (Self::Detected(present_left), Self::Guaranteed(present_right)) => {
                Self::Detected(Present::and(&present_left, present_right))
            },
        }
    }
}

impl Signal {
    /// Determine whether this [`Signal`] indicates a target [`Present`] state,
    /// regardless of signal source.
    #[inline]
    #[must_use]
    pub const fn is(&self, present_left: Present) -> bool {
        let (&Self::Guaranteed(present_right) | &Self::Detected(present_right)) = self;

        match (present_left, present_right) {
            (Present::Yes, Present::Yes) | (Present::No, Present::No) => true,
            (..) => false,
        }
    }
}

/// An enumeration that determines whether a *Processor Feature* is currently
/// present or not.
#[derive(Debug, Clone, Hash, Copy, PartialEq, PartialOrd, Ord, Eq)]
pub enum Present {
    /// The aforementioned *Processor Feature* is not present.
    No,

    /// The aforementioned *Processor Feature* is indeed present.
    Yes,
}

impl Present {
    /// Require that both [`Present`] enumerations be [`Present::Yes`] in
    /// unison.
    #[inline]
    #[must_use]
    pub const fn and(&self, target_right: Self) -> Self {
        match (*self, target_right) {
            (Self::Yes, Self::Yes) => Self::Yes,
            (..) => Self::No,
        }
    }
}

/// Reports instruction support from a baseline or a runtime query.
///
/// A baseline substitutes a compile-time answer for detection. The platform must ensure
/// that every processor using the compiled code satisfies that assumption. A positive
/// answer does not establish privilege, operating-system enablement or CPU affinity.
pub trait Feature {
    /// The predetermined [`Signal`] for this [`Feature`].
    const PREDETERMINED: Option<Signal> = None;

    /// Determine whether the feature is supported by the current processor
    /// core, or a *Feature Baseline* is present.
    #[inline]
    #[must_use]
    fn supported() -> Signal {
        Self::PREDETERMINED.unwrap_or(Signal::unknown())
    }
}
