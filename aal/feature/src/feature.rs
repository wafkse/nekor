//! Architecture-agnostic Processor Feature inquiries.

use nekor_bitwise::prelude::State;

/// A *feature support signal*.
///
/// This contains all the required information for runtime feature detection,
/// with baseline support.
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
    pub const fn forced<const S: bool>() -> Self {
        let target_state = match S {
            true => Present::Yes,
            false => Present::No,
        };

        Self::Guaranteed(target_state)
    }

    /// The availability of the feature is unknown.
    #[inline]
    pub const fn unknown() -> Self {
        Self::Guaranteed(Present::No)
    }

    /// Determine a [`Signal`] from a bitwise [`State`].
    #[inline]
    pub const fn state(target_state: State) -> Self {
        let target_state = if let State::Set = target_state {
            Present::Yes
        } else {
            Present::No
        };

        Self::Detected(target_state)
    }

    /// Require that an extraneous [`Signal`] must be present on top of the
    /// current one.
    #[inline]
    pub const fn and(&self, target_signal: Self) -> Self {
        match (self, target_signal) {
            (&Signal::Guaranteed(ref present_left), Signal::Guaranteed(present_right)) => {
                Self::Guaranteed(Present::and(present_left, present_right))
            }
            (&Signal::Guaranteed(ref present_left), Signal::Detected(present_right))
            | (&Signal::Detected(ref present_left), Signal::Guaranteed(present_right))
            | (&Signal::Detected(ref present_left), Signal::Detected(present_right)) => {
                Self::Detected(Present::and(present_left, present_right))
            }
        }
    }
}

impl Signal {
    /// Determine whether this [`Signal`] indicates a target [`Present`] state,
    /// regardless of signal source.
    #[inline]
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
    pub const fn and(&self, target_right: Self) -> Self {
        match (self, target_right) {
            (Present::Yes, Present::Yes) => Self::Yes,
            (..) => Present::No,
        }
    }
}

/// A trait that encodes a possibly-absent optional extension in this processor.
///
/// # Feature Baselines
///
/// When desired, the runtime detection of specific processor core features can
/// be superceded altogether.
///
/// This is done through *Feature Baselines*, a mechanism where the
/// *configuration flags* can force the result of this trait to be fixed for
/// select implementor types.
pub trait Feature {
    /// The predetermined [`Signal`] for this [`Feature`].
    const PREDETERMINED: Option<Signal> = None;

    /// Determine whether the feature is supported by the current processor
    /// core, or a *Feature Baseline* is present.
    #[inline]
    fn supported() -> Signal {
        Self::PREDETERMINED.unwrap_or(Signal::unknown())
    }
}
