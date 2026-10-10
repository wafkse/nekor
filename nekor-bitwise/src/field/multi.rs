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
/// # Field Width
///
/// `usize` fields use the target pointer width and retain the default fixed-width
/// output selected by [`Size`]. A range extending beyond that width is rejected.
///
/// ```compile_fail
/// use nekor_bitwise::field::multi::Field;
///
/// let value = 0_usize;
/// let _ = Field::<0, { usize::BITS as usize }, usize>::wrap(&value);
/// ```
///
/// The output type must match the field width's selection.
///
/// ```compile_fail
/// use nekor_bitwise::field::multi::Field;
///
/// let value = 0_usize;
/// let _ = Field::<0, 7, usize, u16>::wrap(&value);
/// ```
///
/// # Value Semantics
///
/// Equality, hashing, and ordering use the extracted field value. Dereferencing
/// returns the borrowed source integer, including its other fields.
///
/// # Const Evaluation
///
/// [`Field::const_value()`] provides a const-compatible counterpart to
/// [`Field::value()`] for primitive integers.
#[repr(transparent)]
// NOTE(invariant): The `Extract<N, M>` bound fixes a valid field and its output representation for
// the borrowed value.
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

    /// The source-positioned bitwise mask identifying this [`Field`]'s bits in `E`.
    ///
    /// [`Extract::EXTRACT_MASK`] identifies the selected source positions.
    /// [`Extract::BITSET_MASK`] describes the corresponding low-aligned positions.
    ///
    /// ```
    /// use nekor_bitwise::field::multi::Field;
    ///
    /// type Tag<'value> = Field<'value, 4, 7, usize>;
    /// const MASK: usize = Tag::mask();
    ///
    /// assert_eq!(MASK, 0xf0);
    /// ```
    #[inline]
    #[must_use]
    pub const fn mask() -> E {
        E::EXTRACT_MASK
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
    #[inline]
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
    E: Extract<N, M, Output = O>,
    O: hash::Hash,
{
    #[inline]
    fn hash<H>(&self, state: &mut H)
    where
        H: hash::Hasher,
    {
        self.value().hash(state);
    }
}

impl<const N: usize, const M: usize, E, O> Ord for Field<'_, N, M, E, O>
where
    E: Extract<N, M, Output = O>,
    O: Ord,
{
    #[inline]
    fn cmp(&self, target_value: &Self) -> Ordering {
        self.value().cmp(&target_value.value())
    }
}

impl<const N: usize, const M: usize, E, O> PartialOrd for Field<'_, N, M, E, O>
where
    E: Extract<N, M, Output = O>,
    O: PartialOrd,
{
    #[inline]
    fn partial_cmp(&self, target_value: &Self) -> Option<Ordering> {
        self.value().partial_cmp(&target_value.value())
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
    E: Extract<N, M, Output = O>,
    O: PartialEq,
{
    #[inline]
    fn eq(&self, target_value: &Self) -> bool {
        self.value() == target_value.value()
    }
}

impl<const N: usize, const M: usize, E, O> Eq for Field<'_, N, M, E, O>
where
    E: Extract<N, M, Output = O>,
    O: Eq,
{
}

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

#[cfg(target_pointer_width = "16")]
field!(usize for [u8, u16]);

#[cfg(target_pointer_width = "32")]
field!(usize for [u8, u16, u32]);

#[cfg(target_pointer_width = "64")]
field!(usize for [u8, u16, u32, u64]);

/// A mutable field new-type to interact with a contiguous segment of bits in a
/// `E`.
///
/// # Value Semantics
///
/// Equality, hashing, and ordering use the extracted value, as with [`Field`].
/// Dereferencing returns the borrowed source integer.
///
/// # Const Evaluation
///
/// [`FieldMut::const_value()`] and [`FieldMut::const_merge()`] provide
/// const-compatible counterparts to [`FieldMut::value()`] and
/// [`FieldMut::merge()`] for primitive integers.
#[repr(transparent)]
// NOTE(invariant): The `Extract<N, M>` bound fixes a valid field and its output representation for
// the mutably borrowed value.
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

    /// Replace the selected field and return its previous extracted value.
    ///
    /// Input bits exceeding the field width are discarded. Every bit outside
    /// the selected field is preserved. The output type supplies the field's
    /// integer storage capacity.
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
    #[inline]
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
    E: Extract<N, M, Output = O>,
    O: hash::Hash,
{
    #[inline]
    fn hash<H>(&self, state: &mut H)
    where
        H: hash::Hasher,
    {
        self.value().hash(state);
    }
}

impl<const N: usize, const M: usize, E, O> Ord for FieldMut<'_, N, M, E, O>
where
    E: Extract<N, M, Output = O>,
    O: Ord,
{
    #[inline]
    fn cmp(&self, target_value: &Self) -> Ordering {
        self.value().cmp(&target_value.value())
    }
}

impl<const N: usize, const M: usize, E, O> PartialOrd for FieldMut<'_, N, M, E, O>
where
    E: Extract<N, M, Output = O>,
    O: PartialOrd,
{
    #[inline]
    fn partial_cmp(&self, target_value: &Self) -> Option<Ordering> {
        self.value().partial_cmp(&target_value.value())
    }
}

impl<const N: usize, const M: usize, E, O> PartialEq for FieldMut<'_, N, M, E, O>
where
    E: Extract<N, M, Output = O>,
    O: PartialEq,
{
    #[inline]
    fn eq(&self, target_value: &Self) -> bool {
        self.value() == target_value.value()
    }
}

impl<const N: usize, const M: usize, E, O> Eq for FieldMut<'_, N, M, E, O>
where
    E: Extract<N, M, Output = O>,
    O: Eq,
{
}

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

                /// Replace the selected field and return its previous extracted value.
                ///
                /// This is the const-compatible counterpart of [`FieldMut::merge()`].
                /// Excess input bits are discarded and bits outside the field are preserved.
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

#[cfg(target_pointer_width = "16")]
field_mut!(usize for [u8, u16]);

#[cfg(target_pointer_width = "32")]
field_mut!(usize for [u8, u16, u32]);

#[cfg(target_pointer_width = "64")]
field_mut!(usize for [u8, u16, u32, u64]);

#[cfg(test)]
mod tests {
    //! Test field projections, comparison semantics, and truncating merges.

    extern crate std;

    #[cfg(target_pointer_width = "16")]
    use core::primitive::u16 as Word;
    #[cfg(target_pointer_width = "32")]
    use core::primitive::u32 as Word;
    #[cfg(target_pointer_width = "64")]
    use core::primitive::u64 as Word;
    use core::{
        cmp::Ordering,
        hash::{Hash, Hasher},
    };
    use std::hash::DefaultHasher;

    use super::{
        Field, U8High4, U8High4Mut, U8Low4, U8Low4Mut, U16High8, U16High8Mut, U16Low8, U16Low8Mut, U32High16,
        U32High16Mut, U32Low16, U32Low16Mut, U64High32, U64High32Mut, U64Low32, U64Low32Mut, U128High64, U128High64Mut,
        U128Low64, U128Low64Mut,
    };
    use crate::prelude::{Counterpart, Extract};

    /// Verify that comparisons and hashes depend only on the projection.
    #[test]
    fn projected_comparisons_and_hashes() {
        type Low<'value> = Field<'value, 0, 7, u16>;
        type LowMut<'value> = <Low<'value> as Counterpart>::Mut;

        let left = 0x0101_u16;
        let right = 0x0201_u16;
        let left_field = Low::wrap(&left);
        let right_field = Low::wrap(&right);

        assert_eq!(left_field, right_field);
        assert_eq!(left_field.cmp(&right_field), Ordering::Equal);
        assert_eq!(left_field.partial_cmp(&right_field), Some(Ordering::Equal));
        assert_eq!(*left_field, left);
        assert_eq!(*right_field, right);

        let mut left_hash = DefaultHasher::new();
        let mut right_hash = DefaultHasher::new();

        left_field.hash(&mut left_hash);
        right_field.hash(&mut right_hash);
        assert_eq!(left_hash.finish(), right_hash.finish());

        let mut left = 0x0102_u16;
        let mut right = 0x0201_u16;
        let left_field = LowMut::wrap(&mut left);
        let right_field = LowMut::wrap(&mut right);

        assert!(left_field > right_field);
        assert_eq!(left_field.cmp(&right_field), Ordering::Greater);
        assert_eq!(left_field.partial_cmp(&right_field), Some(Ordering::Greater));

        let mut left = 0x0101_u16;
        let mut right = 0x0201_u16;
        let left_field = LowMut::wrap(&mut left);
        let right_field = LowMut::wrap(&mut right);
        let mut left_hash = DefaultHasher::new();
        let mut right_hash = DefaultHasher::new();

        assert_eq!(left_field, right_field);
        left_field.hash(&mut left_hash);
        right_field.hash(&mut right_hash);
        assert_eq!(left_hash.finish(), right_hash.finish());
    }

    #[test]
    fn pointer_fields_extract_and_merge_without_touching_outside_bits() {
        type Tag<'value> = Field<'value, 4, 7, usize>;
        type TagMut<'value> = <Tag<'value> as Counterpart>::Mut;

        const MASK: usize = Tag::mask();
        const VALUE: u8 = Tag::wrap(&0xa5_usize).const_value();
        const MERGED: (usize, u8, u8) = {
            let mut value = usize::MAX;
            let mut field = TagMut::wrap(&mut value);

            let previous = field.const_merge(0xa5);
            let current = field.const_value();

            (value, previous, current)
        };

        assert_eq!(MASK, 0xf0);
        assert_eq!(MASK, <usize as Extract<4, 7>>::EXTRACT_MASK);
        assert_eq!(U8High4::mask(), 0xf0);
        assert_eq!(U128High64::mask(), 0xffff_ffff_ffff_ffff_0000_0000_0000_0000);
        assert_eq!(VALUE, 0xa);
        assert_eq!(MERGED, (usize::MAX ^ 0xa0, 0xf, 0x5));

        let (merged_value, ..) = MERGED;

        let mut value = usize::MAX;
        let mut field = TagMut::wrap(&mut value);

        assert_eq!(field.value(), 0xf);
        assert_eq!(field.merge(0xa5), 0xf);
        assert_eq!(field.value(), 0x5);
        assert_eq!(value, merged_value);
        assert_eq!(Tag::wrap(&value).value(), 0x5);
    }

    #[test]
    fn pointer_fields_support_high_and_full_width_const_operations() {
        const HIGH: usize = usize::BITS as usize - 1;
        const START: usize = usize::BITS as usize - 8;

        type High<'value> = Field<'value, START, HIGH, usize>;
        type HighMut<'value> = <High<'value> as Counterpart>::Mut;
        type Full<'value> = Field<'value, 0, HIGH, usize>;
        type FullMut<'value> = <Full<'value> as Counterpart>::Mut;

        const HIGH_MASK: usize = High::mask();
        const HIGH_VALUE: u8 = High::wrap(&usize::MAX).const_value();
        const HIGH_MERGED: (usize, u8, u8) = {
            let mut value = usize::MAX;
            let mut field = HighMut::wrap(&mut value);

            let previous = field.const_merge(0);
            let current = field.const_value();

            (value, previous, current)
        };
        const FULL_VALUE: Word = Full::wrap(&usize::MAX).const_value();
        const FULL_MERGED: (usize, Word, Word) = {
            let mut value = usize::MAX;
            let mut field = FullMut::wrap(&mut value);

            let previous = field.const_merge(0x1234);
            let current = field.const_value();

            (value, previous, current)
        };

        assert_eq!(HIGH_VALUE, u8::MAX);
        assert_eq!(HIGH_MERGED, (usize::MAX >> 8, u8::MAX, 0));
        #[cfg(target_pointer_width = "16")]
        assert_eq!(HIGH_MASK, 0xff00);
        #[cfg(target_pointer_width = "32")]
        assert_eq!(HIGH_MASK, 0xff00_0000);
        #[cfg(target_pointer_width = "64")]
        assert_eq!(HIGH_MASK, 0xff00_0000_0000_0000);
        assert_eq!(FULL_VALUE, Word::MAX);
        assert_eq!(FULL_MERGED, (0x1234, Word::MAX, 0x1234));
        assert_eq!(Full::mask(), usize::MAX);

        let (high_merged_value, ..) = HIGH_MERGED;

        let (full_merged_value, ..) = FULL_MERGED;

        let mut value = usize::MAX;

        assert_eq!(HighMut::wrap(&mut value).merge(0), u8::MAX);
        assert_eq!(value, high_merged_value);
        assert_eq!(High::wrap(&value).value(), 0);
        assert_eq!(FullMut::wrap(&mut value).merge(0x1234), Word::MAX >> 8);
        assert_eq!(value, full_merged_value);
        assert_eq!(Full::wrap(&value).value(), 0x1234);
    }

    #[test]
    fn pointer_fields_support_u16_const_output() {
        type Low<'value> = Field<'value, 0, 15, usize>;
        type LowMut<'value> = <Low<'value> as Counterpart>::Mut;

        const VALUE: u16 = Low::wrap(&usize::MAX).const_value();
        const MERGED: (usize, u16, u16) = {
            let mut value = usize::MAX;
            let mut field = LowMut::wrap(&mut value);

            let previous = field.const_merge(0x1234);
            let current = field.const_value();

            (value, previous, current)
        };

        assert_eq!(VALUE, u16::MAX);
        assert_eq!(MERGED, (usize::MAX ^ 0xedcb, u16::MAX, 0x1234));
        assert_eq!(Low::mask(), 0xffff);
    }

    #[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
    #[test]
    fn pointer_fields_support_u32_const_output() {
        type Low<'value> = Field<'value, 0, 31, usize>;
        type LowMut<'value> = <Low<'value> as Counterpart>::Mut;

        const VALUE: u32 = Low::wrap(&usize::MAX).const_value();
        const MERGED: (usize, u32, u32) = {
            let mut value = usize::MAX;
            let mut field = LowMut::wrap(&mut value);

            let previous = field.const_merge(0x1234_5678);
            let current = field.const_value();

            (value, previous, current)
        };

        assert_eq!(VALUE, u32::MAX);
        assert_eq!(MERGED, (usize::MAX ^ 0xedcb_a987, u32::MAX, 0x1234_5678));
        assert_eq!(Low::mask(), 0xffff_ffff);
    }

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
