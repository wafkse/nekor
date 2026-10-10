//! Pointer tagging.
//!
//! An aligned pointer has low address bits that are always zero. For example,
//! eight-byte alignment leaves the lowest three bits available for a tag,
//! provided that they are cleared before the pointer is used to access memory.
//!
//! [`TaggedPtr`] stores such a tag alongside a pointer, without increasing its
//! size. [`Tag`] describes the reserved bits, while [`Bijection`] determines
//! which nonzero patterns represent tag values. [`Bits`] supplies a numeric tag
//! when no separate logical type is needed.
//!
//! The [`Taggable`] marker guarantees that the pointee type has sufficient alignment.
//! Use [`taggable!`] to check this at compile time. An arbitrary raw pointer may
//! still be misaligned, so its address must be checked or covered by the caller's
//! safety guarantee when constructing a [`TaggedPtr`].
//!
//! This module currently supports thin raw pointers and [`NonNull`]. Tagging
//! retains pointer provenance, but does not establish a borrow, allocation
//! ownership, or permission to dereference the recovered pointer.

use core::{marker::PhantomData, mem, num::NonZero, ptr::NonNull};

use crate::{
    field::multi::Field,
    prelude::{Counterpart, For, Size},
};

mod detail {
    //! Limits pointer representations to the built-in thin pointer types.

    /// A supertrait implemented by the supported pointer representations.
    pub trait Sealed {}
}

/// A thin pointer whose address can be changed without discarding its provenance.
///
/// This trait is implemented for `*const V`, `*mut V`, and [`NonNull<V>`](NonNull).
/// Address mapping preserves the pointer type, including its mutability and
/// nullability restrictions.
///
/// [`Pointer::ALIGN`] describes the alignment required by the pointee type. It
/// does not guarantee that an arbitrary raw pointer satisfies that alignment,
/// nor that the pointer can be dereferenced.
pub trait Pointer: detail::Sealed + Copy {
    /// The sized pointee whose alignment determines the available tag bits.
    type Target;

    /// The pointee's alignment requirement in bytes.
    const ALIGN: usize = mem::align_of::<Self::Target>();

    /// Read the address component without exposing provenance.
    fn address(target_value: Self) -> usize;

    /// Map the pointer to an address while retaining its provenance.
    ///
    /// This changes only the address component. It does not establish that the
    /// resulting pointer is aligned or points into an allocation.
    ///
    /// # Failure
    ///
    /// Returns `None` when `Self` is [`NonNull`] and `target_address` is zero.
    /// Raw pointer implementations accept every address, including zero.
    fn map(target_value: Self, target_address: usize) -> Option<Self>;
}

impl<V> detail::Sealed for *const V {}

impl<V> Pointer for *const V {
    type Target = V;

    #[inline]
    fn address(target_value: Self) -> usize {
        target_value.addr()
    }

    #[inline]
    fn map(target_value: Self, target_address: usize) -> Option<Self> {
        Some(target_value.with_addr(target_address))
    }
}

impl<V> detail::Sealed for *mut V {}

impl<V> Pointer for *mut V {
    type Target = V;

    #[inline]
    fn address(target_value: Self) -> usize {
        target_value.addr()
    }

    #[inline]
    fn map(target_value: Self, target_address: usize) -> Option<Self> {
        Some(target_value.with_addr(target_address))
    }
}

impl<V> detail::Sealed for NonNull<V> {}

impl<V> Pointer for NonNull<V> {
    type Target = V;

    #[inline]
    fn address(target_value: Self) -> usize {
        target_value.addr().get()
    }

    #[inline]
    fn map(target_value: Self, target_address: usize) -> Option<Self> {
        NonNull::new(target_value.as_ptr().with_addr(target_address))
    }
}

/// A bijection between tag values and their assigned nonzero integer patterns.
///
/// # Assigned Patterns
///
/// Not every nonzero integer needs to represent a tag. An enum, for example,
/// may assign only `1` and `5`, leaving the other patterns unassigned.
/// [`Bijection::decode()`] rejects those patterns rather than requiring dummy
/// variants to represent them.
///
/// The encoded pattern need not equal an enum's discriminant or a newtype's
/// underlying value. In particular, a logical zero is permitted as long as its
/// encoded pattern is nonzero. Identity mappings are also permitted when the
/// tag values are already nonzero.
///
/// # Safety
///
/// Implementations must satisfy both directions of the bijection.
///
/// - Encoding a valid tag and then decoding it must return that same tag.
/// - If decoding an integer succeeds, encoding the returned tag must produce that exact integer.
///   Distinct accepted patterns must not alias the same tag.
///
/// These conversions must not depend on mutable state or the calling context.
/// [`TaggedPtr`] relies on a stored pattern remaining assigned after construction.
pub unsafe trait Bijection: Copy {
    /// Encode a logical tag as its assigned nonzero code.
    fn encode(target_value: Self) -> NonZero<usize>;

    /// Decode an assigned nonzero pattern.
    ///
    /// Returns `None` when the pattern has no assigned tag value.
    fn decode(target_value: NonZero<usize>) -> Option<Self>;
}

