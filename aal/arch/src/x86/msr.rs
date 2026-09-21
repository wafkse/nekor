//! x86 model-specific register transport and architectural value families.
//!
//! [`read()`] and [`write()`] perform direct `rdmsr` and `wrmsr`
//! instructions. The public submodules own the value representations and
//! capability-aware accessors for coherent architectural MSR families.

use core::{arch, convert::Infallible, mem};

use nekor_bitwise::prelude::{U64High32, U64High32Mut, U64Low32, U64Low32Mut};
use nekor_register::prelude::{
    Ro as RegisterRo, Rw as RegisterRw, Unaccessible as RegisterUnaccessible, Wo as RegisterWo,
};

use crate::x86::{
    fred::Fred as FredCapability,
    privilege::Cpl,
    xstate::{Xfd as XfdCapability, XsaveSupervisor},
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
/// Implementations select an access mode. That mode determines the canonical
/// [`nekor_register::mode::Gated`] descriptor exposed by [`Msr::REGISTER`].
///
/// # Safety
///
/// Every implementation must have the same size and alignment as `u64`, and
/// every `u64` bit pattern must be a valid value of the implementing type.
/// Transparent wrappers may delegate through another type with that contract.
/// `Authority` must require every privilege and feature proof needed to access
/// the selected architectural register.
/// [`RawMsr`] relies on these representation guarantees.
pub unsafe trait Msr: Copy + private::Sealed {
    /// Architectural MSR index loaded into `ECX` by `rdmsr` or `wrmsr`.
    const ADDRESS: u32;

    /// Architectural access mode of this MSR.
    type Access: Access<Self>;

    /// Proof family required before accessing this register.
    type Authority: Authority;

    /// Whether accesses are fenced against speculative execution.
    const FENCE: bool = false;

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
        // SAFETY: `Msr` requires `R` to have the size and alignment of `u64`
        // with every bit pattern valid. `R: Copy` means copying the representation
        // does not invalidate `target_value`.
        unsafe { mem::transmute_copy(&target_value) }
    }

    /// Returns the erased register image.
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

/// Supplies the borrowed proof required by an MSR family.
pub trait Authority: private::AuthoritySealed {
    /// Access proof tied to the current execution context.
    type Proof<'proof, T>
    where
        T: 'proof;
}

/// CPL0 proof required by architectural MSRs available without another feature token.
pub enum Cpl0Access {}

/// CPL0 and XFD proofs required by the XFD register family.
pub enum XfdAccess {}

/// CPL0 and supervisor XSAVE proofs required by IA32_XSS.
pub enum XssAccess {}

/// CPL0 and FRED proofs required by the FRED register family.
pub enum FredAccess {}

/// Uninhabited authority for registers whose full capability proof is not modeled.
pub enum UnavailableAccess {}

impl private::AuthoritySealed for Cpl0Access {}
impl private::AuthoritySealed for XfdAccess {}
impl private::AuthoritySealed for XssAccess {}
impl private::AuthoritySealed for FredAccess {}
impl private::AuthoritySealed for UnavailableAccess {}

impl Authority for Cpl0Access {
    type Proof<'proof, T>
        = &'proof Cpl<0, T>
    where
        T: 'proof;
}

impl Authority for XfdAccess {
    type Proof<'proof, T>
        = (&'proof Cpl<0, T>, &'proof XfdCapability)
    where
        T: 'proof;
}

impl Authority for XssAccess {
    type Proof<'proof, T>
        = (&'proof Cpl<0, T>, &'proof XsaveSupervisor)
    where
        T: 'proof;
}

impl Authority for FredAccess {
    type Proof<'proof, T>
        = (&'proof Cpl<0, T>, &'proof FredCapability)
    where
        T: 'proof;
}

impl Authority for UnavailableAccess {
    type Proof<'proof, T>
        = Infallible
    where
        T: 'proof;
}

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

/// Reads an MSR after receiving its privilege and feature proof.
///
/// Registers select their proof family through [`Msr::Authority`]. The proof
/// cannot be constructed without the CPL and feature capabilities required by
/// that family.
#[inline]
#[must_use]
pub fn read<R, T>(_proof: <R::Authority as Authority>::Proof<'_, T>) -> R
where
    R: Msr,
    R::Access: Readable,
{
    processor_fence(R::FENCE);

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

    processor_fence(R::FENCE);

    let mut raw = u64::MIN;
    let mut low_field = U64Low32Mut::wrap(&mut raw);

    low_field.const_merge(low);

    let mut high_field = U64High32Mut::wrap(&mut raw);

    high_field.const_merge(high);

    // SAFETY: `Msr` guarantees that `R` has the size and alignment of `u64`
    // and that every `u64` bit pattern is valid for `R`.
    unsafe { mem::transmute_copy(&raw) }
}

/// Writes an MSR after receiving its privilege and feature proof.
///
/// # Safety
///
/// `target_value` must satisfy the target register's architectural write
/// requirements. The proof establishes privilege and feature availability.
#[inline]
pub unsafe fn write<R, T>(_proof: <R::Authority as Authority>::Proof<'_, T>, target_value: R)
where
    R: Msr,
    R::Access: Writable,
{
    let RawMsr(target_value) = RawMsr::take(target_value);

    let lo = U64Low32::wrap(&target_value).const_value();
    let hi = U64High32::wrap(&target_value).const_value();

    processor_fence(R::FENCE);

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

    processor_fence(R::FENCE);
}

/// Apply the optional processor execution fence around an MSR access.
#[inline]
fn processor_fence(fence: bool) {
    if fence {
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

    /// Seal for MSR access-authority families.
    pub trait AuthoritySealed {}

    /// Seal for architectural MSR access modes.
    pub trait AccessSealed {}
}

impl Readable for ReadOnly {}
impl Readable for ReadWrite {}
impl Writable for ReadWrite {}
impl Writable for WriteOnly {}

#[cfg(test)]
mod tests {
    use nekor_register::prelude::{Ro, Rw, Unaccessible as RegisterUnaccessible, Wo};

    use super::{Cpl0Access, Msr, RawMsr, ReadOnly, ReadWrite, Unaccessible, WriteOnly};
    use crate::x86::{
        fred::Fred,
        msr::{
            efer::RawEfer,
            fred::RawFredConfig,
            xstate::{RawXfd, RawXss},
        },
        privilege::Cpl,
        xstate::{Xfd, XsaveSupervisor},
    };

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
                type Authority = Cpl0Access;

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

    #[test]
    fn authority_gats_select_the_required_proof_shape() {
        let _: for<'proof> fn(&'proof Cpl<0, ()>) -> RawEfer = super::read::<RawEfer, ()>;
        let _: for<'proof> fn((&'proof Cpl<0, ()>, &'proof Xfd)) -> RawXfd = super::read::<RawXfd, ()>;
        let _: for<'proof> fn((&'proof Cpl<0, ()>, &'proof XsaveSupervisor)) -> RawXss = super::read::<RawXss, ()>;
        let _: for<'proof> fn((&'proof Cpl<0, ()>, &'proof Fred)) -> RawFredConfig = super::read::<RawFredConfig, ()>;
    }
}
