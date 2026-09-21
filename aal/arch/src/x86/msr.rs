//! x86 model-specific register transport and architectural value families.
//!
//! [`read()`] and [`write()`] perform direct `rdmsr` and `wrmsr`
//! instructions. The public submodules own the value representations and
//! capability-aware accessors for coherent architectural MSR families.

use core::{arch, mem};

use nekor_bitwise::prelude::{U64High32, U64High32Mut, U64Low32, U64Low32Mut};
use nekor_register::prelude::{
    Ro as RegisterRo, Rw as RegisterRw, Unaccessible as RegisterUnaccessible, Wo as RegisterWo,
};

/// Extended feature enable register representations and access.
pub mod efer;

/// Fast system-call and system-return register representations.
pub mod syscall;

/// Legacy SYSENTER register representations.
pub mod sysenter;

/// FS and GS segment-base register representations.
pub mod segment;

/// Extended-state register representations and access.
pub mod xstate;

/// Flexible Return and Event Delivery register representations and access.
pub mod fred;

/// An exact model-specific register image with a statically known MSR index.
///
/// Implementations select one access mode. That mode determines the canonical
/// [`nekor_register::mode::Gated`] descriptor exposed by [`Msr::REGISTER`].
///
/// # Safety
///
/// Every implementation must be `#[repr(transparent)]` with exactly one field
/// of type `u64`. It must therefore have the same size and alignment as `u64`,
/// and every `u64` bit pattern must be a valid value of the implementing type.
/// [`RawMsr`] relies on this representation contract.
pub unsafe trait Msr: Copy + private::Sealed {
    /// Architectural MSR index loaded into `ECX` by `rdmsr` or `wrmsr`.
    const ADDRESS: u32;

    /// Architectural access mode of this MSR.
    type Access: Access<Self>;

    /// Canonical register descriptor for this MSR.
    const REGISTER: <Self::Access as Access<Self>>::Register = <Self::Access as Access<Self>>::REGISTER;
}

/// Erased 64-bit model-specific register image.
///
/// This type intentionally carries no register identity or semantic validity. Every `u64`
/// bit-pattern is representable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct RawMsr(pub u64);

impl RawMsr {
    /// Constructs an erased model-specific register image.
    #[inline]
    #[must_use]
    pub const fn new(target_value: u64) -> Self {
        Self(target_value)
    }

    /// Erase a typed MSR into its raw 64-bit representation.
    #[inline]
    #[must_use]
    pub const fn take<R>(target_value: R) -> Self
    where
        R: Msr,
    {
        // SAFETY: `Msr` requires `R` to be transparent over exactly one `u64`
        // and to have identical size and alignment. `R: Copy` means copying the
        // representation does not invalidate `target_value`.
        unsafe { mem::transmute_copy(&target_value) }
    }

    /// Returns the complete erased register image.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u64 {
        let Self(target_value) = self;

        target_value
    }
}

/// Translate an MSR access mode into its canonical register descriptor.
pub trait Access<R>: private::AccessSealed
where
    R: Msr<Access = Self>,
{
    /// `Ro`, `Rw`, `Wo`, or `Unaccessible` descriptor selected by this mode.
    type Register;

    /// Register descriptor at the MSR's architectural index.
    const REGISTER: Self::Register;
}

/// An MSR access mode that permits reads.
pub trait Readable: private::AccessSealed {}

/// An MSR access mode that permits writes.
pub trait Writable: private::AccessSealed {}

/// A model-specific register whose architectural availability is proven by FRED support.
pub trait FredMsr: Msr + private::FredSealed {}

/// Read-only MSR access.
pub enum ReadOnly {}

/// Read-write MSR access.
pub enum ReadWrite {}

/// Write-only MSR access.
pub enum WriteOnly {}

/// Architecturally described MSR without direct access through this interface.
pub enum Unaccessible {}

impl private::AccessSealed for ReadOnly {}
impl private::AccessSealed for ReadWrite {}
impl private::AccessSealed for WriteOnly {}
impl private::AccessSealed for Unaccessible {}

impl<R> Access<R> for ReadOnly
where
    R: Msr<Access = Self>,
{
    type Register = RegisterRo<R, usize>;

    const REGISTER: Self::Register = RegisterRo::register(R::ADDRESS as usize);
}

impl<R> Access<R> for ReadWrite
where
    R: Msr<Access = Self>,
{
    type Register = RegisterRw<R, usize>;

    const REGISTER: Self::Register = RegisterRw::register(R::ADDRESS as usize);
}

impl<R> Access<R> for WriteOnly
where
    R: Msr<Access = Self>,
{
    type Register = RegisterWo<R, usize>;

    const REGISTER: Self::Register = RegisterWo::register(R::ADDRESS as usize);
}

impl<R> Access<R> for Unaccessible
where
    R: Msr<Access = Self>,
{
    type Register = RegisterUnaccessible<R, usize>;

    const REGISTER: Self::Register = RegisterUnaccessible::register(R::ADDRESS as usize);
}