/// A tag whose encoded value occupies a statically selected low address field.
///
/// [`Bijection`] describes the tag values. This trait describes where their
/// encoded patterns are stored. A concrete [`Field`] and its
/// [`Counterpart::Mut`] can supply the corresponding field operations.
///
/// # Field Operations
///
/// [`Tag::read()`] and [`Tag::write()`] operate on raw integers, not logical tags.
/// They therefore accept zero and unassigned patterns independently of
/// [`Bijection::decode()`].
///
/// In other words, writing zero clears the reserved bits without constructing
/// a zero-valued tag. [`TaggedPtr::pointer()`] uses this to recover the original
/// pointer even when zero is not a valid tag value.
///
/// # Safety
///
/// Implementations must ensure that all of the following are satisfied.
///
/// - `MASK` identifies a nonempty contiguous field beginning at bit zero.
/// - Every pattern emitted or accepted by the [`Bijection`] implementation fits within `MASK`.
/// - `read` extracts exactly the reserved bits as a low-aligned integer.
/// - `write` replaces the reserved bits with the low field bits of `target_code`, including zero,
///   and preserves every bit outside `MASK`. Excess input bits are truncated, as with [`Field`].
/// - The field layout and operations do not depend on mutable state or the calling context.
pub unsafe trait Tag: Bijection {
    /// The mask identifying the reserved low address bits.
    const MASK: usize;

    /// Extract the reserved bits without decoding a tag.
    fn read(target_value: usize) -> usize;

    /// Replace the reserved bits while preserving the other address bits.
    ///
    /// Zero clears the field. Input bits exceeding the field width are discarded.
    fn write(target_value: usize, target_code: usize) -> usize;
}

/// A pointee type whose alignment leaves sufficient address bits for `T`.
///
/// This marker applies to the pointee rather than a particular pointer type.
/// Its guarantee is shared by `*const Self`, `*mut Self`, and [`NonNull<Self>`](NonNull).
/// Use [`taggable!`] to check the alignment and implement the marker.
///
/// # Type Alignment
///
/// The marker guarantees that `T` fits into the bits left unused by the type's
/// alignment. It does not check individual raw pointer addresses, which may
/// still be misaligned even when their pointee implements [`Taggable`].
///
/// [`TaggedPtr`] requires this marker in its type bounds. A pointer/tag pairing
/// without it is rejected even when no constructor is called.
///
/// ```compile_fail
/// use nekor_bitwise::pointer::{Bits, TaggedPtr};
///
/// let _ = core::mem::size_of::<TaggedPtr<*const u8, Bits<3>>>();
/// ```
///
/// # Safety
///
/// `T::MASK` must identify a nonempty contiguous low field contained entirely
/// within `align_of::<Self>() - 1`. Those bits must therefore be zero in every
/// address satisfying the pointee type's alignment.
pub unsafe trait Taggable<T: Tag>: Sized {}

