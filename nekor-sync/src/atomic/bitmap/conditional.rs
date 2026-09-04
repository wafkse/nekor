//! Conditional operations on [`AtomicBitmap`].

use core::{fmt, marker};

/// A condition to be applied to an [`AtomicBitmap`];
pub trait Condition {
    /// Determine the status of this condition.
    fn determine(self, target_value: usize) -> Status;
}

#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub enum Status {
    /// The condition is not met.
    Unmet,

    /// The condition was satisfied.
    ///
    /// This includes the observed value, which is guaranteed to:
    ///
    /// - Satisfy the condition.
    /// - Have been acquired with memory ordering [`Acquire`].
    ///
    /// [`Acquire`]: core::sync::atomic::Ordering::Acquire
    Satisfied(usize),
}

impl fmt::Debug for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            &Self::Unmet => write!(f, "Unmet"),
            &Self::Satisfied(target_snapshot) => {
                write!(f, "Satisfied({target_snapshot:#066b})")
            },
        }
    }
}

/// A [`Condition`] to determine whether the bitwise-and operation on the
/// [`AtomicBitmap`] yields zero, or, in other words, whether all the specified
/// bits are unset.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash)]
#[repr(transparent)]
pub struct Unset(pub usize);

impl Condition for Unset {
    #[inline]
    fn determine(self, target_value: usize) -> Status {
        let Self(target_mask) = self;

        match target_mask & target_value {
            0 => Status::Satisfied(target_value),
            1.. => Status::Unmet,
        }
    }
}

/// A [`Condition`] to determine whether the bitwise-and operation on the
/// [`AtomicBitmap`] yields the same condition value, or, in other words,
/// whether all the specified bits are set in unison.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash)]
#[repr(transparent)]
pub struct Set(pub usize);

impl Condition for Set {
    #[inline]
    fn determine(self, target_value: usize) -> Status {
        let Self(target_mask) = self;

        match target_mask & target_value {
            bitwise_result if bitwise_result == target_mask => Status::Satisfied(target_value),
            _ => Status::Unmet,
        }
    }
}

/// A wildcard [`Condition`] to determine whether any of the specified bits are
/// either set or unset, depending on the passed [`Compare`] type parameter.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash)]
#[repr(transparent)]
pub struct Wildcard<R>(pub usize, pub marker::PhantomData<R>)
where
    R: Compare;

impl Condition for Wildcard<Set> {
    #[inline]
    fn determine(self, target_value: usize) -> Status {
        let Self(target_mask, ..) = self;

        Compare::determine(Set(target_mask), target_value)
    }
}

impl Condition for Wildcard<Unset> {
    #[inline]
    fn determine(self, target_value: usize) -> Status {
        let Self(target_mask, ..) = self;

        Compare::determine(Unset(target_mask), target_value)
    }
}

/// An ad-hoc trait to be used for the [`Wildcard`] [`Condition`].
pub trait Compare {
    fn determine(self, target_value: usize) -> Status;
}

impl Compare for Set {
    #[inline]
    fn determine(self, target_value: usize) -> Status {
        let Self(target_mask) = self;

        if target_mask & target_value != 0 {
            Status::Satisfied(target_value)
        } else {
            Status::Unmet
        }
    }
}

impl Compare for Unset {
    #[inline]
    fn determine(self, target_value: usize) -> Status {
        let Self(target_mask) = self;

        if target_mask & target_value == target_mask {
            Status::Unmet
        } else {
            Status::Satisfied(target_value)
        }
    }
}
