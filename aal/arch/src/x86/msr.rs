//! Model-specific register snapshots and access for `x86`.
//!
//! [`read()`] and [`write()`] perform direct `rdmsr` and `wrmsr`
//! instructions. The public submodules define register families and their
//! capability-aware accessors.

use core::{arch, convert::Infallible, mem};

use nekor_bitwise::prelude::{U64High32, U64High32Mut, U64Low32, U64Low32Mut};
use nekor_register::prelude::Gated;

use crate::x86::{
    fred::FredCapability,
    instruction,
    privilege::Cpl,
    xstate::{XfdCapability, XsaveSupervisorCapability},
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

/// A trait for model-specific register snapshots.
///
/// Each implementation associates a snapshot type with its architectural index,
/// a [`Gated`] register description, and an [`Authority`] proof family.
/// [`read()`] and [`write()`] use these associations to access the register.
///
/// # Remarks
///
/// This trait is sealed and is only implemented for the model-specific register snapshot types
/// declared by this module and its submodules.
///
/// # Safety
///
/// Every implementation must have the same size and alignment as `u64`, and
/// every `u64` bit pattern must be a valid value of the implementing type.
/// Transparent wrappers may delegate through another type with that contract.
/// `Authority` must require every privilege and feature proof needed to access
/// the selected architectural register.
/// [`RawMsr`] relies on these representation guarantees.
pub unsafe trait Msr: Copy + detail::Sealed {
    /// Architectural model-specific register index loaded into `ECX` by `rdmsr` or `wrmsr`.
    const ADDRESS: u32;

    /// The [`Gated`] register description encoding read and write permissions.
    type Access: Access<Self>;

    /// The [`Authority`] selecting the proof required to access this register.
    type Authority: Authority;

    /// Whether [`instruction::lfence()`] is executed before and after each access.
    const FENCE: bool = false;

    /// The register description at [`Self::ADDRESS`] with [`Self::Access`] permissions.
    const REGISTER: Self::Access = Self::Access::REGISTER;
}

/// A transparent wrapper over a `u64` containing a model-specific register snapshot.
///
/// Every `u64` bit pattern is representable. [`Self::take()`] erases the register
/// identity of an [`Msr`] snapshot, and [`Self::give()`] restores a selected snapshot type.
///
/// # Remarks
///
/// This type stores register bits without an architectural index, access mode,
/// or authority proof. Constructing or converting a snapshot does not establish
/// that its contents satisfy the register's architectural write requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct RawMsr(
    /// The raw 64-bit register snapshot.
    pub u64,
);

impl RawMsr {
    /// Constructs an erased model-specific register snapshot.
    #[inline]
    #[must_use]
    pub const fn new(target_value: u64) -> Self {
        Self(target_value)
    }

    /// Erase a typed model-specific register into its raw 64-bit representation.
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

    /// Restore a typed model-specific register snapshot without asserting architectural write
    /// validity.
    #[inline]
    #[must_use]
    pub const fn give<R>(self) -> R
    where
        R: Msr,
    {
        let Self(target_value) = self;

        // SAFETY: `Msr` requires the size and alignment of u64 with every bit
        // pattern valid. Restoring the snapshot does not access the register.
        unsafe { mem::transmute_copy(&target_value) }
    }

    /// Returns the erased register snapshot.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u64 {
        let Self(target_value) = self;

        target_value
    }
}

/// A trait for register descriptions associated with an [`Msr`] snapshot.
///
/// [`Self::REGISTER`] describes the register at [`Msr::ADDRESS`] while retaining
/// its snapshot type and read and write permissions.
///
/// # Remarks
///
/// This trait is sealed and is only implemented for [`Gated`] descriptions with
/// a `u32` address and the snapshot type `R`.
pub trait Access<R>: detail::access::Sealed + Sized
where
    R: Msr<Access = Self>,
{
    /// The register description at [`Msr::ADDRESS`].
    const REGISTER: Self;
}

/// A marker trait for model-specific register register descriptions that permit reads.
///
/// [`read()`] requires this marker on [`Msr::Access`].
///
/// # Remarks
///
/// This trait is sealed and is only implemented for [`Gated`] descriptions with
/// a `u32` address and the `READ` parameter set to `true`.
pub trait Readable: detail::access::Sealed {}

/// A marker trait for model-specific register register descriptions that permit writes.
///
/// [`write()`] requires this marker on [`Msr::Access`].
///
/// # Remarks
///
/// This trait is sealed and is only implemented for [`Gated`] descriptions with
/// a `u32` address and the `WRITE` parameter set to `true`.
pub trait Writable: detail::access::Sealed {}

