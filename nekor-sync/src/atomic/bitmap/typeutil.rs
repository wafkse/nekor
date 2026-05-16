//! Type-level utilities to determine whether a bit index or a N-bit integer is
//! within an [`usize`] bitset.

mod detail {
    //! Implementation details for the [`InBound`] trait.
    /// [`InBound`]: super::InBound

    /// A trait to act as a supertrait seal for the [`InBound`] marker trait.
    ///
    /// [`InBound`]: super::InBound
    pub trait Sealed {}
}

/// A marker trait to determine whether an array size can be represented within
/// an [`usize`] bitset.
#[diagnostic::on_unimplemented(message = "{Self} not in `usize` bitwise boundary")]
pub trait InBound: detail::Sealed {}

/// The default bit index type for a given bit count.
pub type BitIndex<const N: u32> = BitIndexU32<N>;

/// A maker type to signal that the (0-based) `N`-th bit is indeed inside an
/// [`usize`]-sized integer.
pub enum BitIndexU32<const N: u32> {}

/// A maker type to signal that the (0-based) `N`-th bit is indeed inside an
/// [`usize`]-sized integer.
pub enum BitIndexUsize<const N: usize> {}

impl<const N: u32> detail::Sealed for BitIndexU32<N> {}

impl<const N: usize> detail::Sealed for BitIndexUsize<N> {}

/// A macro that implements [`InBound`] for [`BitIndex<N>`], where N is an
/// elegible size.
macro_rules! bitindex {
    () => {};
    ($($target_size:literal),+) => {
        $(
            impl InBound for BitIndexU32<{ $target_size }> { }

            impl InBound for BitIndexUsize<{ $target_size }> { }
        )+
    };
}

bitindex!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15);

#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
bitindex!(
    16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31
);

#[cfg(target_pointer_width = "64")]
bitindex!(
    32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55,
    56, 57, 58, 59, 60, 61, 62, 63
);

/// The default bit map type for a given bit count.
pub type BitMap<const N: u32> = BitMapU32<N>;

/// A maker type to signal that the `N`-th integer fits within an
/// [`usize`]-sized integer.
pub enum BitMapU32<const N: u32> {}

/// A maker type to signal that the `N`-th integer fits within an
/// [`usize`]-sized integer.
pub enum BitMapUsize<const N: usize> {}

impl<const N: u32> detail::Sealed for BitMapU32<N> {}

impl<const N: usize> detail::Sealed for BitMapUsize<N> {}

/// A macro that implements [`InBound`] for [`BitMap<N>`], where N is an
/// elegible size.
macro_rules! bitmap {
    () => {};
    ($($target_size:literal),+) => {
        $(
            impl InBound for BitMapU32<{ $target_size }> { }

            impl InBound for BitMapUsize<{ $target_size }> { }
        )+
    };
}

bitmap!(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16);

#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
bitmap!(
    17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32
);

#[cfg(target_pointer_width = "64")]
bitmap!(
    33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56,
    57, 58, 59, 60, 61, 62, 63, 64
);

#[cfg(test)]
mod tests {
    use super::*;

    // Test that BitIndex implements InBound for valid indices
    #[test]
    fn test_bitindex_0_is_in_bound() {
        fn assert_in_bound<T: InBound>() {}
        assert_in_bound::<BitIndex<0>>();
    }

    #[test]
    fn test_bitindex_15_is_in_bound() {
        fn assert_in_bound<T: InBound>() {}
        assert_in_bound::<BitIndex<15>>();
    }

