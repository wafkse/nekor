//! Data structure parametrization for uniform support of various operating
//! modes.

use nekor_primitive::prelude::Scalar;

#[cfg(target_pointer_width = "16")]
#[doc(inline)]
pub use self::Bits16 as Native;
#[cfg(target_pointer_width = "32")]
#[doc(inline)]
pub use self::Bits32 as Native;
#[cfg(target_pointer_width = "64")]
#[doc(inline)]
pub use self::Bits64 as Native;

/// Private sealing implementation for [`Mode`].
mod private {
    /// A trait to act as a seal supertrait to [`Mode`].
    ///
    /// [`Mode`]: super::Mode
    pub trait Sealed {}
}

/// A marker trait for the distinct addressing modes in a `x86` processor.
pub trait Mode: private::Sealed {
    /// The address size used by the mode.
    type Address: Scalar;
}

/// The 32-bit operating mode, otherwise known as *Protected Mode*.
///
/// # Remarks
///
/// `N`-level paging is optional but desired in this mode.
#[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
#[doc(alias = "protected mode")]
pub enum Bits32 {}

impl private::Sealed for Bits32 {}

impl Mode for Bits32 {
    type Address = u32;
}

/// The 64-bit operating mode, otherwise known as *Long Mode*.
///
/// # Remarks
///
/// `N`-level (where `N ∈ {4,5}`) paging is mandatory in this mode.
#[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
#[doc(alias = "long mode")]
pub enum Bits64 {}

impl private::Sealed for Bits64 {}

impl Mode for Bits64 {
    type Address = u64;
}