/// A trait selecting the proof required to access a model-specific register family.
///
/// [`Msr::Authority`] associates a register snapshot with this proof family.
/// [`read()`] and [`write()`] require the corresponding [`Self::Proof`].
///
/// # Remarks
///
/// This trait is sealed and is only implemented for the uninhabited authority
/// types declared in this module.
pub trait Authority: detail::authority::Sealed {
    /// The privilege and feature proof required by this authority.
    ///
    /// `T` is the payload of the [`Cpl`] proof, and `'proof` is its borrow lifetime.
    type Proof<'proof, T>
    where
        T: 'proof;
}

/// An uninhabited type selecting a proof of privilege level zero for register access.
///
/// Its [`Authority`] implementation requires a shared reference to [`Cpl`] at
/// privilege level zero.
pub enum Cpl0Access {}

/// An uninhabited type selecting privilege and extended feature disable proofs for register access.
///
/// Its [`Authority`] implementation requires shared references to [`Cpl`] at
/// privilege level zero and the [`XfdCapability`] proving `XFD` support.
pub enum XfdAccess {}

/// An uninhabited type selecting privilege and supervisor `XSAVE` proofs for `IA32_XSS` access.
///
/// Its [`Authority`] implementation requires shared references to [`Cpl`] at
/// privilege level zero and the [`XsaveSupervisorCapability`] proving `IA32_XSS` availability.
pub enum XssAccess {}

/// An uninhabited type selecting privilege and `FRED` proofs for model-specific register access.
///
/// Its [`Authority`] implementation requires shared references to [`Cpl`] at
/// privilege level zero and the [`FredCapability`] proving `FRED` support.
pub enum FredAccess {}

/// An uninhabited type selecting an unavailable model-specific register access proof.
///
/// Its [`Authority::Proof`] is [`Infallible`], which prevents constructing the
/// proof required by [`read()`] and [`write()`]. Register snapshots can still be
/// represented while their required capability proofs are not modeled.
pub enum UnavailableAccess {}

impl detail::authority::Sealed for Cpl0Access {}
impl detail::authority::Sealed for XfdAccess {}
impl detail::authority::Sealed for XssAccess {}
impl detail::authority::Sealed for FredAccess {}
impl detail::authority::Sealed for UnavailableAccess {}

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
        = (&'proof Cpl<0, T>, &'proof XsaveSupervisorCapability)
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

impl<R, const READ: bool, const WRITE: bool> Access<R> for Gated<R, u32, READ, WRITE>
where
    R: Msr<Access = Self>,
{
    const REGISTER: Self = Self::register(R::ADDRESS);
}

impl<R, const READ: bool, const WRITE: bool> detail::access::Sealed for Gated<R, u32, READ, WRITE> {}

impl<R, const WRITE: bool> Readable for Gated<R, u32, true, WRITE> {}
impl<R, const READ: bool> Writable for Gated<R, u32, READ, true> {}

