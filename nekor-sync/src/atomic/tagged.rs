//! Strict-provenance tagged atomic pointers.
//!
//! This module separates three concepts.
//!
//! - [`Maskable`] describes tag bits guaranteed by a pointee's alignment.
//! - [`TaggedPointer`] combines one non-null pointer with one validated logical
//!   value from a [`Tag`] domain.
//! - [`AtomicTaggedPointer`] stores either null or one tagged pointer
//!   atomically.
//!
//! Tags occupy address bits that are zero in the untagged pointer. Pointer
//! encoding and decoding use strict-provenance address mapping rather than an
//! integer-to-pointer reconstruction.
//!
//! This module does not confer pointee ownership, extend pointee lifetime, or
//! solve pointer ABA. Those properties remain responsibilities of the semantic
//! abstraction built on top of the tagged pointer.

use core::{
    fmt, marker, mem,
    ptr::{self, NonNull},
    sync::atomic::{AtomicPtr, Ordering},
};

use nekor_bitwise::{bit::BitOp, field::dynamic::select::Selected};

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
// NOTE(invariant): The contained value always satisfies `value & M == value`.
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
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

impl private::Sealed for TagBitfield {}

impl private::Sealed for TagField {}

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
/// - `MASK` identifies every address bit used by this tag domain and no other
///   address bit.
/// - `Type` selects the semantic category implemented by this domain.
/// - `Value` is the logical value required by the selected category.
/// - The mask and category remain context-independent for the lifetime of every
///   tagged pointer using this domain.
pub unsafe trait Tag: Copy {
    /// The address-bit mask reserved by this tag domain.
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
/// A field occupies a contiguous range beginning at pointer bit zero. One enum
/// inhabitant describes the complete field rather than one independently
/// combinable bit.
///
/// # Safety
///
/// Implementors must uphold all of the following conditions.
///
/// - The [`Tag`] implementation uses [`TagField`] and `Self` as its value.
/// - [`Tag::MASK`] is a contiguous low-bit mask.
/// - [`Field::value`] never sets a bit outside [`Tag::MASK`].
/// - [`Field::from_value`] rejects values with bits outside [`Tag::MASK`] and
///   every unassigned field pattern.
/// - Converting an inhabitant to its value and back returns the same
///   inhabitant.
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
// NOTE(invariant): `bits` contains no bit outside `B::MASK`. Every set bit has
// an index accepted by `B::from_index`. Safe construction and mutation validate
// both conditions before changing the representation.
#[repr(transparent)]
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
        let Self { bits, .. } = self;
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
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { bits, .. } = self;

        formatter
            .debug_struct("BitfieldSet")
            .field("bits", bits)
            .finish()
    }
}

impl<B> PartialEq for BitfieldSet<B> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        let Self { bits: left, .. } = self;
        let Self { bits: right, .. } = other;

        left == right
    }
}

impl<B> Eq for BitfieldSet<B> {}

/// The sealed conversion policy for one pointer-tag category.
///
/// This trait connects shared tagged-pointer storage to the deliberately
/// distinct [`Field`] and [`Bitfield`] conversion contracts. Only
/// [`TagField`] and [`TagBitfield`] implement it.
pub trait TagEncoding<T>: private::Sealed
where
    T: Tag,
{
    /// Encodes one complete logical tag value.
    fn encode(target_value: T::Value) -> Option<usize>;

    /// Attempts to decode one complete logical tag value.
    fn decode(encoded: usize) -> Option<T::Value>;
}

impl<T> TagEncoding<T> for TagField
where
    T: Field,
{
    #[inline]
    fn encode(target_value: T::Value) -> Option<usize> {
        let encoded = T::value(target_value);

        if T::MASK & T::MASK.wrapping_add(1) != usize::MIN
            || encoded & !T::MASK != usize::MIN
            || T::from_value(encoded).is_none()
        {
            None
        } else {
            Some(encoded)
        }
    }

    #[inline]
    fn decode(encoded: usize) -> Option<T::Value> {
        if T::MASK & T::MASK.wrapping_add(1) != usize::MIN || encoded & !T::MASK != usize::MIN {
            None
        } else {
            T::from_value(encoded)
        }
    }
}

