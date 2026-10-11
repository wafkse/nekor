//! Tag descriptions and typed projections over pointer address fields.
//!
//! [`Maskable`] describes the capacity supplied by a pointee's alignment.
//! [`Field`] converts a complete tag value, while [`Bitfield`] describes
//! independently combinable variants represented by [`BitfieldSet`].
//!
//! [`Tagged`] and [`TaggedMut`] project a tag from an address image through
//! `nekor-bitwise` fields. Their pointer type determines the available alignment
//! bits, while their tag type determines the accepted logical values.
//!
//! The consuming abstraction retains the pointer and applies address changes
//! through provenance-preserving mapping. These views do not establish actual
//! pointer alignment, pointee ownership, lifetime, or protection against ABA.
//! Atomic storage and null-state semantics remain with the consumer.

use core::{fmt, marker, mem, ptr::NonNull};

use nekor_bitwise::{
    bit::BitOp,
    field::{dynamic::select::Selected, multi},
    prelude::{Counterpart, Extract},
};

/// A pointer representation whose pointee alignment provides tag capacity.
///
/// `MASK` describes the low bits that are zero when an address separately
/// satisfies the pointee's alignment requirement. It does not prove that every
/// arbitrary raw-pointer value of the implementing type is aligned.
///
/// # Safety
///
/// `MASK` must contain only bits made available by the represented pointee's
/// alignment. Users must still validate an actual pointer value before tagging
/// it unless another invariant already proves that alignment.
pub unsafe trait Maskable {
    /// The address-bit mask available for tagging.
    const MASK: usize;
}