/// Check that a pointee type can carry a tag, then implement [`Taggable`].
///
/// The declaration checks that the tag reserves a nonempty contiguous low
/// field and that the pointee's alignment leaves all of those bits unused.
/// These checks take place during const evaluation, not pointer construction.
/// They do not check individual raw pointer addresses or the behavior of the
/// tag's [`Bijection`] and [`Tag`] implementations.
///
/// # Declarations
///
/// ```
/// use nekor_bitwise::pointer::{Bits, TaggedPtr, taggable};
///
/// #[repr(align(8))]
/// struct Aligned;
///
/// taggable! {
///     unsafe impl Taggable<Bits<3>> for Aligned;
/// }
///
/// let tag = Bits::<3>::checked(3).expect("nonzero three-bit code");
/// let pointer = core::ptr::null::<Aligned>();
/// let tagged = TaggedPtr::checked(pointer, tag).expect("aligned null pointer");
/// assert!(TaggedPtr::pointer(tagged).is_null());
/// ```
///
/// # Failure
///
/// Insufficient type alignment or an invalid field mask causes a compilation
/// error at the declaration, even if the marker is never used.
///
/// ```compile_fail
/// use nekor_bitwise::pointer::{Bits, taggable};
///
/// struct Byte;
///
/// taggable! {
///     unsafe impl Taggable<Bits<3>> for Byte;
/// }
/// ```
///
/// # Remarks
///
/// Attributes on a declaration are forwarded to its generated implementation.
/// Place configuration attributes on the macro invocation when they must also
/// apply to the alignment check.
// NOTE: `macro_export` permits downstream declarations. `pointer::taggable!` is
// the module-qualified API, and the root name is required by Rust's macro export mechanism.
#[macro_export]
macro_rules! taggable {
    () => {};
    (
        $(#[$target_meta:meta])*

        unsafe impl Taggable<$target_tag:ty> for $target_pointee:ty;
        $($target_rest:tt)*
    ) => {
        $crate::reexport::tokel::stream! {
            const _: () = {
                let mask = <$target_tag as $crate::pointer::Tag>::MASK;

                assert!(mask != usize::MIN, "tag field must be nonempty");
                assert!(mask & mask.wrapping_add(1) == usize::MIN, "tag field must occupy contiguous low bits");
                assert!(
                    mask & !(core::mem::align_of::<$target_pointee>() - 1) == usize::MIN,
                    "tag field exceeds pointee alignment capacity"
                );
            };

            $(
                #[$target_meta]
            )*
            // SAFETY: The declaration checks the field shape and its containment
            // in the pointee's alignment capacity during const evaluation.
            unsafe impl $crate::pointer::Taggable<$target_tag> for $target_pointee {}
        }

        $crate::pointer::taggable! {
            $($target_rest)*
        }
    };
}

pub use crate::taggable;

/// A pointer carrying a tag in the low-address bits left unused by alignment.
///
/// The representation has the size and alignment of `P`. [`Taggable`] guarantees
/// that the pointee type supplies enough unused bits for `T`, while
/// [`TaggedPtr::checked()`] checks that the actual address is aligned.
/// [`TaggedPtr::new()`] instead relies on the caller to guarantee that alignment.
///
/// # Pointer Recovery
///
/// [`TaggedPtr::pointer()`] clears the reserved bits in a copy and returns the
/// original pointer. It does not change the stored tag. [`TaggedPtr::split()`]
/// returns both the original pointer and the decoded tag.
///
/// Address changes retain the original pointer's provenance. No operation
/// dereferences the pointer or establishes that its allocation is still live.
/// The caller remains responsible for the usual requirements when accessing
/// memory through a recovered pointer.
///
/// # Null Pointers
///
/// Stored tag patterns are always nonzero. A raw null pointer can therefore
/// have a nonzero tagged address, but still recover to null when its tag bits
/// are cleared. A [`NonNull`] pointer retains its non-nullness on recovery.
#[derive(Clone, Copy)]
#[repr(transparent)]
// NOTE(invariant): The reserved bits contain an assigned nonzero `T` pattern. Clearing them
// recovers the original pointer, aligned to `P::ALIGN`, with its provenance and nullability intact.
pub struct TaggedPtr<P, T>(P, PhantomData<T>)
where
    P: Pointer,
    T: Tag,
    P::Target: Taggable<T>;

impl<P, T> TaggedPtr<P, T>
where
    P: Pointer,
    T: Tag,
    P::Target: Taggable<T>,
{
    /// Construct a tagged pointer after checking its address alignment.
    ///
    /// The [`Taggable`] bound checks the pointee type's capacity separately. An
    /// over-aligned allocation alone does not make an unsupported type taggable.
    ///
    /// # Failure
    ///
    /// Returns `None` when the address does not satisfy [`Pointer::ALIGN`], even
    /// if the reserved tag bits are already zero.
    #[inline]
    #[must_use]
    pub fn checked(target_value: P, target_tag: T) -> Option<Self> {
        match P::address(target_value) & (P::ALIGN - 1) {
            usize::MIN => {
                // SAFETY: The checked address satisfies the pointee alignment.
                Some(unsafe { Self::new(target_value, target_tag) })
            },
            _ => None,
        }
    }

    /// Construct a tagged pointer from an already aligned raw pointer.
    ///
    /// # Safety
    ///
    /// The address of `target_value` must satisfy [`Pointer::ALIGN`]. This
    /// requires the pointee's alignment, not merely zeroes in the reserved tag
    /// bits.
    ///
    /// The pointer need not refer to a live allocation or be dereferenceable.
    /// Its type already enforces any nullability restriction, including that
    /// of [`NonNull`].
    #[inline]
    #[must_use]
    pub unsafe fn new(target_value: P, target_tag: T) -> Self {
        let target_code = T::encode(target_tag);
        let target_address = T::write(P::address(target_value), target_code.get());

        // SAFETY: The nonzero code makes the packed address nonzero, and the
        // field update preserves the original aligned base address.
        let target_value = unsafe { P::map(target_value, target_address).unwrap_unchecked() };

        Self(target_value, PhantomData)
    }

    /// Read the logical tag without changing the pointer representation.
    #[inline]
    #[must_use]
    pub fn tag(target_value: Self) -> T {
        let Self(target_pointer, _) = target_value;

        let target_code = T::read(P::address(target_pointer));

        // SAFETY: The stored field contains a nonzero bijection code.
        let target_code = unsafe { NonZero::<usize>::new_unchecked(target_code) };

        // SAFETY: The code is assigned and the bijection is context-independent.
        unsafe { T::decode(target_code).unwrap_unchecked() }
    }

    /// Recover the original pointer without changing the stored tag.
    ///
    /// The returned pointer retains its provenance and original address. This
    /// does not extend the lifetime of its allocation or authorize dereferencing.
    #[inline]
    #[must_use]
    pub fn pointer(target_value: Self) -> P {
        let Self(target_pointer, _) = target_value;

        let target_address = T::write(P::address(target_pointer), usize::MIN);

        // SAFETY: Zeroing the field recovers the original pointer representation,
        // which cannot be null when `P` is `NonNull`.
        unsafe { P::map(target_pointer, target_address).unwrap_unchecked() }
    }

    /// Replace the tag, returning the updated pointer and previous tag.
    ///
    /// The recovered pointer is unchanged. Since this takes `Self` by value,
    /// existing copies retain their previous tag.
    #[inline]
    #[must_use]
    pub fn replace(target_value: Self, target_tag: T) -> (Self, T) {
        let previous_tag = Self::tag(target_value);

        let Self(target_pointer, _) = target_value;

        let target_code = T::encode(target_tag);
        let target_address = T::write(P::address(target_pointer), target_code.get());

        // SAFETY: The nonzero code makes the mapped address nonzero and leaves
        // the original aligned base address unchanged.
        let target_pointer = unsafe { P::map(target_pointer, target_address).unwrap_unchecked() };
        let target_value = Self(target_pointer, PhantomData);

        (target_value, previous_tag)
    }

    /// Split the tagged value into its original pointer and logical tag.
    #[inline]
    #[must_use]
    pub fn split(target_value: Self) -> (P, T) {
        let target_pointer = Self::pointer(target_value);
        let target_tag = Self::tag(target_value);

        (target_pointer, target_tag)
    }
}

/// The smallest unsigned storage type capable of holding `N` bits.
type Word<const N: usize> = <Size as For<N>>::Target;

/// The numeric tag field ending at bit `N`, with the selected storage type `O`.
type Area<'value, const N: usize, O> = Field<'value, 0, N, usize, O>;

/// The mutable counterpart of [`Area`].
type AreaMut<'value, const N: usize, O> = <Area<'value, N, O> as Counterpart>::Mut;

/// A nonzero numeric tag occupying `N` low address bits.
///
/// Every nonzero pattern fitting within those bits is a valid tag, and the
/// numeric value is its own encoding. For example, `Bits<3>` represents the
/// values `1..=7`. Use a separate [`Bijection`] and [`Tag`] implementation when
/// only a subset of the patterns has a logical meaning.
///
/// # Width and Storage
///
/// `N` is a bit count, not an inclusive endpoint. [`For`] selects the smallest
/// unsigned integer capable of storing those bits, so `Bits<8>` uses [`u8`]
/// while `Bits<9>` uses [`u16`]. Field endpoints are selected at compile time.
///
/// Numeric tags support widths from two bits through the target's pointer
/// width. The lower bound follows the multi-bit [`Field`] interface. A custom
/// single-bit tag can instead use [`Bit`](crate::field::single::Bit) operations.
///
/// ```compile_fail
/// use nekor_bitwise::pointer::{Bits, Tag};
///
/// fn tag<T: Tag>() {}
/// tag::<Bits<1>>();
/// ```
///
/// Widths exceeding the target's pointer width do not implement [`Tag`].
///
/// ```compile_fail
/// use nekor_bitwise::pointer::{Bits, Tag};
///
/// fn tag<T: Tag>() {}
/// tag::<Bits<{ usize::BITS as usize + 1 }>>();
/// ```
#[derive(Clone, Copy)]
#[repr(transparent)]
// NOTE(invariant): The contained value is nonzero and fits the `N`-bit low-aligned field mask.
pub struct Bits<const N: usize>(Word<N>)
where
    Size: For<N>;

impl<const N: usize> Bits<N>
where
    Size: For<N>,
    Word<N>: TryFrom<usize> + TryInto<usize>,
    Self: Tag,
{
    /// Construct a numeric tag from a nonzero value fitting within `N` bits.
    ///
    /// # Failure
    ///
    /// Returns `None` for zero or a value exceeding the field width. The input
    /// is checked rather than truncated.
    #[inline]
    #[must_use]
    pub fn checked(target_value: usize) -> Option<Self> {
        match (target_value, target_value & !Self::MASK) {
            (usize::MIN, _) => None,
            (_, usize::MIN) => {
                // SAFETY: The field mask bounds the value by the capacity of
                // the unsigned storage type selected by `For`.
                let target_value = unsafe { Word::<N>::try_from(target_value).unwrap_unchecked() };

                Some(Self(target_value))
            },
            _ => None,
        }
    }

    /// Read the nonzero numeric tag value.
    #[inline]
    #[must_use]
    pub fn value(target_value: Self) -> usize {
        let Self(target_value) = target_value;

        // SAFETY: The supported numeric tag widths do not exceed `usize`.
        unsafe { target_value.try_into().unwrap_unchecked() }
    }
}

/// Implement numeric tags with concrete field endpoints and storage types.
macro_rules! bits {
    () => {};
    (
        [$($target_width:literal),* $(,)?]
    ) => {
        crate::reexport::tokel::stream! {
            $(
            // SAFETY: Checked construction admits exactly the nonzero field codes.
            // Both conversions preserve their numeric value without aliases.
            unsafe impl Bijection for Bits<$target_width> {
                #[inline]
                fn encode(target_value: Self) -> NonZero::<usize> {
                    let target_value = Self::value(target_value);

                    // SAFETY: The numeric tag invariant excludes zero.
                    unsafe { NonZero::<usize>::new_unchecked(target_value) }
                }

                #[inline]
                fn decode(target_value: NonZero::<usize>) -> Option<Self> {
                    Self::checked(target_value.get())
                }
            }

            // SAFETY: `Area` defines the mask and both projections. Every assigned
            // code fits it, and the counterpart preserves all bits outside the field.
            unsafe impl Tag for Bits<$target_width> {
                const MASK: usize = Area::<{ $target_width - 1 }, Word<$target_width>>::mask();

                #[inline]
                fn read(target_value: usize) -> usize {
                    let target_code = Area::<{ $target_width - 1 }, Word<$target_width>>::wrap(&target_value).value();

                    // SAFETY: The selected field output is no wider than `usize`.
                    unsafe { target_code.try_into().unwrap_unchecked() }
                }

                #[inline]
                fn write(mut target_value: usize, target_code: usize) -> usize {
                    let target_code = Area::<{ $target_width - 1 }, Word<$target_width>>::wrap(&target_code).value();

                    _ = AreaMut::<{ $target_width - 1 }, Word<$target_width>>::wrap(&mut target_value).merge(target_code);

                    target_value
                }
            }

            )*
        }


    };
}

bits!([2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);

#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
bits!([17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]);

#[cfg(target_pointer_width = "64")]
bits!([
    33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61,
    62, 63, 64,
]);

#[cfg(test)]
mod tests {
    //! Exercise tag assignments, raw fields, alignment proofs, and pointer provenance
    //! independently.

    use core::{
        marker::PhantomData,
        mem::{align_of, size_of},
        num::NonZero,
        ptr::{self, NonNull},
        task::RawWakerVTable,
    };

    use super::{Bijection, Bits, Pointer, Tag, Taggable, TaggedPtr};
    use crate::field::{counterpart::Counterpart, multi::Field};

    /// A copyable pointee whose alignment leaves three low address bits available.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[repr(align(8))]
    struct Aligned(pub usize);

    /// Sparse nonzero discriminants used directly as assigned field codes.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[repr(u8)]
    enum Mode {
        /// An inactive state represented by code `1`.
        Idle = 1,

        /// A runnable state represented by code `5`.
        Ready = 5,
    }

    /// The source field reserved for the sparse [`Mode`] codes.
    type ModeField<'value> = Field<'value, 0, 2, usize>;

    /// The mutable counterpart of [`ModeField`].
    type ModeFieldMut<'value> = <ModeField<'value> as Counterpart>::Mut;

    // SAFETY: The distinct nonzero discriminants are their own codes. Decoding
    // accepts exactly those discriminants, so both inverse laws hold.
    unsafe impl Bijection for Mode {
        #[inline]
        fn encode(target_value: Self) -> NonZero<usize> {
            // SAFETY: Every `Mode` discriminant is nonzero.
            unsafe { NonZero::<usize>::new_unchecked(target_value as usize) }
        }

        #[inline]
        fn decode(target_value: NonZero<usize>) -> Option<Self> {
            match target_value.get() {
                1 => Some(Self::Idle),
                5 => Some(Self::Ready),
                _ => None,
            }
        }
    }

    // SAFETY: `ModeField` supplies a contiguous low field containing both assigned codes.
    // Its mutable counterpart truncates raw input and preserves every bit outside the field.
    unsafe impl Tag for Mode {
        const MASK: usize = ModeField::mask();

        #[inline]
        fn read(target_value: usize) -> usize {
            usize::from(ModeField::wrap(&target_value).value())
        }

        #[inline]
        fn write(mut target_value: usize, target_code: usize) -> usize {
            let target_code = ModeField::wrap(&target_code).value();

            _ = ModeFieldMut::wrap(&mut target_value).merge(target_code);

            target_value
        }
    }

    crate::pointer::taggable! {
        unsafe impl Taggable<Mode> for Aligned;
        unsafe impl Taggable<Bits<2>> for Aligned;
        unsafe impl Taggable<Bits<3>> for Aligned;
    }

    // NOTE: `RawWakerVTable` guarantees at least eight-byte alignment.
    crate::pointer::taggable! {
        unsafe impl Taggable<Bits<3>> for RawWakerVTable;
    }

    /// Naming a pointer/tag pairing requires the pointee's alignment proof.
    struct TypeBound<P, T>(pub PhantomData<(P, T)>)
    where
        P: Pointer,
        T: Tag,
        P::Target: Taggable<T>;

    /// Tag storage does not enlarge any supported pointer representation.
    #[test]
    fn sized_layout() {
        assert_eq!(
            size_of::<TaggedPtr<*const Aligned, Mode>>(),
            size_of::<*const Aligned>()
        );
        assert_eq!(
            align_of::<TaggedPtr<*const Aligned, Mode>>(),
            align_of::<*const Aligned>()
        );
        assert_eq!(size_of::<TaggedPtr<*mut Aligned, Bits<2>>>(), size_of::<*mut Aligned>());
        assert_eq!(
            align_of::<TaggedPtr<*mut Aligned, Bits<2>>>(),
            align_of::<*mut Aligned>()
        );
        assert_eq!(
            size_of::<TaggedPtr<NonNull<Aligned>, Bits<3>>>(),
            size_of::<NonNull<Aligned>>()
        );
        assert_eq!(
            align_of::<TaggedPtr<NonNull<Aligned>, Bits<3>>>(),
            align_of::<NonNull<Aligned>>()
        );
        assert_eq!(size_of::<Bits<3>>(), size_of::<u8>());
        assert_eq!(size_of::<Bits<8>>(), size_of::<u8>());
        assert_eq!(size_of::<Bits<9>>(), size_of::<u16>());
        assert_eq!(size_of::<Bits<16>>(), size_of::<u16>());
        assert_eq!(align_of::<Aligned>(), 8);
    }

    /// Concrete macro proofs satisfy generic bounds before any constructor is called.
    #[test]
    fn marker_type_bounds() {
        let _: TypeBound<*const Aligned, Mode> = TypeBound(PhantomData);
        let _: TypeBound<*mut Aligned, Mode> = TypeBound(PhantomData);
        let _: TypeBound<NonNull<Aligned>, Mode> = TypeBound(PhantomData);
        let _: TypeBound<*const Aligned, Bits<2>> = TypeBound(PhantomData);
        let _: TypeBound<*mut Aligned, Bits<2>> = TypeBound(PhantomData);
        let _: TypeBound<NonNull<Aligned>, Bits<2>> = TypeBound(PhantomData);
        let _: TypeBound<*const Aligned, Bits<3>> = TypeBound(PhantomData);
        let _: TypeBound<*mut Aligned, Bits<3>> = TypeBound(PhantomData);
        let _: TypeBound<NonNull<Aligned>, Bits<3>> = TypeBound(PhantomData);
    }

    /// A downstream-style declaration proves the concrete [`RawWakerVTable`] pairing.
    #[test]
    fn raw_waker_vtable_marker() {
        let _: TypeBound<*const RawWakerVTable, Bits<3>> = TypeBound(PhantomData);
        let _: TypeBound<*mut RawWakerVTable, Bits<3>> = TypeBound(PhantomData);
        let _: TypeBound<NonNull<RawWakerVTable>, Bits<3>> = TypeBound(PhantomData);

        let pointer = ptr::null::<RawWakerVTable>();
        let tag = Bits::<3>::checked(7).expect("seven fits the nonzero three-bit field");
        let tagged = TaggedPtr::checked(pointer, tag).expect("null satisfies the pointee alignment");

        assert_eq!(TaggedPtr::pointer(tagged), pointer);
        assert_eq!(Bits::value(TaggedPtr::tag(tagged)), 7);
    }

    /// Numeric tags reject zero and overflow rather than truncating either input.
    #[test]
    fn numeric_zero_and_overflow() {
        assert!(Bits::<2>::checked(usize::MIN).is_none());
        assert!(Bits::<2>::checked(4).is_none());
        assert!(Bits::<3>::checked(usize::MIN).is_none());
        assert!(Bits::<3>::checked(8).is_none());
        assert!(Bits::<3>::checked(0x101).is_none());
        assert!(Bits::<3>::checked(usize::MAX).is_none());

        let two_bits = Bits::<2>::checked(3).expect("three is the largest two-bit code");
        let three_bits = Bits::<3>::checked(7).expect("seven is the largest three-bit code");

        assert_eq!(Bits::value(two_bits), 3);
        assert_eq!(Bits::value(three_bits), 7);
        assert_eq!(<Bits<2> as Tag>::MASK, 3);
        assert_eq!(<Bits<3> as Tag>::MASK, ModeField::mask());
    }

    /// Each sparse logical value round-trips through its assigned nonzero code.
    #[test]
    fn mode_value_bijection() {
        for (value, code) in [(Mode::Idle, 1), (Mode::Ready, 5)] {
            let encoded = Mode::encode(value);

            assert_eq!(encoded.get(), code);
            assert_eq!(Mode::decode(encoded), Some(value));
        }

        assert_ne!(Mode::encode(Mode::Idle), Mode::encode(Mode::Ready));
    }

    /// Accepted codes round-trip exactly, while holes and out-of-field codes have no aliases.
    #[test]
    fn mode_code_bijection() {
        for code in [1, 5] {
            let code = NonZero::<usize>::new(code).expect("assigned codes are nonzero");
            let decoded = Mode::decode(code).expect("the code is assigned");

            assert_eq!(Mode::encode(decoded), code);
        }

        for code in [2, 3, 4, 6, 7, 8, usize::MAX] {
            let code = NonZero::<usize>::new(code).expect("unassigned test codes are nonzero");

            assert_eq!(Mode::decode(code), None);
        }
    }

    /// Numeric assignments satisfy both inverse laws throughout their nonzero field domain.
    #[test]
    fn numeric_bijection() {
        for value in 1..=7 {
            let tag = Bits::<3>::checked(value).expect("the value fits the nonzero field");
            let code = Bits::encode(tag);
            let decoded = Bits::<3>::decode(code).expect("every nonzero field code is assigned");

            assert_eq!(code.get(), value);
            assert_eq!(Bits::value(decoded), value);
            assert_eq!(Bits::encode(decoded), code);
        }

        let overflow = NonZero::<usize>::new(8).expect("the overflow code is nonzero");

        assert!(Bits::<3>::decode(overflow).is_none());
    }

    /// The widest supported field retains every nonzero native address code.
    #[test]
    fn native_width_bijection() {
        type Native = Bits<{ usize::BITS as usize }>;

        assert_eq!(size_of::<Native>(), size_of::<usize>());
        assert_eq!(Native::MASK, usize::MAX);
        assert!(Native::checked(usize::MIN).is_none());

        for value in [1, usize::MAX - 1, usize::MAX] {
            let tag = Native::checked(value).expect("every nonzero native code fits");
            let decoded = Native::decode(Native::encode(tag)).expect("the code is assigned");

            assert_eq!(Native::value(decoded), value);
            assert_eq!(Native::read(value), value);
            assert_eq!(Native::write(usize::MAX, value), value);
        }

        assert_eq!(Native::write(usize::MAX, usize::MIN), usize::MIN);
    }

    /// Raw field operations accept zero and holes independently of semantic assignment.
    #[test]
    fn pure_field_operations() {
        assert_eq!(Mode::MASK, ModeField::mask());
        assert_eq!(Mode::read(0x1238), usize::MIN);
        assert_eq!(Mode::read(0x123a), 2);
        assert_eq!(Mode::write(0x123f, usize::MIN), 0x1238);
        assert_eq!(Mode::write(0x1238, 5), 0x123d);
        assert_eq!(Mode::write(0x123d, 8), 0x1238);
        assert_eq!(Mode::write(0x123d, 10), 0x123a);
        assert_eq!(<Bits<2> as Tag>::read(0x123c), usize::MIN);
        assert_eq!(<Bits<2> as Tag>::write(0x123f, usize::MIN), 0x123c);
        assert_eq!(<Bits<2> as Tag>::write(0x1238, usize::MAX), 0x123b);
        assert_eq!(<Bits<3> as Tag>::read(usize::MIN), usize::MIN);
        assert_eq!(<Bits<3> as Tag>::write(0x123f, usize::MIN), 0x1238);

        for base in [usize::MIN, 0x1238, usize::MAX - Mode::MASK] {
            for code in usize::MIN..=15 {
                let expected_code = code % (Mode::MASK + 1);
                let seeded = Mode::write(base, Mode::MASK);
                let packed = Mode::write(seeded, code);
                assert_eq!(Mode::read(packed), expected_code);
                assert_eq!(Mode::write(packed, usize::MIN), base);
                assert_eq!(<Bits<3> as Tag>::write(seeded, code), packed);
            }
        }
    }

    /// Actual pointer addresses must be aligned even when the pointee has a valid marker.
    #[test]
    fn actual_field_misalignment() {
        let value = Aligned(17);
        let pointer = ptr::from_ref(&value);
        let tag = Bits::<3>::checked(3).expect("three fits the nonzero field");

        for offset in 1..8 {
            let misaligned = pointer.cast::<u8>().wrapping_add(offset).cast::<Aligned>();

            assert!(TaggedPtr::checked(misaligned, Mode::Idle).is_none());
            assert!(TaggedPtr::checked(misaligned, tag).is_none());
        }

        assert!(TaggedPtr::checked(pointer, Mode::Idle).is_some());
        assert!(TaggedPtr::checked(pointer, tag).is_some());
    }

    /// Clearing a smaller tag field does not substitute for the pointee's full alignment.
    #[test]
    fn actual_full_misalignment() {
        let value = Aligned(23);
        let pointer = ptr::from_ref(&value);
        let misaligned = pointer.cast::<u8>().wrapping_add(4).cast::<Aligned>();
        let mutable = misaligned.cast_mut();
        let nonnull = NonNull::new(mutable).expect("the misaligned address remains nonzero");
        let tag = Bits::<2>::checked(1).expect("one fits the nonzero two-bit field");

        // NOTE: `Bits<2>` sees a zero field, but `Aligned` still requires eight-byte alignment.
        assert_eq!(<Bits<2> as Tag>::read(Pointer::address(misaligned)), usize::MIN);
        assert!(TaggedPtr::checked(misaligned, tag).is_none());
        assert!(TaggedPtr::checked(mutable, tag).is_none());
        assert!(TaggedPtr::checked(nonnull, tag).is_none());
        assert!(TaggedPtr::checked(pointer, tag).is_some());
    }

    /// Const, mutable, and non-null pointers recover their original representations.
    #[test]
    fn all_pointer_kinds() {
        let mut value = Aligned(31);
        let mutable = ptr::from_mut(&mut value);
        let constant = mutable.cast_const();
        let nonnull = NonNull::new(mutable).expect("a live pointee has a nonzero address");
        let constant_tagged = TaggedPtr::checked(constant, Mode::Idle).expect("the pointee is aligned");
        let mutable_tagged = TaggedPtr::checked(mutable, Mode::Ready).expect("the pointee is aligned");
        let nonnull_tagged = TaggedPtr::checked(nonnull, Mode::Ready).expect("the pointee is aligned");

        assert_eq!(TaggedPtr::split(constant_tagged), (constant, Mode::Idle));
        assert_eq!(TaggedPtr::split(mutable_tagged), (mutable, Mode::Ready));
        assert_eq!(TaggedPtr::split(nonnull_tagged), (nonnull, Mode::Ready));
        assert_eq!(<*const Aligned as Pointer>::ALIGN, 8);
        assert_eq!(<*mut Aligned as Pointer>::ALIGN, 8);
        assert_eq!(<NonNull<Aligned> as Pointer>::ALIGN, 8);
        assert_eq!(Pointer::address(constant), Pointer::address(mutable));
        assert_eq!(Pointer::address(nonnull), Pointer::address(mutable));
    }

    /// Address mapping preserves raw nullability and rejects null for [`NonNull`].
    #[test]
    fn pointer_mapping() {
        let mut value = Aligned(37);
        let mutable = ptr::from_mut(&mut value);
        let constant = mutable.cast_const();
        let nonnull = NonNull::new(mutable).expect("a live pointee has a nonzero address");
        let address = Pointer::address(mutable);

        assert_eq!(Pointer::map(constant, address), Some(constant));
        assert_eq!(Pointer::map(mutable, address), Some(mutable));
        assert_eq!(Pointer::map(nonnull, address), Some(nonnull));
        assert_eq!(Pointer::map(constant, usize::MIN), Some(ptr::null()));
        assert_eq!(Pointer::map(mutable, usize::MIN), Some(ptr::null_mut()));
        assert_eq!(Pointer::map(nonnull, usize::MIN), None);
    }

    /// Replacing sparse codes retains the non-null base and returns the previous logical state.
    #[test]
    fn nonnull_preserved_under_replace() {
        let mut value = Aligned(41);
        let pointer = NonNull::from(&mut value);
        let tagged = TaggedPtr::checked(pointer, Mode::Idle).expect("the pointee is aligned");

        let (tagged, previous) = TaggedPtr::replace(tagged, Mode::Ready);

        assert_eq!(previous, Mode::Idle);
        assert_eq!(TaggedPtr::pointer(tagged), pointer);
        assert_eq!(TaggedPtr::tag(tagged), Mode::Ready);

        let (tagged, previous) = TaggedPtr::replace(tagged, Mode::Idle);

        assert_eq!(previous, Mode::Ready);
        assert_eq!(TaggedPtr::split(tagged), (pointer, Mode::Idle));

        let TaggedPtr(packed, _) = tagged;

        assert_ne!(Pointer::address(packed), usize::MIN);
        assert_eq!(Mode::read(Pointer::address(packed)), 1);
    }

    /// A null base retains its tag in a nonzero packed address.
    #[test]
    fn null_base_with_nonzero_encoded_address() {
        let pointer = ptr::null::<Aligned>();
        let tagged = TaggedPtr::checked(pointer, Mode::Idle).expect("null satisfies the pointee alignment");

        let TaggedPtr(packed, _) = tagged;

        assert_eq!(Pointer::address(packed), 1);
        assert_eq!(TaggedPtr::tag(tagged), Mode::Idle);
        assert!(TaggedPtr::pointer(tagged).is_null());
        assert_eq!(TaggedPtr::tag(tagged), Mode::Idle);

        let (tagged, previous) = TaggedPtr::replace(tagged, Mode::Ready);

        let TaggedPtr(packed, _) = tagged;

        assert_eq!(Pointer::address(packed), 5);
        assert_eq!(previous, Mode::Idle);
        assert_eq!(TaggedPtr::split(tagged), (pointer, Mode::Ready));

        let mutable = ptr::null_mut::<Aligned>();
        let tag = Bits::<3>::checked(7).expect("seven fits the nonzero three-bit field");
        let tagged = TaggedPtr::checked(mutable, tag).expect("null satisfies the pointee alignment");

        let TaggedPtr(packed, _) = tagged;

        assert_eq!(Pointer::address(packed), 7);
        assert!(TaggedPtr::pointer(tagged).is_null());
        assert_eq!(Bits::value(TaggedPtr::tag(tagged)), 7);
    }

    /// Trusted construction retains provenance for reading and writing the recovered pointers.
    #[test]
    fn trusted_provenance_dereference() {
        let value = Aligned(43);
        let pointer = ptr::from_ref(&value);

        // SAFETY: `pointer` comes from a reference to the eight-byte-aligned `Aligned`.
        let tagged = unsafe { TaggedPtr::new(pointer, Mode::Idle) };

        let (tagged, previous) = TaggedPtr::replace(tagged, Mode::Ready);

        let recovered = TaggedPtr::pointer(tagged);

        assert_eq!(previous, Mode::Idle);
        assert_eq!(recovered, pointer);

        // SAFETY: Recovery retains the provenance of the live, initialized `Aligned`.
        assert_eq!(unsafe { recovered.read() }, value);

        let mut value = Aligned(47);
        let pointer = ptr::from_mut(&mut value);
        let tag = Bits::<3>::checked(3).expect("three fits the nonzero field");

        // SAFETY: `pointer` comes from an exclusive reference to an aligned `Aligned`.
        let tagged = unsafe { TaggedPtr::new(pointer, tag) };
        let recovered = TaggedPtr::pointer(tagged);

        // SAFETY: The recovered pointer retains exclusive access to the live `Aligned`.
        unsafe { recovered.write(Aligned(53)) };

        assert_eq!(value, Aligned(53));
        assert_eq!(Bits::value(TaggedPtr::tag(tagged)), 3);
    }

    /// Wider numeric fields retain their selected fixed-width output on 32-bit and 64-bit targets.
    #[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
    #[test]
    fn broad_word_output() {
        let tag = Bits::<18>::checked(0x3_ffff).expect("the eighteen-bit maximum fits its field");
        let full_word = Bits::<32>::checked(0xffff_ffff).expect("the thirty-two-bit maximum fits its field");
        let decoded = Bits::<32>::decode(Bits::encode(full_word)).expect("the numeric code is assigned");

        assert_eq!(size_of::<Bits<17>>(), size_of::<u32>());
        assert_eq!(size_of::<Bits<18>>(), size_of::<u32>());
        assert_eq!(size_of::<Bits<32>>(), size_of::<u32>());
        assert_eq!(Bits::value(tag), 0x3_ffff);
        assert!(Bits::<18>::checked(0x4_0000).is_none());
        assert_eq!(Bits::value(decoded), 0xffff_ffff);
        assert_eq!(<Bits<32> as Tag>::read(0xffff_ffff), 0xffff_ffff);
        assert_eq!(<Bits<32> as Tag>::write(0xffff_ffff, usize::MIN), usize::MIN);
    }

    /// Fields beyond a 32-bit word select [`u64`] storage on a 64-bit target.
    #[cfg(target_pointer_width = "64")]
    #[test]
    fn broad_double_word_output() {
        let tag = Bits::<33>::checked(0x1_ffff_ffff).expect("the thirty-three-bit maximum fits its field");
        let decoded = Bits::<33>::decode(Bits::encode(tag)).expect("the numeric code is assigned");

        assert_eq!(size_of::<Bits<33>>(), size_of::<u64>());
        assert_eq!(Bits::value(decoded), 0x1_ffff_ffff);
        assert!(Bits::<33>::checked(0x2_0000_0000).is_none());
        assert_eq!(<Bits<33> as Tag>::write(0x2_0000_0000, Bits::value(tag)), 0x3_ffff_ffff);
        assert_eq!(<Bits<33> as Tag>::read(0x3_ffff_ffff), Bits::value(tag));
    }
}