impl<T> TagEncoding<T> for TagBitfield
where
    T: Bitfield,
{
    #[inline]
    fn encode(target_value: T::Value) -> Option<usize> {
        let encoded = BitfieldSet::<T>::bits(target_value);

        BitfieldSet::<T>::from_bits(encoded).map(|_| encoded)
    }

    #[inline]
    fn decode(encoded: usize) -> Option<T::Value> {
        BitfieldSet::<T>::from_bits(encoded)
    }
}

/// A non-null pointer carrying one validated logical tag value in its address
/// bits.
///
/// The pointer is never dereferenced while tagged. Use
/// [`TaggedPointer::pointer`] or [`TaggedPointer::split`] to recover its
/// untagged address first.
// NOTE(invariant): `tagged` is non-null. Clearing `T::MASK` yields the original
// non-null pointer with the same provenance. The masked bits decode as
// `T::Value` through `T::Type`.
#[repr(transparent)]
pub struct TaggedPointer<P, T>
where
    T: Tag,
{
    /// The provenance-carrying pointer with its tag bits applied.
    tagged: NonNull<P>,

    /// The encoded tag type.
    marker: marker::PhantomData<T>,
}

impl<P, T> TaggedPointer<P, T>
where
    T: Tag,
    T::Type: TagEncoding<T>,
{
    /// Attempts to attach `target_tag` to `target_pointer`.
    ///
    /// This checks the actual pointer address, which permits tagging an erased
    /// pointer whose static pointee alignment no longer communicates the
    /// alignment of its underlying allocation.
    #[inline]
    #[must_use]
    pub fn new(target_pointer: NonNull<P>, target_tag: T::Value) -> Option<Self> {
        let encoded_tag = <T::Type as TagEncoding<T>>::encode(target_tag)?;
        let pointer_address = target_pointer.as_ptr().addr();

        if pointer_address & T::MASK != usize::MIN {
            return None;
        }

        let tagged_pointer = target_pointer
            .as_ptr()
            .map_addr(|target_address| target_address | encoded_tag);
        let tagged = NonNull::new(tagged_pointer)?;
        let marker = marker::PhantomData;

        Some(Self { tagged, marker })
    }

    /// Determines whether the static pointee alignment supports this tag.
    ///
    /// A `false` result does not prohibit tagging a particular erased pointer
    /// through [`TaggedPointer::new`]. It means only that the tag requires more
    /// bits than the static pointee alignment makes available.
    #[inline]
    #[must_use]
    pub const fn pointee_supports_tag() -> bool
    where
        NonNull<P>: Maskable,
    {
        T::MASK & !<NonNull<P> as Maskable>::MASK == 0
    }

    /// Returns the untagged pointer while preserving its provenance.
    #[inline]
    #[must_use]
    pub fn pointer(self) -> NonNull<P> {
        let Self { tagged, .. } = self;
        let target_pointer = tagged
            .as_ptr()
            .map_addr(|target_address| target_address & !T::MASK);

        // SAFETY: The representation invariant guarantees that clearing the tag
        // recovers the original non-null pointer.
        unsafe { NonNull::new_unchecked(target_pointer) }
    }

    /// Attempts to decode the stored logical tag value.
    #[inline]
    #[must_use]
    pub fn tag(self) -> Option<T::Value> {
        let Self { tagged, .. } = self;
        let encoded_tag = tagged.as_ptr().addr() & T::MASK;

        <T::Type as TagEncoding<T>>::decode(encoded_tag)
    }

    /// Recovers the untagged pointer and decoded logical tag value.
    ///
    /// Returns [`None`] only when an unsafe tag-domain implementation violated
    /// its decoding contract or unsafe code forged this type's representation.
    #[inline]
    #[must_use]
    pub fn split(self) -> Option<(NonNull<P>, T::Value)> {
        self.tag().map(|target_tag| (self.pointer(), target_tag))
    }

    /// Attempts to replace the stored logical tag value without changing the
    /// pointer.
    #[inline]
    #[must_use]
    pub fn retag(self, target_tag: T::Value) -> Option<Self> {
        Self::new(self.pointer(), target_tag)
    }

    /// Returns the tagged pointer for atomic storage and comparison.
    #[inline]
    const fn as_tagged_ptr(self) -> *mut P {
        let Self { tagged, .. } = self;

        tagged.as_ptr()
    }

    /// Reconstructs a tagged pointer read from a trusted atomic container.
    ///
    /// # Safety
    ///
    /// `tagged` must have been produced by a valid `TaggedPointer<P, T>` with
    /// the same `P`, `T`, and tag-category implementation.
    #[inline]
    const unsafe fn from_tagged(tagged: NonNull<P>) -> Self {
        let marker = marker::PhantomData;

        Self { tagged, marker }
    }
}

