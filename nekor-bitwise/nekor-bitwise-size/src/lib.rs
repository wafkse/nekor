#![no_std]
#![forbid(
    clippy::all,
    clippy::perf,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::panic,
    clippy::pedantic,
    unsafe_code,
    rustdoc::all
)]
#![recursion_limit = "256"]
//! Type-level integer primitive type selector.

use nekor_primitive::scalar::Scalar;

/// A uninhabited, completely compile-time type selector for the minimum viable
/// integer type for a target bit width.
///
/// This implements both the [`For`] and [`For2`] family of traits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Size {}

/// A helper trait for the compile-time type selector [`Size`].
pub trait For<const N: usize> {
    /// The selected scalar type.
    type Target: Scalar;
}

macro_rules! selector {
    () => {};
    (
        [$($target_index:literal),* $(,)?] for $target_type:ty
    ) => {
        $(
            impl For<$target_index> for Size {
                type Target = $target_type;
            }
        )+
    };
}

selector!(
    [0, 1, 2, 3, 4, 5, 6, 7, 8] for u8
);

selector!(
    [9, 10, 11, 12, 13, 14, 15, 16] for u16
);

selector!(
    [17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32] for u32
);

selector!(
    [33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64] for u64
);

selector!(
    [65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128] for u128
);

/// A helper trait for the compile-time type selector [`Size`].
pub trait For2<const N: usize, const M: usize> {
    /// The selected scalar type.
    type Target: Scalar;
}

/// A macro to implement the [`For2`] trait for [`Size`].
///
/// This macro will take a list of valid bit indices, and implement all possible
/// 2-element permutations of the [`For2<N, M>`] trait for the target type.
macro_rules! for2 {
    (
        @ impl [$target_index:literal]
    ) => {};
    (
        @ impl override [$target_index:literal]
    ) => {};
    (
        # impl ($target_left:literal $target_right:literal)
    ) => {
        #[automatically_derived]
        impl For2<$target_left, $target_right> for Size
        {
            type Target = <Size as For<{ $target_right - $target_left + 1 }>>::Target;
        }
    };
    (
        @ impl override [$target_index:literal $target_next_index:literal $($target_rest:literal)*]
    ) => {
        for2!(# impl ($target_index $target_next_index));

        for2!(@ impl override [$target_index $($target_rest)*]);
    };
    (
        @ impl [$target_index:literal $target_next_index:literal $($target_rest:literal)*]
    ) => {
        for2!(# impl ($target_index $target_next_index));

        for2!(@ impl override [$target_index $($target_rest)*]);
        for2!(@ impl [$target_next_index $($target_rest)*]);
    };
    (
        [
            $($target_index:literal),*
        ]
    ) => {
        for2!(@ impl [ $($target_index)* ]);
    };
}

for2!([
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49,
    50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73,
    74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97,
    98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116,
    117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127
]);