    #[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
    #[test]
    fn test_bitindex_31_is_in_bound() {
        fn assert_in_bound<T: InBound>() {}
        assert_in_bound::<BitIndex<31>>();
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_bitindex_63_is_in_bound() {
        fn assert_in_bound<T: InBound>() {}
        assert_in_bound::<BitIndex<63>>();
    }

    // Test all 16-bit valid indices
    #[test]
    fn test_bitindex_all_16bit_valid() {
        fn assert_in_bound<T: InBound>() {}

        assert_in_bound::<BitIndex<0>>();
        assert_in_bound::<BitIndex<1>>();
        assert_in_bound::<BitIndex<2>>();
        assert_in_bound::<BitIndex<3>>();
        assert_in_bound::<BitIndex<4>>();
        assert_in_bound::<BitIndex<5>>();
        assert_in_bound::<BitIndex<6>>();
        assert_in_bound::<BitIndex<7>>();
        assert_in_bound::<BitIndex<8>>();
        assert_in_bound::<BitIndex<9>>();
        assert_in_bound::<BitIndex<10>>();
        assert_in_bound::<BitIndex<11>>();
        assert_in_bound::<BitIndex<12>>();
        assert_in_bound::<BitIndex<13>>();
        assert_in_bound::<BitIndex<14>>();
        assert_in_bound::<BitIndex<15>>();
    }

    #[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
    #[test]
    fn test_bitindex_32bit_extended_valid() {
        fn assert_in_bound<T: InBound>() {}

        assert_in_bound::<BitIndex<16>>();
        assert_in_bound::<BitIndex<17>>();
        assert_in_bound::<BitIndex<20>>();
        assert_in_bound::<BitIndex<25>>();
        assert_in_bound::<BitIndex<30>>();
        assert_in_bound::<BitIndex<31>>();
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_bitindex_64bit_extended_valid() {
        fn assert_in_bound<T: InBound>() {}

        assert_in_bound::<BitIndex<32>>();
        assert_in_bound::<BitIndex<40>>();
        assert_in_bound::<BitIndex<50>>();
        assert_in_bound::<BitIndex<60>>();
        assert_in_bound::<BitIndex<63>>();
    }

    // Test BitMap implementations
    #[test]
    fn test_bitmap_1_is_in_bound() {
        fn assert_in_bound<T: InBound>() {}
        assert_in_bound::<BitMap<1>>();
    }

    #[test]
    fn test_bitmap_16_is_in_bound() {
        fn assert_in_bound<T: InBound>() {}
        assert_in_bound::<BitMap<16>>();
    }

    #[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
    #[test]
    fn test_bitmap_32_is_in_bound() {
        fn assert_in_bound<T: InBound>() {}
        assert_in_bound::<BitMap<32>>();
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_bitmap_64_is_in_bound() {
        fn assert_in_bound<T: InBound>() {}
        assert_in_bound::<BitMap<64>>();
    }

    // Test all 16-bit valid bitmap sizes
    #[test]
    fn test_bitmap_all_16bit_valid() {
        fn assert_in_bound<T: InBound>() {}

        assert_in_bound::<BitMap<1>>();
        assert_in_bound::<BitMap<2>>();
        assert_in_bound::<BitMap<3>>();
        assert_in_bound::<BitMap<4>>();
        assert_in_bound::<BitMap<5>>();
        assert_in_bound::<BitMap<6>>();
        assert_in_bound::<BitMap<7>>();
        assert_in_bound::<BitMap<8>>();
        assert_in_bound::<BitMap<9>>();
        assert_in_bound::<BitMap<10>>();
        assert_in_bound::<BitMap<11>>();
        assert_in_bound::<BitMap<12>>();
        assert_in_bound::<BitMap<13>>();
        assert_in_bound::<BitMap<14>>();
        assert_in_bound::<BitMap<15>>();
        assert_in_bound::<BitMap<16>>();
    }

    #[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
    #[test]
    fn test_bitmap_32bit_extended_valid() {
        fn assert_in_bound<T: InBound>() {}

        assert_in_bound::<BitMap<17>>();
        assert_in_bound::<BitMap<20>>();
        assert_in_bound::<BitMap<24>>();
        assert_in_bound::<BitMap<28>>();
        assert_in_bound::<BitMap<32>>();
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_bitmap_64bit_extended_valid() {
        fn assert_in_bound<T: InBound>() {}

        assert_in_bound::<BitMap<33>>();
        assert_in_bound::<BitMap<40>>();
        assert_in_bound::<BitMap<50>>();
        assert_in_bound::<BitMap<60>>();
        assert_in_bound::<BitMap<64>>();
    }

    // Test boundary conditions
    #[test]
    fn test_bitindex_boundary_min() {
        fn assert_in_bound<T: InBound>() {}
        assert_in_bound::<BitIndex<0>>();
    }

    #[test]
    fn test_bitmap_boundary_min() {
        fn assert_in_bound<T: InBound>() {}
        assert_in_bound::<BitMap<1>>();
    }

    // Test that sealed trait cannot be implemented externally
    #[test]
    fn test_sealed_trait_prevents_external_impl() {
        // This test just verifies the types exist and are properly sealed
        // The actual sealing is enforced at compile time
        fn check_sealed<T: detail::Sealed>() {}
        check_sealed::<BitIndex<0>>();
        check_sealed::<BitMap<1>>();
    }

    // Test usage with actual bitmap operations
    #[test]
    fn test_bitindex_with_actual_bitmap() {
        use crate::atomic::bitmap::AtomicBitmap;
        use crate::atomic::bitmap::mode::cooperative::Cooperative;

        let bitmap = AtomicBitmap::zeroed();

        // These should compile because BitIndex<N> is InBound
        let _ = bitmap.static_at::<0>().one::<Cooperative>();
        let _ = bitmap.static_at::<5>().one::<Cooperative>();
        let _ = bitmap.static_at::<15>().one::<Cooperative>();

        let expected = (1 << 0) | (1 << 5) | (1 << 15);
        assert_eq!(bitmap.snapshot(), expected);
    }

    #[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
    #[test]
    fn test_bitindex_32bit_with_bitmap() {
        use crate::atomic::bitmap::AtomicBitmap;
        use crate::atomic::bitmap::mode::cooperative::Cooperative;

        let bitmap = AtomicBitmap::zeroed();

        let _ = bitmap.static_at::<16>().one::<Cooperative>();
        let _ = bitmap.static_at::<31>().one::<Cooperative>();

        let expected = (1 << 16) | (1 << 31);
        assert_eq!(bitmap.snapshot(), expected);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_bitindex_64bit_with_bitmap() {
        use crate::atomic::bitmap::AtomicBitmap;
        use crate::atomic::bitmap::mode::cooperative::Cooperative;

        let bitmap = AtomicBitmap::zeroed();

        let _ = bitmap.static_at::<32>().one::<Cooperative>();
        let _ = bitmap.static_at::<63>().one::<Cooperative>();

        let expected = (1usize << 32) | (1usize << 63);
        assert_eq!(bitmap.snapshot(), expected);
    }

    // Test const evaluation
    #[test]
    fn test_bitindex_const_context() {
        const fn test_const<const N: u32>() -> u32
        where
            BitIndex<N>: InBound,
        {
            N
        }

        assert_eq!(test_const::<0>(), 0);
        assert_eq!(test_const::<10>(), 10);
        assert_eq!(test_const::<15>(), 15);
    }

    #[test]
    fn test_bitmap_const_context() {
        const fn test_const<const N: u32>() -> u32
        where
            BitMap<N>: InBound,
        {
            N
        }

        assert_eq!(test_const::<1>(), 1);
        assert_eq!(test_const::<8>(), 8);
        assert_eq!(test_const::<16>(), 16);
    }
}