/// Implements [`Maskable`] for a generic pointer representation.
///
/// The generic pointee type must be named `T` by the invocation.
macro_rules! maskable {
    () => {};
    (
        $(#[$target_meta:meta])*
        $target_type:ty
    ) => {
        $(#[$target_meta])*
        // SAFETY: Valid pointers to `T` are aligned to `align_of::<T>()`. Since
        // alignment is a power of two, subtracting one produces exactly the low
        // address bits guaranteed to be zero.
        unsafe impl<T> Maskable for $target_type {
            const MASK: usize = mem::align_of::<T>().wrapping_sub(1);
        }
    };
}

maskable!(*const T);
maskable!(*mut T);
maskable!(NonNull<T>);

/// An integer proven to contain no bits outside `M`.
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The contained value always satisfies `value & M == value`.
pub struct Masked<const M: usize>(usize);

impl<const M: usize> Masked<M> {
    /// Attempts to prove that `target_value` fits inside `M`.
    #[inline]
    #[must_use]
    pub const fn new(target_value: usize) -> Option<Self> {
        if target_value & M == target_value {
            Some(Self(target_value))
        } else {
            None
        }
    }

    /// Returns the proven masked value.
    #[inline]
    #[must_use]
    pub const fn unwrap(self) -> usize {
        let Self(target_value) = self;

        target_value
    }

    /// Returns the mask carried by this monomorphization.
    #[inline]
    #[must_use]
    pub const fn mask() -> usize {
        M
    }
}

mod private {
    //! Seals the pointer-tag encoding policies.

    /// A supertrait implemented only by built-in tag-category markers.
    pub trait Sealed {}
}

/// A type-level marker for an independently combinable tag bitfield.
pub enum TagBitfield {}

/// A type-level marker for one value encoded inside a contiguous tag field.
pub enum TagField {}

/// A type-level marker for a checked integer tag.
pub enum TagMasked {}

impl private::Sealed for TagBitfield {}

impl private::Sealed for TagField {}

impl private::Sealed for TagMasked {}

// SAFETY: `Masked<M>` admits exactly the integer values contained in `M`.
unsafe impl<const M: usize> Tag for Masked<M> {
    type Type = TagMasked;
    type Value = Self;

    const MASK: usize = M;
}

/// The shared layout metadata for a pointer-tag domain.
///
/// Conversion is intentionally absent from this trait. [`Field`] converts one
/// complete packed value, while [`Bitfield`] converts individual enum variants
/// as bit indices and uses [`BitfieldSet`] for combinations.
///
/// # Safety
///
/// Implementors must uphold all of the following conditions.
///
/// - `MASK` identifies every bit used by the unshifted tag encoding and no other bit.
/// - `Type` selects the semantic category implemented by this domain.
/// - `Value` is the logical value required by the selected category. [`Encoding::encode`] must
///   return `Some(encoded)` for every valid value, with no bits outside `MASK`.
/// - [`Encoding::decode`] must recover the same logical value from every emitted encoding.
/// - The mask, category, and conversions remain context-independent for the lifetime of every
///   projection using this domain.
pub unsafe trait Tag: Copy {
    /// The low-aligned mask of the tag encoding, independent of its address placement.
    const MASK: usize;

    /// The type-level semantic category of this tag domain.
    type Type;

    /// The complete logical value stored in the reserved address bits.
    type Value: Copy;
}

/// A tag domain whose enum variants represent independently combinable bits.
///
/// Each variant's numeric value is the base-two logarithm of its encoded bit.
/// In other words, a variant with index `N` is encoded as `1usize << N`.
/// Multiple variants are represented by [`BitfieldSet<Self>`], never by one
/// enum inhabitant.
///
/// # Safety
///
/// Implementors must uphold all of the following conditions.
///
/// - The [`Tag`] implementation uses [`TagBitfield`] and [`BitfieldSet<Self>`].
/// - [`Bitfield::index`] returns the numeric discriminant of the variant.
/// - Every returned index is less than [`usize::BITS`].
/// - The one-hot bit for every returned index is contained in [`Tag::MASK`].
/// - [`Bitfield::from_index`] accepts exactly the assigned variant indices.
/// - Converting a variant to its index and back returns the same variant.
pub unsafe trait Bitfield: Tag<Type = TagBitfield, Value = BitfieldSet<Self>> {
    /// Returns the bit index represented by this enum variant.
    fn index(self) -> u32;

    /// Attempts to decode one enum variant from its bit index.
    fn from_index(target_index: u32) -> Option<Self>;
}

/// A tag domain whose enum variants represent complete packed field values.
///
/// A field encoding occupies a contiguous range beginning at bit zero. A
/// [`Tagged`] view places that encoding in the selected address bits. One enum
/// inhabitant describes the field rather than one independently combinable bit.
///
/// # Safety
///
/// Implementors must uphold all of the following conditions.
///
/// - The [`Tag`] implementation uses [`TagField`] and `Self` as its value.
/// - [`Tag::MASK`] is a contiguous low-bit mask.
/// - [`Field::value`] never sets a bit outside [`Tag::MASK`].
/// - [`Field::from_value`] rejects values with bits outside [`Tag::MASK`] and every unassigned
///   field pattern.
/// - Converting an inhabitant to its value and back returns the same inhabitant.
pub unsafe trait Field: Tag<Type = TagField, Value = Self> {
    /// Returns the complete packed value represented by this enum variant.
    fn value(self) -> usize;

    /// Attempts to decode one enum variant from a complete packed value.
    fn from_value(target_value: usize) -> Option<Self>;
}

/// A validated set of independently combinable [`Bitfield`] variants.
///
/// The empty set is valid. Sparse bitfield domains are also supported, so a bit
/// inside the pointer's alignment capacity may remain unassigned.
#[repr(transparent)]
// NOTE(invariant): `bits` contains only indices accepted by `B::from_index` inside `B::MASK`.
pub struct BitfieldSet<B> {
    /// The one-hot union of all contained variants.
    bits: usize,

    /// The bitfield enum whose variants interpret the set bits.
    marker: marker::PhantomData<B>,
}

impl<B> BitfieldSet<B>
where
    B: Bitfield,
{
    /// Constructs an empty bitfield set.
    #[inline]
    #[must_use]
    pub const fn empty() -> Self {
        let bits = usize::MIN;
        let marker = marker::PhantomData;

        Self { bits, marker }
    }

    /// Attempts to construct a set from an encoded one-hot union.
    ///
    /// Returns [`None`] when any bit lies outside [`Tag::MASK`] or does not map
    /// to an assigned `B` variant.
    #[inline]
    #[must_use]
    pub fn from_bits(target_bits: usize) -> Option<Self> {
        if !Self::bits_are_valid(target_bits) {
            return None;
        }

        let bits = target_bits;
        let marker = marker::PhantomData;

        Some(Self { bits, marker })
    }

    /// Attempts to construct a set containing exactly one variant.
    #[inline]
    #[must_use]
    pub fn single(target_bit: B) -> Option<Self> {
        let target_bits = Self::variant_bit(target_bit)?;

        Self::from_bits(target_bits)
    }

    /// Returns the encoded one-hot union of this set.
    #[inline]
    #[must_use]
    pub const fn bits(self) -> usize {
        let Self { bits, .. } = self;

        bits
    }

    /// Determines whether this set contains `target_bit`.
    #[inline]
    #[must_use]
    pub fn contains(&self, target_bit: B) -> bool {
        let &Self { ref bits, .. } = self;

        let Some(target_mask) = Self::variant_bit(target_bit) else {
            return false;
        };

        *bits & target_mask != usize::MIN
    }

    /// Inserts `target_bit` into this set.
    ///
    /// Returns whether the variant was newly inserted. An invalid variant from
    /// a broken unsafe [`Bitfield`] implementation is rejected without changing
    /// the set.
    #[inline]
    pub fn insert(&mut self, target_bit: B) -> bool {
        let &mut Self { ref mut bits, .. } = self;

        let Some(target_mask) = Self::variant_bit(target_bit) else {
            return false;
        };

        let was_absent = *bits & target_mask == usize::MIN;

        *bits |= target_mask;

        was_absent
    }

    /// Removes `target_bit` from this set.
    ///
    /// Returns whether the variant was present before this operation.
    #[inline]
    pub fn remove(&mut self, target_bit: B) -> bool {
        let &mut Self { ref mut bits, .. } = self;

        let Some(target_mask) = Self::variant_bit(target_bit) else {
            return false;
        };

        let was_present = *bits & target_mask != usize::MIN;

        *bits &= !target_mask;

        was_present
    }

    /// Encodes one variant index as a one-hot bit through `nekor-bitwise`.
    #[inline]
    fn variant_bit(target_bit: B) -> Option<usize> {
        let target_index = B::index(target_bit);
        let selected_index = Selected::<usize>::try_new(target_index)?;
        let target_mask = <usize as BitOp>::single(selected_index);

        if target_mask & B::MASK != target_mask || B::from_index(target_index).is_none() {
            None
        } else {
            Some(target_mask)
        }
    }

    /// Validates every set bit against the bitfield domain.
    #[inline]
    fn bits_are_valid(target_bits: usize) -> bool {
        if target_bits & !B::MASK != usize::MIN {
            return false;
        }

        let mut remaining_bits = target_bits;

        while remaining_bits != usize::MIN {
            let target_index = remaining_bits.trailing_zeros();

            let Some(selected_index) = Selected::<usize>::try_new(target_index) else {
                return false;
            };

            if B::from_index(target_index).is_none() {
                return false;
            }

            remaining_bits &= !<usize as BitOp>::single(selected_index);
        }

        true
    }
}

impl<B> Clone for BitfieldSet<B> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<B> Copy for BitfieldSet<B> {}

impl<B> Default for BitfieldSet<B>
where
    B: Bitfield,
{
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}

impl<B> fmt::Debug for BitfieldSet<B> {
    #[inline]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let &Self { ref bits, .. } = self;

        formatter.debug_struct("BitfieldSet").field("bits", bits).finish()
    }
}

impl<B> PartialEq for BitfieldSet<B> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        let &Self { bits: ref left, .. } = self;

        let &Self { bits: ref right, .. } = other;

        left == right
    }
}