/// Read one readable MSR directly with `rdmsr`.
///
/// When `FENCE` is `true`, an `LFENCE` is emitted immediately before and after
/// `RDMSR`, preventing instruction execution from crossing the access. The
/// inline assembly intentionally does not use `nomem`, so it is also a compiler
/// barrier for surrounding memory accesses.
///
/// # Safety
///
/// The current privilege level and processor model must permit access to `R`.
#[inline]
#[must_use]
pub unsafe fn read<R, const FENCE: bool>() -> R
where
    R: Msr,
    R::Access: Readable,
{
    processor_fence::<FENCE>();

    let high: u32;
    let low: u32;

    // SAFETY: The caller guarantees that `R::ADDRESS` names an available,
    // readable MSR at the current privilege level. Omitting `nomem` makes this
    // asm block a compiler barrier for surrounding memory accesses.
    unsafe {
        arch::asm!(
            "rdmsr",
            in("ecx") R::ADDRESS,
            lateout("edx") high,
            lateout("eax") low,
            options(nostack, preserves_flags, att_syntax)
        );
    }

    processor_fence::<FENCE>();

    let mut raw = u64::MIN;
    let mut low_field = U64Low32Mut::wrap(&mut raw);

    low_field.const_merge(low);

    let mut high_field = U64High32Mut::wrap(&mut raw);

    high_field.const_merge(high);

    // SAFETY: `Msr` guarantees that `R` is transparent over exactly one `u64`
    // and that every `u64` bit pattern is valid for `R`.
    unsafe { mem::transmute_copy(&raw) }
}

/// Write one writable MSR directly with `wrmsr`.
///
/// When `FENCE` is `true`, an `LFENCE` is emitted immediately before and after
/// `WRMSR`, preventing instruction execution from crossing the access. The
/// inline assembly intentionally does not use `nomem`, so it is also a compiler
/// barrier for surrounding memory accesses.
///
/// # Safety
///
/// The current privilege level and processor model must permit access to `R`,
/// and `value` must satisfy that MSR's architectural requirements.
#[inline]
pub unsafe fn write<R, const FENCE: bool>(target_value: R)
where
    R: Msr,
    R::Access: Writable,
{
    let RawMsr(target_value) = RawMsr::take(target_value);

    let lo = U64Low32::wrap(&target_value).const_value();
    let hi = U64High32::wrap(&target_value).const_value();

    processor_fence::<FENCE>();

    // SAFETY: The caller guarantees that `R::ADDRESS` names an available,
    // writable MSR and that `value` is architecturally valid for it. Omitting
    // `nomem` makes this asm block a compiler barrier for surrounding memory
    // accesses.
    unsafe {
        arch::asm!(
            "wrmsr",
            in("ecx") R::ADDRESS,
            in("edx") hi,
            in("eax") lo,
            options(nostack, preserves_flags, att_syntax)
        );
    }

    processor_fence::<FENCE>();
}

/// Apply the optional processor execution fence around an MSR access.
#[inline]
fn processor_fence<const FENCE: bool>() {
    if FENCE {
        // SAFETY: `lfence` only constrains processor execution ordering. The
        // missing `nomem` option also prevents compiler memory motion across
        // this boundary.
        unsafe {
            arch::asm!("lfence", options(nostack, preserves_flags, att_syntax));
        }
    }
}

/// Sealing traits for architectural MSR and access-mode implementations.
mod private {
    /// Seal for architectural MSR value types.
    pub trait Sealed {}

    /// Seal for architectural MSR access modes.
    pub trait AccessSealed {}

    /// Seal for model-specific registers whose availability follows FRED enumeration.
    pub trait FredSealed {}
}

impl Readable for ReadOnly {}
impl Readable for ReadWrite {}
impl Writable for ReadWrite {}
impl Writable for WriteOnly {}

#[cfg(test)]
mod tests {
    use nekor_register::prelude::{Ro, Rw, Unaccessible as RegisterUnaccessible, Wo};

    use super::{Msr, RawMsr, ReadOnly, ReadWrite, Unaccessible, WriteOnly};

    #[derive(Clone, Copy)]
    #[repr(transparent)]
    struct TestRo(u64);

    #[derive(Clone, Copy)]
    #[repr(transparent)]
    struct TestRw(u64);

    #[derive(Clone, Copy)]
    #[repr(transparent)]
    struct TestWo(u64);

    #[derive(Clone, Copy)]
    #[repr(transparent)]
    struct TestNone(u64);

    macro_rules! test_msr {
        ($target:ty, $address:expr, $access:ty) => {
            // SAFETY: The synthetic MSR is a transparent `u64` wrapper with no invalid bit
            // patterns.
            unsafe impl Msr for $target {
                type Access = $access;

                const ADDRESS: u32 = $address;
            }

            impl super::private::Sealed for $target {}
        };
    }

    test_msr!(TestRo, 1, ReadOnly);
    test_msr!(TestRw, 2, ReadWrite);
    test_msr!(TestWo, 3, WriteOnly);
    test_msr!(TestNone, 4, Unaccessible);

    #[test]
    fn access_modes_translate_to_register_gates() {
        fn ro(_: Ro<TestRo, usize>) {}
        fn rw(_: Rw<TestRw, usize>) {}
        fn wo(_: Wo<TestWo, usize>) {}
        fn inaccessible(_: RegisterUnaccessible<TestNone, usize>) {}

        ro(TestRo::REGISTER);
        rw(TestRw::REGISTER);
        wo(TestWo::REGISTER);
        inaccessible(TestNone::REGISTER);
    }

    #[test]
    fn raw_msr_erases_typed_representation() {
        let RawMsr(target_value) = RawMsr::take(TestRw(0x800));

        assert_eq!(target_value, 0x800);
    }
}
