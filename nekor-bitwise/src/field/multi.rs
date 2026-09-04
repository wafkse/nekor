//! Multi-bit bitwise fields for an already-existing integer.
//!
//! See [`Field`] and [`FieldMut`] for additional information.

use core::{cmp::Ordering, fmt, hash, marker, ops::Deref};

use crate::{
    field::counterpart::Counterpart,
    prelude::{Extract, Extractor, For2, Size},
};

/// A field new-type to interact with a contiguous segment of bits in an `E`.
///
/// This item incorporates total or partial *const-fn* support, const-evaluable
/// alternatives are outlined as:
///
/// - [`Field::const_value`] as [`Field::value`]
///
/// These are identical in function prototype, modulo *constness* coloring.
#[repr(transparent)]
pub struct Field<'a, const N: usize, const M: usize, E, O = <Size as For2<N, M>>::Target>(
    &'a E,
    marker::PhantomData<fn() -> O>,
)
where
    E: Extract<N, M, Output = O>;

impl<'a, const N: usize, const M: usize, E, O> Field<'a, N, M, E, O>
where
    E: Extract<N, M, Output = O>,
{
    /// Wrap the target value `E` into this [`Field`] wrapper.
    #[inline]
    pub const fn wrap(target_value: &'a E) -> Self {
        Self(target_value, marker::PhantomData)
    }

    /// Determine the value of this [`Field`], as the correspondant
    /// [`Extract::Output`] type.
    #[inline]
    #[must_use]
    pub fn value(&self) -> O {
        let &Self(target_value, ..) = self;

        <E as Extract<N, M>>::extract(target_value)
    }
}

impl<'a, const N: usize, const M: usize, E, O> Counterpart for Field<'a, N, M, E, O>
where
    E: Extract<N, M, Output = O>,
{
    type Immut = Self;
    type Mut = FieldMut<'a, N, M, E, O>;
}

impl<const N: usize, const M: usize, E, O> Deref for Field<'_, N, M, E, O>
where
    E: Extract<N, M, Output = O>,
{
    type Target = E;

    #[inline]
    fn deref(&self) -> &Self::Target {
        let &Self(target_value, ..) = self;

        target_value
    }
}

impl<const N: usize, const M: usize, E, O> fmt::Debug for Field<'_, N, M, E, O>
where
    E: Extract<N, M, Output = O> + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let &Self(target_value, ..) = self;

        f.debug_tuple("Field").field(target_value).finish_non_exhaustive()
    }
}

impl<const N: usize, const M: usize, E, O> fmt::Display for Field<'_, N, M, E, O>
where
    E: Extract<N, M, Output = O> + fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as fmt::Debug>::fmt(self, f)
    }
}

impl<const N: usize, const M: usize, E, O> hash::Hash for Field<'_, N, M, E, O>
where
    E: Extract<N, M, Output = O> + hash::Hash,
{
    #[inline]
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        let &Self(target_value, ..) = self;

        target_value.hash(state);
    }
}

impl<const N: usize, const M: usize, E, O> Ord for Field<'_, N, M, E, O>
where
    E: Extract<N, M, Output = O> + Ord,
{
    #[inline]
    fn cmp(&self, &Self(right_value, ..): &Self) -> Ordering {
        let &Self(left_value, ..) = self;

        left_value.cmp(right_value)
    }
}

impl<const N: usize, const M: usize, E, O> PartialOrd for Field<'_, N, M, E, O>
where
    E: Extract<N, M, Output = O> + PartialOrd,
{
    #[inline]
    fn partial_cmp(&self, &Self(right_value, ..): &Self) -> Option<Ordering> {
        let &Self(left_value, ..) = self;

        left_value.partial_cmp(right_value)
    }
}

impl<const N: usize, const M: usize, E, O> Clone for Field<'_, N, M, E, O>
where
    E: Extract<N, M, Output = O> + Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<const N: usize, const M: usize, E, O> Copy for Field<'_, N, M, E, O> where E: Extract<N, M, Output = O> + Copy {}

impl<const N: usize, const M: usize, E, O> PartialEq for Field<'_, N, M, E, O>
where
    E: Extract<N, M, Output = O> + PartialEq,
{
    #[inline]
    fn eq(&self, &Self(right_value, ..): &Self) -> bool {
        let &Self(left_value, ..) = self;

        left_value == right_value
    }
}

impl<const N: usize, const M: usize, E, O> Eq for Field<'_, N, M, E, O> where E: Extract<N, M, Output = O> + Eq {}

/// A helper macro to implement the distinct combinations of the [`Field`]
/// implementors.
macro_rules! field {
    () => {};
    (
        $(
            #[$target_meta:meta]
        )*

        $target_type:ident for [$($target_output:ident),+ $(,)?]
    ) => {
       $(
            impl<'a, const N: usize, const M: usize> Field<'a, N, M, $target_type, $target_output>
            where
                $target_type: Extract<N, M, Output = $target_output>,
            {
                /// Determine the value of this [`Field`], as the correspondant [`Extract::Output`] type.
                ///
                /// This is a const-fn-compatible alternative to the `value` associated function of this item.
                #[inline]
                pub const fn const_value(&self) -> $target_output {
                    let &Self(target_value, ..) = self;

                    Extractor::<N, M, $target_type, $target_output,>::extract(target_value)
                }
            }
       )+
    };
}

field!(u8 for [u8]);

field!(u16 for [u8, u16]);

field!(u32 for [u8, u16, u32]);

field!(u64 for [u8, u16, u32, u64]);

