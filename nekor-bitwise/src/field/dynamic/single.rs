//! Single-bit bitwise fields for an already-existing integer.
//!
//! See [`BitDyn`] and [`BitMutDyn`] for additional information.

use crate::{bit::BitOp, field::dynamic::select::Selected, prelude::State};

/// A managed immutable handle to a bit in the integer `B`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct BitDyn<'a, B>(&'a B, Selected<B>)
where
    B: BitOp;

impl<'a, B> BitDyn<'a, B>
where
    B: BitOp,
{
    /// Construct a new [`BitDyn`] wrapper over an existing integer at a target
    /// bit indice.
    #[inline]
    pub const fn wrap(target_value: &'a B, target_indice: Selected<B>) -> Self {
        Self(target_value, target_indice)
    }

    /// Move the currently-selected bit indice inside this newtype wrapper to a
    /// brand-new one.
    #[inline]
    #[must_use]
    pub const fn to(self, target_indice: Selected<B>) -> Self {
        let Self(target_value, ..) = self;

        Self(target_value, target_indice)
    }

    /// Determine the [`Selected`] bit indice.
    #[inline]
    #[must_use]
    pub const fn selected(&self) -> Selected<B> {
        let &Self(.., target_value) = self;

        target_value
    }
}

impl<'a, B> BitDyn<'a, B>
where
    B: BitOp,
{
    /// Determine the [`State`] of this [`BitDyn`].
    #[inline]
    #[must_use]
    pub fn state(&self) -> State {
        let &Self(target_value, target_indice) = self;

        <B as BitOp>::get(target_value, target_indice)
    }

    /// Determine whether the current bit [`State`] is the one provided.
    #[inline]
    #[must_use]
    pub fn is(&self, target_state: State) -> bool {
        self.state() == target_state
    }

    /// Make a copy of the underlying integer `B` with the target bit unset.
    #[inline]
    #[must_use]
    pub fn cleared(&self) -> B {
        let &Self(target_value, target_indice) = self;

        <B as BitOp>::cleared(target_value, target_indice)
    }

    /// Make a copy of the underlying integer `B` with the target bit set.
    #[inline]
    #[must_use]
    pub fn enabled(&self) -> B {
        let &Self(target_value, target_indice) = self;

        <B as BitOp>::enabled(target_value, target_indice)
    }
}

/// A managed mutable handle to a bit in the integer `B`.
#[derive(Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct BitDynMut<'a, B>(&'a mut B, Selected<B>)
where
    B: BitOp;

impl<'a, B> BitDynMut<'a, B>
where
    B: BitOp,
{
    /// Construct a new [`BitDynMut`] wrapper over an existing integer at a
    /// target bit indice.
    #[inline]
    pub const fn wrap(target_value: &'a mut B, target_indice: Selected<B>) -> Self {
        Self(target_value, target_indice)
    }

    /// Move the currently-selected bit indice inside this newtype wrapper to a
    /// brand-new one.
    #[inline]
    #[must_use]
    pub const fn to(self, target_indice: Selected<B>) -> Self {
        let Self(target_value, ..) = self;

        Self(target_value, target_indice)
    }
}

impl<'a, B> BitDynMut<'a, B>
where
    B: BitOp,
{
    /// Determine the [`State`] of this [`BitDyn`].
    #[inline]
    #[must_use]
    pub fn state(&self) -> State {
        let &Self(ref target_value, target_indice) = self;

        <B as BitOp>::get(target_value, target_indice)
    }
    /// Determine whether the current bit [`State`] is the one provided.
    #[inline]
    #[must_use]
    pub fn is(&self, target_state: State) -> bool {
        self.state() == target_state
    }

    /// Make a copy of the underlying integer `B` with the target bit unset.
    #[inline]
    #[must_use]
    pub fn cleared(&self) -> B {
        let &Self(ref target_value, target_indice) = self;

        <B as BitOp>::cleared(target_value, target_indice)
    }

    /// Make a copy of the underlying integer `B` with the target bit set.
    #[inline]
    #[must_use]
    pub fn enabled(&self) -> B {
        let &Self(ref target_value, target_indice) = self;

        <B as BitOp>::enabled(target_value, target_indice)
    }

    /// Set the state of the bit that this handle is for.
    #[inline]
    #[must_use]
    pub fn set(&mut self, target_state: State) -> State {
        let &mut Self(ref mut target_value, target_indice) = self;

        <B as BitOp>::set(target_value, target_indice, target_state)
    }

    /// Toggle the state of the bit that this handle is for.
    #[inline]
    #[must_use]
    pub fn toggle(&mut self) -> State {
        let &mut Self(ref mut target_value, target_indice) = self;

        <B as BitOp>::toggle(target_value, target_indice)
    }
}
