//! Single-bit bitwise fields for an already-existing integer.
//!
//! See [`BitDyn`] and [`BitDynMut`] for additional information.

use crate::{bit::BitOp, field::dynamic::select::Selected, prelude::State};

/// A managed immutable handle to a bit in the integer `B`.
///
/// Equality and ordering compare the selected [`State`].
#[derive(Debug, Copy, Clone)]
// NOTE(invariant): The stored `Selected<B>` proves the selected bit is within the borrowed value.
pub struct BitDyn<'a, B>(&'a B, Selected<B>)
where
    B: BitOp;

impl<'a, B> BitDyn<'a, B>
where
    B: BitOp,
{
    /// Construct a new [`BitDyn`] wrapper over an existing integer at a target
    /// bit index.
    #[inline]
    pub const fn wrap(target_value: &'a B, target_indice: Selected<B>) -> Self {
        Self(target_value, target_indice)
    }

    /// Move the currently-selected bit index inside this newtype wrapper to a
    /// brand-new one.
    #[inline]
    #[must_use]
    pub const fn to(self, target_indice: Selected<B>) -> Self {
        let Self(target_value, ..) = self;

        Self(target_value, target_indice)
    }

    /// Determine the [`Selected`] bit index.
    #[inline]
    #[must_use]
    pub const fn selected(&self) -> Selected<B> {
        let &Self(.., target_value) = self;

        target_value
    }
}

impl<B> BitDyn<'_, B>
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
///
/// Equality and ordering compare the selected [`State`], as with [`BitDyn`].
#[derive(Debug)]
// NOTE(invariant): The stored `Selected<B>` proves the selected bit is within the borrowed value.
pub struct BitDynMut<'a, B>(&'a mut B, Selected<B>)
where
    B: BitOp;

impl<'a, B> BitDynMut<'a, B>
where
    B: BitOp,
{
    /// Construct a new [`BitDynMut`] wrapper over an existing integer at a
    /// target bit index.
    #[inline]
    pub const fn wrap(target_value: &'a mut B, target_indice: Selected<B>) -> Self {
        Self(target_value, target_indice)
    }

    /// Move the currently-selected bit index inside this newtype wrapper to a
    /// brand-new one.
    #[inline]
    #[must_use]
    pub const fn to(self, target_indice: Selected<B>) -> Self {
        let Self(target_value, ..) = self;

        Self(target_value, target_indice)
    }
}

impl<B> BitDynMut<'_, B>
where
    B: BitOp,
{
    /// Determine the [`State`] of this [`BitDyn`].
    #[inline]
    #[must_use]
    pub fn state(&self) -> State {
        let Self(target_value, target_indice) = self;

        <B as BitOp>::get(*target_value, *target_indice)
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
        let Self(target_value, target_indice) = self;

        <B as BitOp>::cleared(*target_value, *target_indice)
    }

    /// Make a copy of the underlying integer `B` with the target bit set.
    #[inline]
    #[must_use]
    pub fn enabled(&self) -> B {
        let Self(target_value, target_indice) = self;

        <B as BitOp>::enabled(*target_value, *target_indice)
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

/// Implement comparisons over a runtime-selected bit's projected state.
macro_rules! compare {
    () => {};
    ($($target_type:ident),* $(,)?) => {
        crate::reexport::tokel::stream! {
            $(
            impl<B> PartialEq for $target_type<'_, B>
            where
                B: BitOp,
            {
                #[inline]
                fn eq(&self, target_value: &Self) -> bool {
                    self.state() == target_value.state()
                }
            }

            impl<B> Eq for $target_type<'_, B> where B: BitOp {}

            impl<B> Ord for $target_type<'_, B>
            where
                B: BitOp,
            {
                #[inline]
                fn cmp(&self, target_value: &Self) -> core::cmp::Ordering {
                    self.state().cmp(&target_value.state())
                }
            }

            impl<B> PartialOrd for $target_type<'_, B>
            where
                B: BitOp,
            {
                #[inline]
                fn partial_cmp(&self, target_value: &Self) -> Option<core::cmp::Ordering> {
                    Some(self.cmp(target_value))
                }
            }

            )*
        }


    };
}

compare!(BitDyn, BitDynMut);

#[cfg(test)]
mod tests {
    //! Compare projected states independently of their selected indices.

    use super::{BitDyn, BitDynMut};
    use crate::field::dynamic::select::Selected;

    /// Different backing integers and indices can select the same state.
    #[test]
    fn projected_comparisons() {
        let left = 1_u8;
        let right = 0x80_u8;
        let low = Selected::<u8>::new(u32::MIN);
        let high = Selected::<u8>::new(7);

        assert_eq!(BitDyn::wrap(&left, low), BitDyn::wrap(&right, high));
        assert!(BitDyn::wrap(&0xfe, low) < BitDyn::wrap(&left, low));

        let mut left = left;
        let mut right = right;

        assert_eq!(BitDynMut::wrap(&mut left, low), BitDynMut::wrap(&mut right, high));
    }
}