impl<P, T> Clone for TaggedPointer<P, T>
where
    T: Tag,
{
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<P, T> Copy for TaggedPointer<P, T> where T: Tag {}

impl<P, T> fmt::Debug for TaggedPointer<P, T>
where
    T: Tag,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TaggedPointer")
            .finish_non_exhaustive()
    }
}

impl<P, T> PartialEq for TaggedPointer<P, T>
where
    T: Tag,
{
    fn eq(&self, other: &Self) -> bool {
        let Self { tagged: left, .. } = self;
        let Self { tagged: right, .. } = other;

        ptr::eq(left.as_ptr(), right.as_ptr())
    }
}

impl<P, T> Eq for TaggedPointer<P, T> where T: Tag {}

/// The result of a tagged-pointer compare-and-exchange operation.
///
/// Both the success and failure cases contain the optional value observed
/// before the attempted exchange.
pub type CompareExchangeResult<P, T> =
    Result<Option<TaggedPointer<P, T>>, Option<TaggedPointer<P, T>>>;

/// A nullable atomic [`TaggedPointer`].
///
/// Null represents the absence of a pointer. Every non-null value is stored in
/// the representation produced by `TaggedPointer<P, T>`.
///
/// This type delegates progress and ordering properties to [`AtomicPtr`]. It
/// does not make platform atomics wait-free and does not prevent pointer ABA.
// NOTE(invariant): `pointer` is either null or a tagged pointer produced with
// the same `P` and `T`. All non-null writes pass through methods accepting
// `TaggedPointer<P, T>`.
#[repr(transparent)]
pub struct AtomicTaggedPointer<P, T>
where
    T: Tag,
{
    /// The null or tagged pointer representation.
    pointer: AtomicPtr<P>,

    /// The encoded tag type.
    marker: marker::PhantomData<T>,
}

impl<P, T> AtomicTaggedPointer<P, T>
where
    T: Tag,
    T::Type: TagEncoding<T>,
{
    /// Constructs an atomic pointer initialized to null.
    #[inline]
    #[must_use]
    pub const fn null() -> Self {
        let pointer = AtomicPtr::new(ptr::null_mut());
        let marker = marker::PhantomData;

        Self { pointer, marker }
    }

    /// Constructs an atomic pointer initialized with an optional tagged value.
    #[inline]
    #[must_use]
    pub fn new(target_pointer: Option<TaggedPointer<P, T>>) -> Self {
        let pointer = AtomicPtr::new(Self::encode(target_pointer));
        let marker = marker::PhantomData;

        Self { pointer, marker }
    }

    /// Loads the current optional tagged pointer.
    #[inline]
    #[must_use]
    pub fn load(&self, target_order: Ordering) -> Option<TaggedPointer<P, T>> {
        let Self { pointer, .. } = self;

        Self::decode(pointer.load(target_order))
    }

    /// Stores an optional tagged pointer.
    #[inline]
    pub fn store(&self, target_pointer: Option<TaggedPointer<P, T>>, target_order: Ordering) {
        let Self { pointer, .. } = self;

        pointer.store(Self::encode(target_pointer), target_order);
    }

    /// Exchanges the current optional tagged pointer.
    #[inline]
    pub fn swap(
        &self,
        target_pointer: Option<TaggedPointer<P, T>>,
        target_order: Ordering,
    ) -> Option<TaggedPointer<P, T>> {
        let Self { pointer, .. } = self;
        let previous_pointer = pointer.swap(Self::encode(target_pointer), target_order);

        Self::decode(previous_pointer)
    }

    /// Compares and exchanges an optional tagged pointer.
    ///
    /// # Errors
    ///
    /// Returns the observed optional value when it differs from `current`.
    #[inline]
    pub fn compare_exchange(
        &self,
        current: Option<TaggedPointer<P, T>>,
        new: Option<TaggedPointer<P, T>>,
        success: Ordering,
        failure: Ordering,
    ) -> CompareExchangeResult<P, T> {
        let Self { pointer, .. } = self;

        match pointer.compare_exchange(Self::encode(current), Self::encode(new), success, failure) {
            Ok(previous) => Ok(Self::decode(previous)),
            Err(observed) => Err(Self::decode(observed)),
        }
    }

    /// Weakly compares and exchanges an optional tagged pointer.
    ///
    /// # Errors
    ///
    /// Returns the observed optional value when the operation fails or differs
    /// from `current`. The operation may fail spuriously.
    #[inline]
    pub fn compare_exchange_weak(
        &self,
        current: Option<TaggedPointer<P, T>>,
        new: Option<TaggedPointer<P, T>>,
        success: Ordering,
        failure: Ordering,
    ) -> CompareExchangeResult<P, T> {
        let Self { pointer, .. } = self;

        match pointer.compare_exchange_weak(
            Self::encode(current),
            Self::encode(new),
            success,
            failure,
        ) {
            Ok(previous) => Ok(Self::decode(previous)),
            Err(observed) => Err(Self::decode(observed)),
        }
    }

    /// Determines whether the current atomic value is null.
    #[inline]
    #[must_use]
    pub fn is_null(&self, target_order: Ordering) -> bool {
        let Self { pointer, .. } = self;

        pointer.load(target_order).is_null()
    }

    /// Encodes an optional tagged pointer as a nullable raw pointer.
    #[inline]
    fn encode(target_pointer: Option<TaggedPointer<P, T>>) -> *mut P {
        target_pointer.map_or(ptr::null_mut(), TaggedPointer::as_tagged_ptr)
    }

    /// Decodes a nullable raw pointer from this trusted atomic container.
    #[inline]
    fn decode(target_pointer: *mut P) -> Option<TaggedPointer<P, T>> {
        NonNull::new(target_pointer).map(|tagged_pointer| {
            // SAFETY: The atomic representation invariant guarantees that every
            // non-null value was written from `TaggedPointer<P, T>`.
            unsafe { TaggedPointer::from_tagged(tagged_pointer) }
        })
    }
}

