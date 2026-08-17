//! Endianness helper types.

mod detail {
    //! Implementation details sub-module.

    /// A trait to act as a supertrait seal for the [`Endian`] trait.
    ///
    /// [`Endian`]: super::Endian
    pub trait Sealed {}
}

/// A trait for integer primitives that can be represented in a non-native
/// endian.
pub trait Endian: detail::Sealed + Copy + 'static {}

/// A helper macro that expands to an appropiate [`detail::Sealed`] + [`Endian`]
/// implementation for a target list of types.
macro_rules! endian {
    ($($target_type:ty),+ $(,)?) => {
        $(
            impl detail::Sealed for $target_type {}

            impl Endian for $target_type {}
        )+
    };
}

endian![u8, u16, u32, u64, u128];
endian![i8, i16, i32, i64, i128];

/// A newtype `repr(transparent)` wrapper over an integer primitive represented
/// internally as a little-endian value.
///
/// On platforms where little-endian is the de-facto endianness, this is a
/// no-op.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct Le<T>(T)
where
    T: Endian;

impl<T> Le<T>
where
    T: Endian,
{
    /// Creates a new wrapper from an integer primitive already represented as a
    /// little-endian value.
    #[inline]
    #[must_use]
    pub const fn bare(target_value: T) -> Self {
        Self(target_value)
    }

    /// Returns the integer primitive represented internally as a little-endian
    /// value.
    #[inline]
    #[must_use]
    pub const fn raw(&self) -> T {
        let &Self(target_value) = self;

        target_value
    }

    /// Returns a mutable reference to the integer primitive represented
    /// internally as a little-endian value.
    #[inline]
    #[must_use]
    pub const fn raw_mut(&mut self) -> &mut T {
        let Self(target_value) = self;

        target_value
    }
}

/// Implements little-endian conversion methods for the provided integer
/// primitives.
macro_rules! le {
    ($($target_type:ty),+ $(,)?) => {
        $(
            impl Le<$target_type> {
                /// Creates a new wrapper from an integer primitive represented as a
                /// native-endian value.
                #[inline]
                #[must_use]
                pub const fn new(native_value: $target_type) -> Self {
                    Self(<$target_type>::to_le(native_value))
                }

                /// Returns the integer primitive represented as a native-endian value.
                #[inline]
                #[must_use]
                pub const fn unwrap(self) -> $target_type {
                    let Self(target_value) = self;

                    <$target_type>::from_le(target_value)
                }
            }
        )+
    };
}

le![u8, u16, u32, u64, u128];
le![i8, i16, i32, i64, i128];

/// A newtype `repr(transparent)` wrapper over an integer primitive represented
/// internally as a big-endian value.
///
/// On platforms where big-endian is the de-facto endianess, this is a no-op.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct Be<T>(T)
where
    T: Endian;

impl<T> Be<T>
where
    T: Endian,
{
    /// Creates a new wrapper from an integer primitive already represented as a
    /// big-endian value.
    #[inline]
    #[must_use]
    pub const fn bare(target_value: T) -> Self {
        Self(target_value)
    }

    /// Returns the integer primitive represented internally as a big-endian
    /// value.
    #[inline]
    #[must_use]
    pub const fn raw(&self) -> T {
        let &Self(target_value) = self;

        target_value
    }

    /// Returns a mutable reference to the integer primitive represented
    /// internally as a big-endian value.
    #[inline]
    #[must_use]
    pub const fn raw_mut(&mut self) -> &mut T {
        let Self(target_value) = self;

        target_value
    }
}

/// Implements big-endian conversion methods for the provided integer
/// primitives.
macro_rules! be {
    ($($target_type:ty),+ $(,)?) => {
        $(
            impl Be<$target_type> {
                /// Creates a new wrapper from an integer primitive represented as a
                /// native-endian value.
                #[inline]
                #[must_use]
                pub const fn new(native_value: $target_type) -> Self {
                    Self(<$target_type>::to_be(native_value))
                }

                /// Returns the integer primitive represented as a native-endian value.
                #[inline]
                #[must_use]
                pub const fn unwrap(self) -> $target_type {
                    let Self(target_value) = self;

                    <$target_type>::from_be(target_value)
                }
            }
        )+
    };
}

be![u8, u16, u32, u64, u128];
be![i8, i16, i32, i64, i128];