impl<B> Eq for BitfieldSet<B> {}

/// The sealed conversion policy for one pointer-tag category.
///
/// [`TagField`], [`TagBitfield`], and [`TagMasked`] provide policies for the
/// corresponding tag categories. The unsafe [`Tag`] contract governs the
/// selected policy, including implementations for a downstream tag domain.
pub trait Encoding<T>: private::Sealed
where
    T: Tag,
{
    /// Encodes one complete logical tag value.
    fn encode(target_value: T::Value) -> Option<usize>;

    /// Attempts to decode one complete logical tag value.
    fn decode(encoded: usize) -> Option<T::Value>;
}

impl<T> Encoding<T> for TagField
where
    T: Field,
{
    #[inline]
    fn encode(target_value: T::Value) -> Option<usize> {
        Some(T::value(target_value))
    }

    #[inline]
    fn decode(encoded: usize) -> Option<T::Value> {
        T::from_value(encoded)
    }
}

impl<T> Encoding<T> for TagBitfield
where
    T: Bitfield,
{
    #[inline]
    fn encode(target_value: T::Value) -> Option<usize> {
        Some(target_value.bits())
    }

    #[inline]
    fn decode(encoded: usize) -> Option<T::Value> {
        BitfieldSet::<T>::from_bits(encoded)
    }
}

impl<const M: usize> Encoding<Masked<M>> for TagMasked {
    #[inline]
    fn encode(target_value: Masked<M>) -> Option<usize> {
        Some(target_value.unwrap())
    }

    #[inline]
    fn decode(encoded: usize) -> Option<Masked<M>> {
        Masked::new(encoded)
    }
}

/// The integer field underlying a pointer-tag projection.
type Area<'value, const N: usize, const M: usize> = multi::Field<'value, N, M, usize, <usize as Extract<N, M>>::Output>;

/// The mutable counterpart of [`Area`].
type AreaMut<'value, const N: usize, const M: usize> = <Area<'value, N, M> as Counterpart>::Mut;