/// Reads a model-specific register after receiving its privilege and feature proof.
///
/// Registers select their proof family through [`Msr::Authority`]. The proof
/// cannot be constructed without the CPL and feature capabilities required by
/// that family.
#[inline]
#[must_use]
pub fn read<R, T>(_: <R::Authority as Authority>::Proof<'_, T>) -> R
where
    R: Msr,
    R::Access: Readable,
{
    if const { R::FENCE } {
        instruction::lfence();
    }

    let half_high: u32;
    let half_low: u32;

    // SAFETY: The authority proof and `Msr` contract establish availability,
    // privilege, and readability. Omitting `nomem` makes this
    // asm block a compiler barrier for surrounding memory accesses.
    unsafe {
        arch::asm!(
            "rdmsr",
            in("ecx") R::ADDRESS,
            lateout("edx") half_high,
            lateout("eax") half_low,
            options(nostack, preserves_flags, att_syntax)
        );
    }

    if const { R::FENCE } {
        instruction::lfence();
    }

    let mut raw = u64::MIN;

    U64Low32Mut::wrap(&mut raw).const_merge(half_low);
    U64High32Mut::wrap(&mut raw).const_merge(half_high);

    RawMsr::new(raw).give()
}

/// Writes a model-specific register after receiving its privilege and feature proof.
///
/// # Safety
///
/// `target_value` must satisfy the target register's architectural write
/// requirements. The proof establishes privilege and feature availability.
#[inline]
pub unsafe fn write<R, T>(_: <R::Authority as Authority>::Proof<'_, T>, target_value: R)
where
    R: Msr,
    R::Access: Writable,
{
    let RawMsr(target_value) = RawMsr::take(target_value);

    let low_half = U64Low32::wrap(&target_value).const_value();
    let high_half = U64High32::wrap(&target_value).const_value();

    if const { R::FENCE } {
        instruction::lfence();
    }

    // SAFETY: The authority proof and `Msr` contract establish availability,
    // privilege, and writability. The caller guarantees architectural write validity. Omitting
    // `nomem` makes this asm block a compiler barrier for surrounding memory
    // accesses.
    unsafe {
        arch::asm!(
            "wrmsr",
            in("ecx") R::ADDRESS,
            in("edx") high_half,
            in("eax") low_half,
            options(nostack, preserves_flags, att_syntax)
        );
    }

    if const { R::FENCE } {
        instruction::lfence();
    }
}

/// Implementation details for model-specific register snapshots, authorities, and access modes.
mod detail {
    /// Shared model-specific register snapshot declarations.
    pub(in crate::x86::msr) mod snapshotted {
        /// Defines a transparent `u64` model-specific register snapshot with its index and
        /// authority.
        macro_rules! snapshot {
            () => {};
            (
                $(
                    #[$target_meta:meta]
                )*

                $target_vis:vis struct $target_name:ident for ($target_address:expr)
                use $target_authority:ty
            ) => {
                tokel::stream!(
                    $(
                        #[$target_meta]
                    )*
                    #[repr(transparent)]
                    $target_vis struct $target_name(u64);

                    impl $target_name {
                        /// Constructs a raw register snapshot.
                        #[inline]
                        #[must_use]
                        $target_vis const fn new(target_value: u64) -> Self {
                            Self(target_value)
                        }

                        /// Returns the raw register snapshot.
                        #[inline]
                        #[must_use]
                        $target_vis const fn raw(self) -> u64 {
                            let Self(target_value) = self;

                            target_value
                        }
                    }

                    // SAFETY: The snapshot is transparent over u64 with every bit pattern valid.
                    // The declared authority supplies the register's privilege and feature proof.
                    unsafe impl $crate::x86::msr::Msr for $target_name {
                        type Access = nekor_register::mode::Rw<Self, u32>;
                        type Authority = $target_authority;

                        const ADDRESS: u32 = $target_address;
                    }

                    impl $crate::x86::msr::detail::Sealed for $target_name {}
                );
            };
        }

        // NOTE: model-specific register family modules share the representation contract
        // implemented by this macro.
        pub(in crate::x86::msr) use snapshot;
    }

    /// A trait to act as a seal supertrait for the [`Msr`] trait.
    ///
    /// [`Msr`]: super::Msr
    pub trait Sealed {}

    /// Sealing for model-specific register access-authority families.
    pub mod authority {
        /// A trait to act as a seal supertrait for the [`Authority`] trait.
        ///
        /// [`Authority`]: super::super::Authority
        pub trait Sealed {}
    }

    /// Sealing for architectural model-specific register access modes.
    pub mod access {
        /// A trait to act as a seal supertrait for the [`Access`], [`Readable`], and
        /// [`Writable`] traits.
        ///
        /// [`Access`]: super::super::Access
        /// [`Readable`]: super::super::Readable
        /// [`Writable`]: super::super::Writable
        pub trait Sealed {}
    }
}

#[cfg(test)]
mod tests {
    use nekor_register::prelude::{Ro, Rw, Unaccessible, Wo};

    use super::{Msr, RawMsr};
    use crate::x86::{
        fred::FredCapability,
        msr::{
            efer::RawEfer,
            fred::RawFredConfig,
            xstate::{RawXfd, RawXss},
        },
        privilege::Cpl,
        xstate::{XfdCapability, XsaveSupervisorCapability},
    };

    /// Implementation details for synthetic model-specific register snapshots.
    mod detail {
        /// Defines a synthetic model-specific register snapshot with the selected register
        /// permissions.
        macro_rules! snapshot {
            () => {};
            (
                $(
                    #[$target_meta:meta]
                )*

                $target_vis:vis struct $target_name:ident for ($target_address:expr) use $target_access:ident
            ) => {
                tokel::stream!(
                    $(
                        #[$target_meta]
                    )*
                    #[repr(transparent)]
                    $target_vis struct $target_name(u64);

                    // SAFETY: The synthetic model-specific register is a transparent u64 wrapper with no invalid bit patterns.
                    unsafe impl $crate::x86::msr::Msr for $target_name {
                        type Access = $target_access<Self, u32>;
                        type Authority = $crate::x86::msr::Cpl0Access;

                        const ADDRESS: u32 = $target_address;
                    }

                    impl $crate::x86::msr::detail::Sealed for $target_name {}
                );
            };
        }

        pub(in crate::x86::msr::tests) use snapshot;
    }

    detail::snapshot!(
        #[derive(Clone, Copy)]
        struct TestRo for (1) use Ro
    );

    detail::snapshot!(
        #[derive(Clone, Copy)]
        struct TestRw for (2) use Rw
    );

    detail::snapshot!(
        #[derive(Clone, Copy)]
        struct TestWo for (3) use Wo
    );

    detail::snapshot!(
        #[derive(Clone, Copy)]
        struct TestNone for (4) use Unaccessible
    );

    #[test]
    fn access_modes_translate_to_register_gates() {
        fn ro(_: Ro<TestRo, u32>) {}
        fn rw(_: Rw<TestRw, u32>) {}
        fn wo(_: Wo<TestWo, u32>) {}
        fn inaccessible(_: Unaccessible<TestNone, u32>) {}

        ro(TestRo::REGISTER);
        rw(TestRw::REGISTER);
        wo(TestWo::REGISTER);
        inaccessible(TestNone::REGISTER);

        assert_eq!(TestRo::REGISTER.address(), TestRo::ADDRESS);
        assert_eq!(TestRw::REGISTER.address(), TestRw::ADDRESS);
        assert_eq!(TestWo::REGISTER.address(), TestWo::ADDRESS);
        assert_eq!(TestNone::REGISTER.address(), TestNone::ADDRESS);
    }

    #[test]
    fn permitted_operations_retain_authority_proofs() {
        let _: fn(&Cpl<0, ()>) -> TestRo = super::read::<TestRo, ()>;
        let _: fn(&Cpl<0, ()>) -> TestRw = super::read::<TestRw, ()>;
        let _: unsafe fn(&Cpl<0, ()>, TestRw) = super::write::<TestRw, ()>;
        let _: unsafe fn(&Cpl<0, ()>, TestWo) = super::write::<TestWo, ()>;
    }

    #[test]
    fn raw_msr_erases_typed_representation() {
        let RawMsr(target_value) = RawMsr::take(TestRw(0x800));

        assert_eq!(target_value, 0x800);
    }

    #[test]
    fn raw_conversion_is_const_and_preserves_arbitrary_bits() {
        const SNAPSHOT: u64 = 0xa5a5_0123_ffff_8001;
        const EFER: RawEfer = RawMsr::new(SNAPSHOT).give();
        const XFD: RawXfd = RawMsr::new(SNAPSHOT).give();
        const XSS: RawXss = RawMsr::new(SNAPSHOT).give();
        const FRED: RawFredConfig = RawMsr::new(SNAPSHOT).give();
        const ERASED: [RawMsr; 4] = [
            RawMsr::take(EFER),
            RawMsr::take(XFD),
            RawMsr::take(XSS),
            RawMsr::take(FRED),
        ];

        assert_eq!(EFER.raw(), SNAPSHOT);
        assert_eq!(XFD.raw(), SNAPSHOT);
        assert_eq!(XSS.raw(), SNAPSHOT);
        assert_eq!(FRED.raw(), SNAPSHOT);
        assert_eq!(ERASED, [RawMsr(SNAPSHOT); 4]);

        for target_value in [u64::MIN, u64::MAX, 1, 1 << 63] {
            let TestRw(restored) = RawMsr::new(target_value).give();

            assert_eq!(restored, target_value);
        }
    }

    #[test]
    fn authority_gats_select_the_required_proof_shape() {
        let _: for<'proof> fn(&'proof Cpl<0, ()>) -> RawEfer = super::read::<RawEfer, ()>;
        let _: for<'proof> fn((&'proof Cpl<0, ()>, &'proof XfdCapability)) -> RawXfd = super::read::<RawXfd, ()>;
        let _: for<'proof> fn((&'proof Cpl<0, ()>, &'proof XsaveSupervisorCapability)) -> RawXss =
            super::read::<RawXss, ()>;
        let _: for<'proof> fn((&'proof Cpl<0, ()>, &'proof FredCapability)) -> RawFredConfig =
            super::read::<RawFredConfig, ()>;
    }
}