impl<P, T> Default for AtomicTaggedPointer<P, T>
where
    T: Tag,
    T::Type: TagEncoding<T>,
{
    #[inline]
    fn default() -> Self {
        Self::null()
    }
}

impl<P, T> fmt::Debug for AtomicTaggedPointer<P, T>
where
    T: Tag,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AtomicTaggedPointer")
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AtomicTaggedPointer, Bitfield, BitfieldSet, Field, Maskable, Masked, Tag, TagBitfield,
        TagEncoding, TagField, TaggedPointer,
    };

    use core::{ptr::NonNull, sync::atomic::Ordering};

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
        const MASK: usize = 0b11;

        type Type = TagField;

        type Value = Self;
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
        const MASK: usize = 0b101;

        type Type = TagBitfield;

        type Value = BitfieldSet<Self>;
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
    struct AlignedBytes([u8; 4]);

    #[test]
    fn masked_value_checks_bits() {
        assert_eq!(Masked::<0b11>::new(0b10).map(Masked::unwrap), Some(0b10));
        assert!(Masked::<0b11>::new(0b100).is_none());
        assert_eq!(Masked::<0b11>::mask(), 0b11);
    }

    #[test]
    fn field_encoding_rejects_unassigned_pattern() {
        assert_eq!(<TagField as TagEncoding<TestTag>>::decode(0b10), None);
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
        assert!(TaggedPointer::<AlignedBytes, TestTag>::pointee_supports_tag());
        assert!(!TaggedPointer::<u8, TestTag>::pointee_supports_tag());
    }

    #[test]
    fn tagged_pointer_round_trips_pointer_and_tag() {
        let mut target_value = AlignedBytes([0; 4]);
        let target_pointer = NonNull::from_mut(&mut target_value);
        let tagged_pointer =
            TaggedPointer::<AlignedBytes, TestTag>::new(target_pointer, TestTag::Three);

        assert!(tagged_pointer.is_some());

        let Some(tagged_pointer) = tagged_pointer else {
            return;
        };
        let split = tagged_pointer.split();

        assert!(split.is_some());

        let Some((decoded_pointer, decoded_tag)) = split else {
            return;
        };

        assert_eq!(decoded_pointer, target_pointer);
        assert_eq!(decoded_tag, TestTag::Three);
    }

    #[test]
    fn tagged_pointer_round_trips_bitfield_set() {
        let mut target_value = AlignedBytes([0; 4]);
        let target_pointer = NonNull::from_mut(&mut target_value);
        let mut target_set = BitfieldSet::<TestBit>::empty();

        assert!(target_set.insert(TestBit::First));
        assert!(target_set.insert(TestBit::Third));

        let tagged_pointer =
            TaggedPointer::<AlignedBytes, TestBit>::new(target_pointer, target_set);

        assert!(tagged_pointer.is_some());

        let Some(tagged_pointer) = tagged_pointer else {
            return;
        };
        let split = tagged_pointer.split();

        assert_eq!(split, Some((target_pointer, target_set)));
    }

    #[test]
    fn erased_pointer_uses_actual_address_alignment() {
        let mut target_value = AlignedBytes([0; 4]);
        let erased_pointer = NonNull::from_mut(&mut target_value).cast::<()>();
        let tagged_pointer = TaggedPointer::<(), TestTag>::new(erased_pointer, TestTag::One);

        assert!(tagged_pointer.is_some());
        assert!(!TaggedPointer::<(), TestTag>::pointee_supports_tag());
    }

    #[test]
    fn insufficiently_aligned_address_is_rejected() {
        let mut target_value = AlignedBytes([0; 4]);
        let base_pointer = NonNull::from_mut(&mut target_value).cast::<u8>();

        // SAFETY: The four-byte array contains the requested one-byte offset.
        let offset_pointer = unsafe { base_pointer.as_ptr().add(1) };
        let Some(offset_pointer) = NonNull::new(offset_pointer) else {
            return;
        };

        assert!(TaggedPointer::<u8, TestTag>::new(offset_pointer, TestTag::One).is_none());
    }

    #[test]
    fn retagging_preserves_pointer() {
        let mut target_value = AlignedBytes([0; 4]);
        let target_pointer = NonNull::from_mut(&mut target_value);
        let original = TaggedPointer::<AlignedBytes, TestTag>::new(target_pointer, TestTag::One);

        assert!(original.is_some());

        let Some(original) = original else {
            return;
        };
        let retagged = original.retag(TestTag::Three);

        assert!(retagged.is_some());

        let Some(retagged) = retagged else {
            return;
        };

        assert_eq!(retagged.pointer(), target_pointer);
        assert_eq!(retagged.tag(), Some(TestTag::Three));
    }

    #[test]
    fn atomic_tagged_pointer_supports_null_and_swap() {
        let mut target_value = AlignedBytes([0; 4]);
        let target_pointer = NonNull::from_mut(&mut target_value);
        let tagged_pointer =
            TaggedPointer::<AlignedBytes, TestTag>::new(target_pointer, TestTag::One);

        assert!(tagged_pointer.is_some());

        let Some(tagged_pointer) = tagged_pointer else {
            return;
        };
        let target_atomic = AtomicTaggedPointer::null();

        assert!(target_atomic.is_null(Ordering::SeqCst));
        assert_eq!(
            target_atomic.swap(Some(tagged_pointer), Ordering::SeqCst),
            None
        );
        assert_eq!(target_atomic.load(Ordering::SeqCst), Some(tagged_pointer));
        assert_eq!(
            target_atomic.swap(None, Ordering::SeqCst),
            Some(tagged_pointer)
        );
        assert!(target_atomic.is_null(Ordering::SeqCst));
    }

    #[test]
    fn atomic_tagged_pointer_compares_complete_representation() {
        let mut first_value = AlignedBytes([0; 4]);
        let mut second_value = AlignedBytes([0; 4]);
        let first = TaggedPointer::<AlignedBytes, TestTag>::new(
            NonNull::from_mut(&mut first_value),
            TestTag::One,
        );
        let second = TaggedPointer::<AlignedBytes, TestTag>::new(
            NonNull::from_mut(&mut second_value),
            TestTag::Three,
        );

        assert!(first.is_some());
        assert!(second.is_some());

        let Some(first) = first else {
            return;
        };
        let Some(second) = second else {
            return;
        };
        let target_atomic = AtomicTaggedPointer::new(Some(first));
        let failed =
            target_atomic.compare_exchange(Some(second), None, Ordering::SeqCst, Ordering::SeqCst);

        assert_eq!(failed, Err(Some(first)));
        assert_eq!(
            target_atomic.compare_exchange(
                Some(first),
                Some(second),
                Ordering::SeqCst,
                Ordering::SeqCst,
            ),
            Ok(Some(first))
        );
        assert_eq!(target_atomic.load(Ordering::SeqCst), Some(second));
    }
}