/// An immutable projection of a tag from a pointer address image.
///
/// `P` describes the pointer representation and `T` describes the tag. The
/// selected address bits must fit the alignment capacity of `P`, and every
/// encoding admitted by `T` must fit the selected field. Both restrictions are
/// checked during const evaluation when the projection is constructed.
///
/// Insufficient pointee alignment is rejected even when a particular address
/// would happen to supply the requested bits.
///
/// ```compile_fail
/// use core::ptr::NonNull;
/// use nekor_sync::atomic::tagged::{Masked, Tagged};
///
/// let address = 0_usize;
/// let _ = Tagged::<0, 1, NonNull<u8>, Masked<3>>::wrap(&address);
/// ```
///
/// The borrowed integer is an address image, not an owning pointer. Its origin
/// and actual alignment remain the caller's responsibility. Apply changes to
/// the original pointer through address mapping rather than integer casts.
#[repr(transparent)]
// NOTE(invariant): The projected field decodes to a valid `T::Value`. Its positions fit
// `P::MASK`, and `T::MASK` fits the field's low-aligned encoding capacity.
pub struct Tagged<'value, const N: usize, const M: usize, P, T>(
    Area<'value, N, M>,
    marker::PhantomData<fn() -> (P, T)>,
)
where
    usize: Extract<N, M>,
    P: Maskable,
    T: Tag;

/// A mutable projection of a tag from a pointer address image.
///
/// Mutation accepts logical tag values and leaves the other address bits
/// unchanged. The underlying integer and raw mutable field are not exposed.
/// [`TaggedMut::clear`] consumes the projection because zero need not be an
/// assigned tag encoding.
///
/// The positioned field, rather than its unshifted width, must fit the pointer
/// alignment capacity.
///
/// ```compile_fail
/// use core::ptr::NonNull;
/// use nekor_sync::atomic::tagged::{Masked, TaggedMut};
///
/// let mut address = 0_usize;
/// let _ = TaggedMut::<1, 2, NonNull<u32>, Masked<3>>::wrap(&mut address);
/// ```
#[repr(transparent)]
// NOTE(invariant): The projected field decodes to a valid `T::Value`. Its positions fit
// `P::MASK`, and `T::MASK` fits the field's low-aligned encoding capacity.
pub struct TaggedMut<'value, const N: usize, const M: usize, P, T>(
    AreaMut<'value, N, M>,
    marker::PhantomData<fn() -> (P, T)>,
)
where
    usize: Extract<N, M>,
    P: Maskable,
    T: Tag;

mod detail {
    //! Shared operations for immutable and mutable tag projections.

    /// Implements operations shared by the statically declared projection types.
    macro_rules! projected {
        (impl $target_name:ident for($target_borrow:ty)use $target_area:ident) => {
            tokel::stream! {
                impl<'value, const N: usize, const M: usize, P, T> $target_name<'value, N, M, P, T>
                where
                    usize: Extract<N, M>,
                    <usize as Extract<N, M>>::Output: TryInto<usize>,
                    P: Maskable,
                    T: Tag,
                    T::Type: Encoding<T>,
                {
                    /// The source-positioned mask reserved by this projection.
                    #[inline]
                    #[must_use]
                    pub const fn mask() -> usize {
                        const {
                            assert!(
                                <usize as Extract<N, M>>::EXTRACT_MASK & !P::MASK == 0,
                                "tag field exceeds pointer alignment capacity"
                            );
                            assert!(
                                T::MASK & !<usize as Extract<N, M>>::BITSET_MASK == 0,
                                "tag encoding exceeds projected field capacity"
                            );
                        }

                        Area::<N, M>::mask()
                    }

                    /// Project an existing tag encoding from an address image.
                    ///
                    /// Returns [`None`] for an unassigned encoding, including bits
                    /// outside the tag's domain but inside the projected field.
                    #[inline]
                    #[must_use]
                    pub fn wrap(target_value: $target_borrow) -> Option<Self> {
                        _ = Self::mask();

                        let target_area = $target_area::<N, M>::wrap(target_value);

                        // SAFETY: The sealed `usize` extractor produces an unsigned
                        // field value no wider than `usize`.
                        let encoded = unsafe { target_area.value().try_into().unwrap_unchecked() };

                        match encoded & !T::MASK {
                            0 => <T::Type as Encoding<T>>::decode(encoded)
                                .map(|_| Self(target_area, marker::PhantomData)),
                            _ => None,
                        }
                    }

                    /// Read the logical value held by this projection.
                    #[inline]
                    #[must_use]
                    pub fn value(&self) -> T::Value {
                        let &Self(ref target_area, _) = self;

                        // SAFETY: The sealed `usize` extractor produces an unsigned
                        // field value no wider than `usize`.
                        let encoded = unsafe { target_area.value().try_into().unwrap_unchecked() };

                        // SAFETY: The projection invariant establishes an accepted
                        // encoding, and `Tag` requires context-independent decoding.
                        unsafe { <T::Type as Encoding<T>>::decode(encoded).unwrap_unchecked() }
                    }
                }
            }
        };
    }

