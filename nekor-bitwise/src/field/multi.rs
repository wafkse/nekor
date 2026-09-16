//! Multi-bit bitwise fields for an already-existing integer.
//!
//! See [`Field`] and [`FieldMut`] for additional information.

use core::{cmp::Ordering, fmt, hash, marker, ops::Deref};

use crate::{
    field::counterpart::Counterpart,
    prelude::{Extract, Extractor, For2, Size},
};

/// Low 4-bit field of one 8-bit word.
pub type U8Low4<'value> = Field<'value, 0, 3, u8>;

/// Mutable low 4-bit field of one 8-bit word.
pub type U8Low4Mut<'value> = <U8Low4<'value> as Counterpart>::Mut;

/// High 4-bit field of one 8-bit word.
pub type U8High4<'value> = Field<'value, 4, 7, u8>;

/// Mutable high 4-bit field of one 8-bit word.
pub type U8High4Mut<'value> = <U8High4<'value> as Counterpart>::Mut;

/// Low 8-bit field of one 16-bit word.
pub type U16Low8<'value> = Field<'value, 0, 7, u16>;

/// Mutable low 8-bit field of one 16-bit word.
pub type U16Low8Mut<'value> = <U16Low8<'value> as Counterpart>::Mut;

/// High 8-bit field of one 16-bit word.
pub type U16High8<'value> = Field<'value, 8, 15, u16>;

/// Mutable high 8-bit field of one 16-bit word.
pub type U16High8Mut<'value> = <U16High8<'value> as Counterpart>::Mut;

/// Low 16-bit field of one 32-bit word.
pub type U32Low16<'value> = Field<'value, 0, 15, u32>;

/// Mutable low 16-bit field of one 32-bit word.
pub type U32Low16Mut<'value> = <U32Low16<'value> as Counterpart>::Mut;

/// High 16-bit field of one 32-bit word.
pub type U32High16<'value> = Field<'value, 16, 31, u32>;

/// Mutable high 16-bit field of one 32-bit word.
pub type U32High16Mut<'value> = <U32High16<'value> as Counterpart>::Mut;

/// Low 32-bit field of one 64-bit word.
pub type U64Low32<'value> = Field<'value, 0, 31, u64>;

/// Mutable low 32-bit field of one 64-bit word.
pub type U64Low32Mut<'value> = <U64Low32<'value> as Counterpart>::Mut;

/// High 32-bit field of one 64-bit word.
pub type U64High32<'value> = Field<'value, 32, 63, u64>;

/// Mutable high 32-bit field of one 64-bit word.
pub type U64High32Mut<'value> = <U64High32<'value> as Counterpart>::Mut;

/// Low 64-bit field of one 128-bit word.
pub type U128Low64<'value> = Field<'value, 0, 63, u128>;

/// Mutable low 64-bit field of one 128-bit word.
pub type U128Low64Mut<'value> = <U128Low64<'value> as Counterpart>::Mut;

/// High 64-bit field of one 128-bit word.
pub type U128High64<'value> = Field<'value, 64, 127, u128>;

/// Mutable high 64-bit field of one 128-bit word.
pub type U128High64Mut<'value> = <U128High64<'value> as Counterpart>::Mut;

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

field!(u128 for [u8, u16, u32, u64, u128]);

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

field_mut!(u128 for [u8, u16, u32, u64, u128]);

#[cfg(test)]
mod tests {
    use super::{
        U8High4, U8High4Mut, U8Low4, U8Low4Mut, U16High8, U16High8Mut, U16Low8, U16Low8Mut, U32High16, U32High16Mut,
        U32Low16, U32Low16Mut, U64High32, U64High32Mut, U64Low32, U64Low32Mut, U128High64, U128High64Mut, U128Low64,
        U128Low64Mut,
    };

    #[test]
    fn half_aliases_project_each_supported_unsigned_word() {
        let value = 0xa5_u8;
        assert_eq!(U8Low4::wrap(&value).const_value(), 0x5);
        assert_eq!(U8High4::wrap(&value).const_value(), 0xa);

        let value = 0x1234_u16;
        assert_eq!(U16Low8::wrap(&value).const_value(), 0x34);
        assert_eq!(U16High8::wrap(&value).const_value(), 0x12);

        let value = 0x1234_5678_u32;
        assert_eq!(U32Low16::wrap(&value).const_value(), 0x5678);
        assert_eq!(U32High16::wrap(&value).const_value(), 0x1234);

        let value = 0x1234_5678_9abc_def0_u64;
        assert_eq!(U64Low32::wrap(&value).const_value(), 0x9abc_def0);
        assert_eq!(U64High32::wrap(&value).const_value(), 0x1234_5678);

        let value = 0x0123_4567_89ab_cdef_fedc_ba98_7654_3210_u128;
        assert_eq!(U128Low64::wrap(&value).const_value(), 0xfedc_ba98_7654_3210);
        assert_eq!(U128High64::wrap(&value).const_value(), 0x0123_4567_89ab_cdef);
    }

    #[test]
    fn mutable_half_aliases_merge_without_touching_the_other_half() {
        let mut value = 0xa5_u8;
        U8Low4Mut::wrap(&mut value).const_merge(0x3);
        assert_eq!(value, 0xa3);

        let mut value = 0x1234_u16;
        U16High8Mut::wrap(&mut value).const_merge(0xab);
        assert_eq!(value, 0xab34);

        let mut value = 0x1234_5678_u32;
        U32Low16Mut::wrap(&mut value).const_merge(0xabcd);
        assert_eq!(value, 0x1234_abcd);

        let mut value = 0x1234_5678_9abc_def0_u64;
        U64High32Mut::wrap(&mut value).const_merge(0xabcd_ef01);
        assert_eq!(value, 0xabcd_ef01_9abc_def0);

        let mut value = 0x0123_4567_89ab_cdef_fedc_ba98_7654_3210_u128;
        U128Low64Mut::wrap(&mut value).const_merge(0x1111_2222_3333_4444);
        assert_eq!(value, 0x0123_4567_89ab_cdef_1111_2222_3333_4444);

        let mut value = 0_u8;
        U8High4Mut::wrap(&mut value).const_merge(0x5);
        assert_eq!(value, 0x50);

        let mut value = 0_u16;
        U16Low8Mut::wrap(&mut value).const_merge(0x6);
        assert_eq!(value, 0x0006);

        let mut value = 0_u32;
        U32High16Mut::wrap(&mut value).const_merge(0x1234);
        assert_eq!(value, 0x1234_0000);

        let mut value = 0_u64;
        U64Low32Mut::wrap(&mut value).const_merge(0x5678_9abc);
        assert_eq!(value, 0x0000_0000_5678_9abc);

        let mut value = 0_u128;
        U128High64Mut::wrap(&mut value).const_merge(0x0123_4567_89ab_cdef);
        assert_eq!(value, 0x0123_4567_89ab_cdef_0000_0000_0000_0000);
    }
}
