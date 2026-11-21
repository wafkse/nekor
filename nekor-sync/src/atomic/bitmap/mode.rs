//! Engagement modes for the [`crate::atomic::bitmap::AtomicBitmap`] atomic construct.

use core::{
    fmt::{self, Display},
    marker,
};

use crate::atomic::bitmap::at::At;

pub mod cooperative;

pub mod exclusive;

mod detail {
    //! Implementation details module for [`mode`].
    //!
    //! [`mode`]: super

    /// A trait to act as a seal supertrait for the [`InMode`] trait.
    ///
    /// [`InMode`]: super::InMode
    pub trait Sealed {}

    use super::{cooperative::Cooperative, exclusive::Exclusive};

    impl Sealed for Cooperative {}
    impl Sealed for Exclusive {}
}

/// An *engagement* mode for a particular bit in an [`AtomicBitmap`].
///
/// The associated functions in this trait mirror the behavior of the [`At`]
/// associated functions.
///
/// This trait is sealed and cannot be implemented outside of this crate.
///
/// # Safety
///
/// Each individual associated function in this trait must be properly
/// implemented with thread-safe or atomic properties.
///
/// Additionally, the associated [`Mode::Signal`] or [`Mode::Signal`] (via an
/// out-pointer to a non-freezed structure) associated types
///
/// Furthermore, the implementation of this type must document exactly what
/// guarantees it provides.
///
/// [`AtomicBitmap`]: crate::atomic::bitmap::AtomicBitmap
pub unsafe trait Mode: detail::Sealed {
    /// The output signal type.
    ///
    /// This is used to determine the outcome of the operation in a way that it
    /// is appropiate for the underlying mode.
    type Signal;

    /// The type of the arguments required by the engagement mode.
    ///
    /// This can be a miscenalleous type of data, such as a backoff strategy or
    /// a limit imposition.
    type State;

    /// Clear the engaged bit.
    ///
    /// # [`Mode`]-specific behavior
    ///
    /// On the [`exclusive::Exclusive`] mode, this Finalizer Operation can:
    ///
    /// - Block until the engaged bit is zeroed.
    /// - Bail out if the engaged bit was zeroed already, but not by us.
    #[must_use = "the mode signal may be particularly relevant"]
    fn zero_with(at: At<'_>, mode_state: &mut Self::State, in_mode: &InMode) -> Self::Signal;

    /// Set the engaged bit.
    ///
    /// # [`Mode`]-specific behavior
    ///
    /// On the [`exclusive::Exclusive`] mode, this Finalizer Operation can:
    ///
    /// - Block until the engaged bit is zeroed.
    /// - Bail out if the engaged bit was zeroed already, but not by us.
    #[must_use = "the mode signal may be particularly relevant"]
    fn one_with(at: At<'_>, mode_state: &mut Self::State, in_mode: &InMode) -> Self::Signal;
}

/// Permission to expose a bit selection's atomic word inside a mode finalizer.
///
/// [`At`] constructs this token when calling a [`Mode`] operation. The token lets that
/// operation obtain the selected word, snapshot and index through [`At::disengage`].
#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
// NOTE(invariant): Construction requires execution inside a bitmap mode finalizer.
pub struct InMode(marker::PhantomData<Self>);

impl InMode {
    /// Assert that the control-flow is in a [`Mode`] finalizer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the control-flow is indeed in a [`Mode`]
    /// finalizer.
    #[inline]
    #[must_use = "in-mode privilege token must be used in some way"]
    pub const unsafe fn affirmative() -> Self {
        Self(marker::PhantomData)
    }
}

impl Display for InMode {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<in-mode>")
    }
}

#[cfg(all(test, usermode))]
mod tests {}