    pub(in crate::atomic::tagged) use projected;
}

detail::projected!(impl Tagged for (&'value usize) use Area);
detail::projected!(impl TaggedMut for (&'value mut usize) use AreaMut);

impl<'value, const N: usize, const M: usize, P, T> TaggedMut<'value, N, M, P, T>
where
    usize: Extract<N, M>,
    <usize as Extract<N, M>>::Output: TryFrom<usize> + TryInto<usize>,
    P: Maskable,
    T: Tag,
    T::Type: Encoding<T>,
{
    /// Initialize a tag field, replacing even an unassigned existing encoding.
    ///
    /// Bits outside the selected field are unchanged. This does not prove that
    /// the address image came from an aligned pointer.
    ///
    /// A tag's encoding space must fit the field, not just its current value.
    ///
    /// ```compile_fail
    /// use core::ptr::NonNull;
    /// use nekor_sync::atomic::tagged::{Masked, TaggedMut};
    ///
    /// let mut address = 0_usize;
    /// let tag = Masked::<7>::new(1).expect("tag fits its own domain");
    /// let _ = TaggedMut::<0, 1, NonNull<u32>, Masked<7>>::new(&mut address, tag);
    /// ```
    #[inline]
    #[must_use]
    pub fn new(target_value: &'value mut usize, target_tag: T::Value) -> Self {
        _ = Self::mask();

        // SAFETY: `Tag` requires every logical value to have an encoding within `T::MASK`.
        let encoded = unsafe { <T::Type as Encoding<T>>::encode(target_tag).unwrap_unchecked() };

        // SAFETY: `Self::mask()` proves that `T::MASK` fits the field, whose sealed
        // extractor selects an output wide enough for every field value.
        let target_input = unsafe { <usize as Extract<N, M>>::Output::try_from(encoded).unwrap_unchecked() };

        let mut target_area = AreaMut::<N, M>::wrap(target_value);

        _ = target_area.merge(target_input);

        Self(target_area, marker::PhantomData)
    }

    /// Replace the tag and return its previous logical value.
    #[inline]
    pub fn merge(&mut self, target_tag: T::Value) -> T::Value {
        // SAFETY: `Tag` requires every logical value to have an encoding within `T::MASK`.
        let encoded = unsafe { <T::Type as Encoding<T>>::encode(target_tag).unwrap_unchecked() };

        // SAFETY: The projection invariant bounds `T::MASK` by the field capacity,
        // and the sealed extractor's output represents every field value.
        let target_input = unsafe { <usize as Extract<N, M>>::Output::try_from(encoded).unwrap_unchecked() };
        let previous_tag = self.value();

        let &mut Self(ref mut target_area, _) = self;

        _ = target_area.merge(target_input);

        previous_tag
    }

    /// Clear the reserved field and consume the projection.
    ///
    /// Returns the previous tag. Zero may be unassigned, so the cleared image
    /// must be validated again before it can be projected as a tag.
    ///
    /// ```compile_fail
    /// use core::ptr::NonNull;
    /// use nekor_sync::atomic::tagged::{Masked, TaggedMut};
    ///
    /// let mut address = 0_usize;
    /// let tag = Masked::<3>::new(1).expect("tag fits");
    /// let view = TaggedMut::<0, 1, NonNull<u32>, Masked<3>>::new(&mut address, tag);
    /// let _previous = view.clear();
    /// let _ = view.value();
    /// ```
    #[inline]
    pub fn clear(self) -> T::Value {
        let previous_tag = self.value();

        let Self(mut target_area, _) = self;

        // SAFETY: The sealed extractor's unsigned output always represents zero.
        let target_input = unsafe { <usize as Extract<N, M>>::Output::try_from(usize::MIN).unwrap_unchecked() };

        _ = target_area.merge(target_input);

        previous_tag
    }
}

impl<'value, const N: usize, const M: usize, P, T> Counterpart for Tagged<'value, N, M, P, T>
where
    usize: Extract<N, M>,
    P: Maskable,
    T: Tag,
{
    type Immut = Self;
    type Mut = TaggedMut<'value, N, M, P, T>;
}

impl<'value, const N: usize, const M: usize, P, T> Counterpart for TaggedMut<'value, N, M, P, T>
where
    usize: Extract<N, M>,
    P: Maskable,
    T: Tag,
{
    type Immut = Tagged<'value, N, M, P, T>;
    type Mut = Self;
}

impl<const N: usize, const M: usize, P, T> Clone for Tagged<'_, N, M, P, T>
where
    usize: Extract<N, M>,
    P: Maskable,
    T: Tag,
{
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<const N: usize, const M: usize, P, T> Copy for Tagged<'_, N, M, P, T>
where
    usize: Extract<N, M>,
    P: Maskable,
    T: Tag,
{
}

#[cfg(test)]
mod tests {
    use core::{mem, ptr::NonNull};

    use nekor_bitwise::{bit::BitAtExtractor, field::multi, prelude::Counterpart};

    use super::{
        Bitfield, BitfieldSet, Encoding, Field, Maskable, Masked, Tag, TagBitfield, TagField, Tagged, TaggedMut,
    };

    /// The field used by `TestTag`.
    type TestTagField<'value> = multi::Field<'value, 0, 1, usize>;

    #[derive(Debug, Clone, Copy, Eq, PartialEq)]
    #[repr(usize)]
    enum TestTag {
        Zero = 0,
        One = 1,
        Three = 3,
    }

    // SAFETY: The contiguous two-bit mask reserves every bit used by this
    // domain. `TagField` interprets its logical value as one `TestTag`.
    unsafe impl Tag for TestTag {
        type Type = TagField;
        type Value = Self;

        const MASK: usize = TestTagField::mask();
    }

    // SAFETY: The mask is one contiguous two-bit field. Every encoded variant
    // fits that field, and decoding rejects the sole unassigned pattern.
    unsafe impl Field for TestTag {
        fn value(self) -> usize {
            self as usize
        }

        fn from_value(target_value: usize) -> Option<Self> {
            match target_value {
                target_value if target_value == Self::Zero as usize => Some(Self::Zero),
                target_value if target_value == Self::One as usize => Some(Self::One),
                target_value if target_value == Self::Three as usize => Some(Self::Three),
                _ => None,
            }
        }
    }

    #[derive(Debug, Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    enum TestBit {
        First = 0,
        Third = 2,
    }

    // SAFETY: The mask is exactly the union of the one-hot bits at the assigned
    // variant indices. The logical tag value is their validated set.
    unsafe impl Tag for TestBit {
        type Type = TagBitfield;
        type Value = BitfieldSet<Self>;

        const MASK: usize = BitAtExtractor::<{ Self::First as usize }, usize>::single()
            | BitAtExtractor::<{ Self::Third as usize }, usize>::single();
    }

    // SAFETY: Each discriminant is a valid `usize` bit index. Index conversion
    // accepts exactly those discriminants and round-trips both variants.
    unsafe impl Bitfield for TestBit {
        fn index(self) -> u32 {
            self as u32
        }

        fn from_index(target_index: u32) -> Option<Self> {
            match target_index {
                target_index if target_index == Self::First as u32 => Some(Self::First),
                target_index if target_index == Self::Third as u32 => Some(Self::Third),
                _ => None,
            }
        }
    }

    #[repr(C, align(8))]
    struct AlignedBytes(pub [u8; 4]);

    /// Pointee alignment supplying the positioned test field.
    #[repr(C, align(128))]
    struct AlignedImage(pub u8);

    /// Pointee alignment supplying a field with a `u32` output.
    #[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
    #[repr(C, align(262144))]
    struct WideImage(pub u8);

    /// A three-bit field beginning above the low address bits.
    type Positioned<'value, T> = Tagged<'value, 4, 6, NonNull<AlignedImage>, T>;

    /// The mutable counterpart of the positioned test field.
    type PositionedMut<'value, T> = <Positioned<'value, T> as Counterpart>::Mut;

    /// A field domain with no assigned zero encoding.
    #[derive(Debug, Clone, Copy, Eq, PartialEq)]
    #[repr(usize)]
    enum NonzeroTag {
        One = 1,
        Three = 3,
    }

    // SAFETY: Both logical values fit the two-bit domain and round-trip through `TagField`.
    unsafe impl Tag for NonzeroTag {
        type Type = TagField;
        type Value = Self;

        const MASK: usize = TestTagField::mask();
    }

    // SAFETY: Conversion accepts exactly the two assigned nonzero discriminants.
    unsafe impl Field for NonzeroTag {
        fn value(self) -> usize {
            self as usize
        }

        fn from_value(target_value: usize) -> Option<Self> {
            match target_value {
                target_value if target_value == Self::One as usize => Some(Self::One),
                target_value if target_value == Self::Three as usize => Some(Self::Three),
                _ => None,
            }
        }
    }

    /// A numeric domain whose decoder deliberately normalizes spare bits.
    #[derive(Clone, Copy)]
    enum AliasTag {}

    // SAFETY: Every `Masked<1>` value encodes within the mask and round-trips.
    // Accepting additional raw patterns does not change that encoding contract.
    unsafe impl Tag for AliasTag {
        type Type = super::TagMasked;
        type Value = Masked<1>;

        const MASK: usize = 1;
    }

    impl Encoding<AliasTag> for super::TagMasked {
        #[inline]
        fn encode(target_value: Masked<1>) -> Option<usize> {
            Some(target_value.unwrap())
        }

        #[inline]
        fn decode(encoded: usize) -> Option<Masked<1>> {
            Masked::new(encoded & AliasTag::MASK)
        }
    }

    /// Projection validation does not rely on a decoder rejecting spare bits.
    #[test]
    fn projection_rejects_spare_bits_before_decoding() {
        let mut image = 0b010;

        assert!(<super::TagMasked as Encoding<AliasTag>>::decode(image).is_some());
        assert!(Tagged::<0, 2, NonNull<AlignedBytes>, AliasTag>::wrap(&image).is_none());
        assert!(TaggedMut::<0, 2, NonNull<AlignedBytes>, AliasTag>::wrap(&mut image).is_none());
        assert_eq!(image, 0b010);
    }

    /// Positioned extraction and mutation preserve address bits on both sides.
    #[test]
    fn positioned_projection_preserves_other_bits() {
        let mut image = 0xa58b;

        assert_eq!(Positioned::<TestTag>::mask(), 0x70);
        assert_eq!(
            Positioned::<TestTag>::wrap(&image).expect("zero tag").value(),
            TestTag::Zero
        );

        let mut view = PositionedMut::<TestTag>::new(&mut image, TestTag::Three);

        assert_eq!(view.value(), TestTag::Three);
        assert_eq!(view.merge(TestTag::One), TestTag::Three);
        assert_eq!(view.value(), TestTag::One);
        assert_eq!(view.clear(), TestTag::One);
        assert_eq!(image, 0xa58b);

        let _view = PositionedMut::<TestTag>::new(&mut image, TestTag::Three);

        assert_eq!(image, 0xa5bb);
    }

    /// Checked wrapping rejects spare field bits without changing the image.
    #[test]
    fn projection_rejects_unassigned_and_spare_bits() {
        for mut image in [0xa5ab, 0xa5cb] {
            let original = image;

            assert!(Positioned::<TestTag>::wrap(&image).is_none());
            assert!(PositionedMut::<TestTag>::wrap(&mut image).is_none());
            assert_eq!(image, original);

            let view = PositionedMut::<TestTag>::new(&mut image, TestTag::One);

            assert_eq!(view.value(), TestTag::One);
            assert_eq!(image, 0xa59b);
        }
    }

    /// Sparse tags reject holes and initialization replaces the entire field.
    #[test]
    fn sparse_projection_preserves_assigned_set() {
        let mut image = 0xa5ab;

        assert!(Positioned::<TestBit>::wrap(&image).is_none());
        assert!(PositionedMut::<TestBit>::wrap(&mut image).is_none());

        let mut set = BitfieldSet::<TestBit>::single(TestBit::First).expect("assigned bit");

        assert!(set.insert(TestBit::Third));

        let view = PositionedMut::<TestBit>::new(&mut image, set);

        assert_eq!(view.value(), set);
        assert_eq!(image, 0xa5db);
        assert_eq!(
            PositionedMut::<TestBit>::wrap(&mut image)
                .expect("assigned set")
                .clear(),
            set
        );
        assert_eq!(image, 0xa58b);
    }

    /// Consuming a projection permits clearing even when zero is unassigned.
    #[test]
    fn clear_consumes_nonzero_tag_projection() {
        let mut image = 0xa58b;

        assert!(Positioned::<NonzeroTag>::wrap(&image).is_none());

        let mut view = PositionedMut::<NonzeroTag>::new(&mut image, NonzeroTag::One);

        assert_eq!(view.merge(NonzeroTag::Three), NonzeroTag::One);
        assert_eq!(view.clear(), NonzeroTag::Three);
        assert_eq!(image, 0xa58b);
        assert!(PositionedMut::<NonzeroTag>::wrap(&mut image).is_none());
    }

    /// Numeric tags retain their checked domain through field operations.
    #[test]
    fn masked_projection_checks_numeric_domain() {
        let mut image = 0xa5ab;
        let tag = Masked::<0b101>::new(0b101).expect("numeric tag fits");

        assert!(Positioned::<Masked<0b101>>::wrap(&image).is_none());

        let mut view = PositionedMut::<Masked<0b101>>::new(&mut image, tag);
        let zero = Masked::new(0).expect("zero fits");

        assert_eq!(view.value(), tag);
        assert_eq!(view.merge(zero), tag);
        assert_eq!(view.clear(), zero);
        assert_eq!(image, 0xa58b);
    }

    /// Field-output conversion also covers tags wider than `u16`.
    #[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
    #[test]
    fn projection_supports_wider_output() {
        const MASK: usize = multi::Field::<0, 16, usize>::mask();

        let mut image = 0x8000_0000;
        let tag = Masked::<MASK>::new(0x1_2345).expect("wide tag fits");
        let view = TaggedMut::<0, 16, NonNull<WideImage>, Masked<MASK>>::new(&mut image, tag);

        assert_eq!(view.value(), tag);
        assert_eq!(view.clear(), tag);
        assert_eq!(image, 0x8000_0000);
    }

    /// The projections add no storage beyond their borrowed field.
    #[test]
    fn projection_layout_matches_field() {
        assert_eq!(mem::size_of::<Positioned<'_, TestTag>>(), mem::size_of::<&usize>());
        assert_eq!(
            mem::size_of::<PositionedMut<'_, TestTag>>(),
            mem::size_of::<&mut usize>()
        );
        assert_eq!(mem::align_of::<Positioned<'_, TestTag>>(), mem::align_of::<&usize>());
    }

    /// Address-image projection leaves provenance with the mapped pointer.
    #[test]
    fn projection_round_trips_pointer_provenance() {
        let mut payload = AlignedImage(7);
        let pointer = NonNull::from(&mut payload);
        let packed = pointer.as_ptr().map_addr(|mut address| {
            let _view = PositionedMut::<TestTag>::new(&mut address, TestTag::Three);

            address
        });
        let recovered = packed.map_addr(|mut address| {
            let view = PositionedMut::<TestTag>::wrap(&mut address).expect("packed tag");

            assert_eq!(view.clear(), TestTag::Three);

            address
        });

        assert_eq!(recovered, pointer.as_ptr());

        // SAFETY: Address mapping preserves the original mutable pointer's provenance,
        // and clearing the field recovers its live, aligned allocation address.
        unsafe { recovered.write(AlignedImage(42)) };

        let AlignedImage(value) = payload;

        assert_eq!(value, 42);
    }

    #[test]
    fn masked_value_checks_bits() {
        assert_eq!(Masked::<0b11>::new(0b10).map(Masked::unwrap), Some(0b10));
        assert!(Masked::<0b11>::new(0b100).is_none());
        assert_eq!(Masked::<0b11>::mask(), 0b11);
    }

    #[test]
    fn field_encoding_rejects_unassigned_pattern() {
        assert_eq!(<TagField as Encoding<TestTag>>::decode(0b10), None);
        assert_eq!(<TagField as Encoding<TestTag>>::decode(0b100), None);
        assert_eq!(<TagField as Encoding<TestTag>>::decode(usize::MAX), None);
    }

    #[test]
    fn field_encoding_round_trips_assigned_values() {
        for target_tag in [TestTag::Zero, TestTag::One, TestTag::Three] {
            let encoded =
                <TagField as Encoding<TestTag>>::encode(target_tag).expect("assigned field value is encodable");

            assert_eq!(encoded, target_tag.value());
            assert_eq!(<TagField as Encoding<TestTag>>::decode(encoded), Some(target_tag));
        }
    }

    #[test]
    fn bitfield_variants_encode_as_bit_indices() {
        let target_set = BitfieldSet::<TestBit>::single(TestBit::Third);

        assert!(target_set.is_some());

        let Some(mut target_set) = target_set else {
            return;
        };

        assert_eq!(target_set.bits(), 0b100);
        assert!(target_set.contains(TestBit::Third));
        assert!(!target_set.contains(TestBit::First));
        assert!(target_set.insert(TestBit::First));
        assert!(!target_set.insert(TestBit::First));
        assert_eq!(target_set.bits(), 0b101);
        assert!(target_set.remove(TestBit::Third));
        assert!(!target_set.remove(TestBit::Third));
        assert_eq!(target_set.bits(), 0b001);
    }

    #[test]
    fn bitfield_set_rejects_unassigned_bits() {
        assert!(BitfieldSet::<TestBit>::from_bits(0b010).is_none());
        assert!(BitfieldSet::<TestBit>::from_bits(0b111).is_none());
        assert_eq!(
            BitfieldSet::<TestBit>::from_bits(0b101).map(BitfieldSet::bits),
            Some(0b101)
        );
    }

    #[test]
    fn maskable_uses_pointee_alignment() {
        assert_eq!(<*const AlignedBytes as Maskable>::MASK, 0b111);
        assert_eq!(<*mut AlignedBytes as Maskable>::MASK, 0b111);
        assert_eq!(<NonNull<AlignedBytes> as Maskable>::MASK, 0b111);
        assert_eq!(<NonNull<u8> as Maskable>::MASK, usize::MIN);
        assert_eq!(<NonNull<()> as Maskable>::MASK, usize::MIN);
    }
}
