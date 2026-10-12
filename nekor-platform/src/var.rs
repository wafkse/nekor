//! Compile-time values from the platform build environment.
//!
//! [`integer!`] reads a primitive integer whose type is inferred from the caller.
//! [`string!`] reads a `&'static str`. Both evaluate within a const block,
//! including when called from a runtime function.
//!
//! Missing variables cause compilation errors. Integer literals use Rust syntax
//! and must fit the inferred type.
//!
//! Cargo supplies the environment variables used in these examples.
//!
//! ```
//! use nekor_platform::var;
//!
//! const MAJOR: u64 = var::integer!("CARGO_PKG_VERSION_MAJOR");
//! const PACKAGE: &str = var::string!("CARGO_PKG_NAME");
//!
//! let major: usize = var::integer!("CARGO_PKG_VERSION_MAJOR");
//! let package: &str = var::string!("CARGO_PKG_NAME");
//! ```

mod detail {
    //! Seal the set of primitive platform integer types.

    /// Prevent downstream implementations of the integer marker.
    pub trait Sealed {}
}

/// Marker for the primitive signed and unsigned integer types.
///
/// This trait is sealed, and cannot be implemented externally.
pub trait Integer: detail::Sealed + Copy {}

/// Implement the integer marker and its seal for the selected primitives.
macro_rules! primitive {
    ($($type:ty),+ $(,)?) => {
        $(
            impl detail::Sealed for $type {}

            impl Integer for $type {}
        )+
    };
}

primitive!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);

/// Preserve an integer value while constraining its inferred type.
///
/// Non-integer types cannot pass this compile-time type boundary.
///
/// ```compile_fail
/// const VALUE: f64 = nekor_platform::var::identity(1.5);
/// ```
///
/// ```compile_fail
/// const VALUE: bool = nekor_platform::var::identity(true);
/// ```
#[inline]
#[must_use]
pub const fn identity<T>(target_value: T) -> T
where
    T: Integer,
{
    target_value
}

/// A string supplied by the platform build configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// NOTE(invariant): None.
pub struct VarString(&'static str);

impl VarString {
    /// Construct a platform string value.
    #[inline]
    #[must_use]
    pub const fn new(target_variable: &'static str) -> Self {
        Self(target_variable)
    }

    /// Return the platform string value.
    #[inline]
    #[must_use]
    pub const fn value(self) -> &'static str {
        let Self(value) = self;

        value
    }
}

/// An integer supplied by the platform build configuration.
///
/// [`integer!`] supplies the value with its primitive type inferred from the caller.
/// The wrapper retains the value alone; environment names are compilation inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// NOTE(invariant): The sealed integer bound restricts the value to a primitive integer.
pub struct VarInteger<T>(T)
where
    T: Integer;

impl<T> VarInteger<T>
where
    T: Integer,
{
    /// Construct a platform integer value.
    #[inline]
    #[must_use]
    pub const fn new(target_value: T) -> Self {
        Self(target_value)
    }

    /// Borrow the platform integer value.
    #[inline]
    #[must_use]
    pub const fn value(&self) -> &T {
        let Self(value) = self;

        value
    }
}

/// Read a platform integer at compile time.
///
/// The platform supplies Rust integer literal syntax. Unsuffixed literals infer
/// their primitive type from the surrounding declaration, such as
/// `let value: u64 = var::integer!("NEKOR_NAME");`.
/// Missing variables and values outside that type's range fail compilation.
/// Construction remains in a const block inside runtime functions as well.
///
/// ```
/// use nekor_platform::var;
///
/// const MAJOR: u64 = var::integer!("CARGO_PKG_VERSION_MAJOR");
/// let major: i32 = var::integer!("CARGO_PKG_VERSION_MAJOR");
/// ```
#[macro_export]
macro_rules! integer {
    ($target_name:literal $(,)?) => {
        const {
            // Retain the builtin expansion for compiler environment dependency tracking.
            let _ = env!($target_name);

            let target_value = $crate::reexport::tokel::stream! {
                [< >]:environment[[$target_name]]:unstringify
            };

            let target_value = $crate::var::identity(target_value);

            *$crate::var::VarInteger::value(&$crate::var::VarInteger::new(target_value))
        }
    };
}

/// Read a platform string at compile time.
///
/// The result is a `&'static str`. Missing variables fail compilation.
///
/// ```
/// use nekor_platform::var;
///
/// const PACKAGE: &str = var::string!("CARGO_PKG_NAME");
/// let package = var::string!("CARGO_PKG_NAME");
/// ```
#[macro_export]
macro_rules! string {
    ($target_name:literal $(,)?) => {
        const {
            let target_content = env!($target_name);

            $crate::var::VarString::value($crate::var::VarString::new(target_content))
        }
    };
}

pub use crate::{integer, string};

#[cfg(test)]
mod tests {
    use crate::var::{self, VarInteger, VarString};

    #[test]
    fn integer_type_is_inferred_in_const_and_runtime_contexts() {
        const UNSIGNED: u128 = var::integer!("CARGO_PKG_VERSION_MAJOR");
        const SIGNED: i8 = var::integer!("CARGO_PKG_VERSION_MAJOR");

        let word: usize = var::integer!("CARGO_PKG_VERSION_MAJOR");
        let wide: u64 = var::integer!("CARGO_PKG_VERSION_MAJOR");

        assert_eq!(UNSIGNED, 0);
        assert_eq!(SIGNED, 0);
        assert_eq!(word, 0);
        assert_eq!(wide, 0);
        assert_eq!(size_of::<VarInteger<u64>>(), size_of::<u64>());
    }

    #[test]
    fn string_value_does_not_retain_its_environment_name() {
        const VALUE: &str = var::string!("CARGO_PKG_NAME");

        assert_eq!(VALUE, "nekor-platform");
        assert_eq!(size_of::<VarString>(), size_of::<&str>());
    }
}