/// A mutable field new-type to interact with a contiguous segment of bits in a
/// `E`.
///
/// This item incorporates total or partial *const-fn* support, const-evaluable
/// alternatives are outlined as:
///
/// - [`FieldMut::const_value`] as [`FieldMut::value`]
/// - [`FieldMut::const_merge`] as [`FieldMut::merge`]
///
/// These are identical in function prototype, modulo *constness* coloring.
#[repr(transparent)]
pub struct FieldMut<'a, const N: usize, const M: usize, E, O = <Size as For2<N, M>>::Target>(
    &'a mut E,
    marker::PhantomData<fn() -> O>,
)
where
    E: Extract<N, M, Output = O>;

impl<'a, const N: usize, const M: usize, E, O> FieldMut<'a, N, M, E, O>
where
    E: Extract<N, M, Output = O>,
{
    /// Wrap the target value `E` into this [`FieldMut`] wrapper.
    #[inline]
    pub const fn wrap(target_value: &'a mut E) -> Self {
        Self(target_value, marker::PhantomData)
    }

    /// Determine the value of this [`FieldMut`], as the correspondant
    /// [`Extract::Output`] type.
    #[inline]
    #[must_use]
    pub fn value(&self) -> O {
        let Self(target_value, ..) = self;

        <E as Extract<N, M>>::extract(target_value)
    }

    /// Merge the value of this [`FieldMut`] and [`Extract::Output`] type.
    #[inline]
    pub fn merge(&mut self, target_input: O) -> O {
        let &mut Self(ref mut target_value, ..) = self;

        <E as Extract<N, M>>::merge(target_value, target_input)
    }
}

impl<'a, const N: usize, const M: usize, E, O> Counterpart for FieldMut<'a, N, M, E, O>
where
    E: Extract<N, M, Output = O>,
{
    type Immut = Field<'a, N, M, E, O>;
    type Mut = Self;
}

impl<const N: usize, const M: usize, E, O> Deref for FieldMut<'_, N, M, E, O>
where
    E: Extract<N, M, Output = O>,
{
    type Target = E;

    #[inline]
    fn deref(&self) -> &Self::Target {
        let Self(target_value, ..) = self;

        target_value
    }
}

impl<const N: usize, const M: usize, E, O> fmt::Debug for FieldMut<'_, N, M, E, O>
where
    E: Extract<N, M, Output = O> + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self(target_value, ..) = self;

        f.debug_tuple("FieldMut").field(target_value).finish_non_exhaustive()
    }
}

impl<const N: usize, const M: usize, E, O> fmt::Display for FieldMut<'_, N, M, E, O>
where
    E: Extract<N, M, Output = O> + fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as fmt::Debug>::fmt(self, f)
    }
}

impl<const N: usize, const M: usize, E, O> hash::Hash for FieldMut<'_, N, M, E, O>
where
    E: Extract<N, M, Output = O> + hash::Hash,
{
    #[inline]
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        let Self(target_value, ..) = self;

        target_value.hash(state);
    }
}

impl<const N: usize, const M: usize, E, O> Ord for FieldMut<'_, N, M, E, O>
where
    E: Extract<N, M, Output = O> + Ord,
{
    #[inline]
    fn cmp(&self, Self(right_value, ..): &Self) -> Ordering {
        let Self(left_value, ..) = self;

        left_value.cmp(right_value)
    }
}

impl<const N: usize, const M: usize, E, O> PartialOrd for FieldMut<'_, N, M, E, O>
where
    E: Extract<N, M, Output = O> + PartialOrd,
{
    #[inline]
    fn partial_cmp(&self, Self(right_value, ..): &Self) -> Option<Ordering> {
        let Self(left_value, ..) = self;

        left_value.partial_cmp(right_value)
    }
}

impl<const N: usize, const M: usize, E, O> PartialEq for FieldMut<'_, N, M, E, O>
where
    E: Extract<N, M, Output = O> + PartialEq,
{
    #[inline]
    fn eq(&self, Self(right_value, ..): &Self) -> bool {
        let Self(left_value, ..) = self;

        left_value == right_value
    }
}

impl<const N: usize, const M: usize, E, O> Eq for FieldMut<'_, N, M, E, O> where E: Extract<N, M, Output = O> + Eq {}

/// A helper macro to implement the distinct combinations of the [`FieldMut`]
/// implementors.
macro_rules! field_mut {
    () => {};
    (
        $(
            #[$target_meta:meta]
        )*

        $target_type:ident for [$($target_output:ident),+ $(,)?]
    ) => {
       $(
            impl<'a, const N: usize, const M: usize> FieldMut<'a, N, M, $target_type, $target_output>
            where
                $target_type: Extract<N, M, Output = $target_output>,
            {
                /// Determine the value of this [`FieldMut`], as the correspondant [`Extract::Output`] type.
                ///
                /// This is a const-fn-compatible alternative to the `value` associated function.
                #[inline]
                pub const fn const_value(&self) -> $target_output {
                    let &Self(ref target_value, ..) = self;

                    Extractor::<N, M, $target_type, $target_output>::extract(target_value)
                }

                /// Merge the value of this [`FieldMut`] and [`Extract::Output`] type.
                ///
                /// This is a const-fn-compatible alternative to the `merge` associated function in this item.
                #[inline]
                pub const fn const_merge(&mut self, target_input: $target_output) -> $target_output {
                    let &mut Self(ref mut target_value, ..) = self;

                    Extractor::<N, M, $target_type, $target_output>::merge(target_value, target_input)
                }
            }
       )+
    };
}

field_mut!(u8 for [u8]);

field_mut!(u16 for [u8, u16]);

field_mut!(u32 for [u8, u16, u32]);

field_mut!(u64 for [u8, u16, u32, u64]);
