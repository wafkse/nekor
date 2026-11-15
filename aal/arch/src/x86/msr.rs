//! Typed model-specific register images and direct processor access.
//!
//! Register images preserve all 64 bits, including reserved and processor-maintained fields.
//! Their constructors and field views prepare or inspect values without accessing hardware.
//! [`Msr`] associates each image with an architectural index and a [`Gated`] access description.
//!
//! [`read()`] and [`write()`] accept a value implementing [`Authorize`] for that register.
//! Authorization establishes the execution context, while a write also requires the caller
//! to establish that installing the supplied bits is safe. Neither operation adds fences.

use core::arch;

use nekor_bitwise::prelude::{U64High32, U64High32Mut, U64Low32, U64Low32Mut};
use nekor_register::prelude::Gated;

pub mod efer;
pub mod fred;
pub mod segment;
pub mod syscall;
pub mod sysenter;
pub mod xstate;

/// A trait associating an MSR image with its index and permitted operations.
///
/// Conversion explicitly constructs or extracts bits. It does not depend on the image's
/// Rust layout and makes no claim that the processor accepts a particular value.
/// Implementations belong to the architectural register declarations in this module.
pub trait Msr: Copy + detail::Sealed {
    /// The index supplied in `ECX` by the access instructions.
    const ADDRESS: u32;

    /// The register description carrying read and write permissions.
    type Access: Access<Self>;

    /// The typed description at the architectural index.
    const REGISTER: Self::Access = Self::Access::REGISTER;

    /// Construct an image without discarding any register bits.
    fn new(target_value: u64) -> Self;

    /// Extract the register bits without accessing hardware.
    fn raw(self) -> u64;
}

/// A trait authorizing access to a particular MSR in the executing context.
///
/// Implement this trait on the capability or borrowed collection of capabilities supplied
/// to an access. It is open to platform-specific owners, so they can express their own
/// execution scopes without another authority selector or associated proof type.
/// Authorization does not establish that an arbitrary register value is safe to install.
///
/// A CPL0 token alone cannot authorize a register requiring an additional feature.
///
/// ```compile_fail
/// use nekor_aal_arch::x86::{msr::{self, xstate::MsrXss}, privilege::Cpl};
///
/// fn supervisor_state(privilege: &Cpl<0, ()>) -> MsrXss {
///     msr::read::<MsrXss>(privilege)
/// }
/// ```
///
/// A platform can attach authorization to its own borrowed execution owner.
///
/// ```
/// use nekor_aal_arch::x86::{
///     msr::{self, Authorize, efer::MsrEfer},
///     privilege::Cpl,
/// };
///
/// struct Kernel<'scope>(&'scope Cpl<0, ()>);
///
/// // SAFETY: The owner borrows the CPL0 execution token required for EFER access.
/// unsafe impl Authorize<MsrEfer> for &Kernel<'_> {}
///
/// fn inspect(kernel: &Kernel<'_>) -> MsrEfer {
///     let Kernel(privilege) = kernel;
///     let _: &Cpl<0, ()> = privilege;
///     msr::read(kernel)
/// }
/// ```
///
/// An arbitrary value cannot supply authorization.
///
/// ```compile_fail
/// use nekor_aal_arch::x86::msr::{self, efer::MsrEfer};
/// let image = msr::read::<MsrEfer>(());
/// ```
///
/// # Safety
///
/// For soundness purposes, the following must be satisfied:
///
/// * Possession of the implementing value must establish the privilege and processor support
///   required to access the selected register for the duration of the operation.
/// * Its construction and lifetime must exclude execution in a context where those guarantees no
///   longer hold, including migration to an incompatible processor.
pub unsafe trait Authorize<R: Msr> {}

/// A trait associating an [`Msr`] image with its typed register description.
///
/// Only [`Gated`] descriptions with a `u32` index implement this sealed trait.
pub trait Access<R>: detail::access::Sealed + Sized
where
    R: Msr<Access = Self>,
{
    /// The description using the image's architectural index and access permissions.
    const REGISTER: Self;
}

/// A marker for [`Gated`] descriptions whose read bit is set.
pub trait Readable: detail::access::Sealed {}

/// A marker for [`Gated`] descriptions whose write bit is set.
pub trait Writable: detail::access::Sealed {}

impl<R, const READ: bool, const WRITE: bool> Access<R> for Gated<R, u32, READ, WRITE>
where
    R: Msr<Access = Self>,
{
    const REGISTER: Self = Self::register(R::ADDRESS);
}

impl<R, const READ: bool, const WRITE: bool> detail::access::Sealed for Gated<R, u32, READ, WRITE> {}

impl<R, const WRITE: bool> Readable for Gated<R, u32, true, WRITE> {}
impl<R, const READ: bool> Writable for Gated<R, u32, READ, true> {}

/// Read the register authorized by the supplied capability.
///
/// The result is an image of the observed bits, including fields the processor maintains.
#[inline]
#[must_use]
pub fn read<R>(_: impl Authorize<R>) -> R
where
    R: Msr,
    R::Access: Readable,
{
    let half_high: u32;
    let half_low: u32;

    // SAFETY: The supplied authorizer establishes the execution context for this register,
    // and the access description permits reads. The output operands receive both halves
    // before the image is constructed.
    unsafe {
        arch::asm!(
            "rdmsr",
            in("ecx") R::ADDRESS,
            lateout("edx") half_high,
            lateout("eax") half_low,
            options(nostack, preserves_flags, att_syntax)
        );
    }

    let mut target_value = u64::MIN;

    U64Low32Mut::wrap(&mut target_value).const_merge(half_low);
    U64High32Mut::wrap(&mut target_value).const_merge(half_high);

    R::new(target_value)
}

/// Install an MSR image in the authorized execution context.
///
/// Callers own register-specific sequencing and any synchronization with other code or CPUs.
/// A successful instruction does not establish that dependent software state is consistent.
/// Authorization alone cannot make the write safe.
///
/// ```compile_fail
/// use nekor_aal_arch::x86::{msr::{self, efer::MsrEfer}, privilege::Cpl};
///
/// fn install(privilege: &Cpl<0, ()>) {
///     msr::write(privilege, MsrEfer::RESET);
/// }
/// ```
///
/// # Safety
///
/// For soundness purposes, the following must be satisfied:
///
/// * The supplied image must satisfy the register's architectural write requirements.
/// * Installing it must preserve the mappings, execution state and resource ownership required by
///   code that can run during or after the operation.
#[inline]
pub unsafe fn write<R>(_: impl Authorize<R>, target_value: R)
where
    R: Msr,
    R::Access: Writable,
{
    let target_value = target_value.raw();
    let low_half = U64Low32::wrap(&target_value).const_value();
    let high_half = U64High32::wrap(&target_value).const_value();

    // SAFETY: The authorizer establishes privilege and availability, and the access description
    // permits writes. The caller establishes the register's write requirements and the safety
    // of the resulting machine state. The named field views supply the architectural halves.
    unsafe {
        arch::asm!(
            "wrmsr",
            in("ecx") R::ADDRESS,
            in("edx") high_half,
            in("eax") low_half,
            options(nostack, preserves_flags, att_syntax)
        );
    }
}

/// The shared declarations and seals for architectural register images.
mod detail {
    pub(in crate::x86::msr) mod snapshotted {
        //! Common image construction and register associations.

        /// Declare an image and optionally authorize a borrowed capability for it.
        macro_rules! snapshot {
            () => {};
            (
                $(
                    #[$target_meta:meta]
                )*

                $target_vis:vis struct $target_name:ident for ($target_address:expr)
                $(for<$target_lifetime:lifetime> $target_authorization:ty)?
            ) => {
                tokel::stream!(
                    $(
                        #[$target_meta]
                    )*
                    #[repr(transparent)]
                    // NOTE(invariant): None.
                    $target_vis struct $target_name(u64);

                    impl $target_name {
                        /// Preserve all bits of a register image without accessing hardware.
                        #[inline]
                        #[must_use]
                        $target_vis const fn new(target_value: u64) -> Self {
                            Self(target_value)
                        }

                        /// Extract the image bits, including reserved fields.
                        #[inline]
                        #[must_use]
                        $target_vis const fn raw(self) -> u64 {
                            let Self(target_value) = self;

                            target_value
                        }
                    }

                    impl $crate::x86::msr::Msr for $target_name {
                        type Access = nekor_register::mode::Rw<Self, u32>;
                        const ADDRESS: u32 = $target_address;

                        #[inline]
                        fn new(target_value: u64) -> Self {
                            Self::new(target_value)
                        }

                        #[inline]
                        fn raw(self) -> u64 {
                            Self::raw(self)
                        }
                    }

                    impl $crate::x86::msr::detail::Sealed for $target_name {}

                    $(
                        // SAFETY: The borrowed capabilities establish CPL0 and the feature
                        // support required by this register throughout the access.
                        unsafe impl<$target_lifetime, T: $target_lifetime> $crate::x86::msr::Authorize<$target_name>
                            for $target_authorization {}
                    )?
                );
            };
        }

        pub(in crate::x86::msr) use snapshot;
    }

    /// A seal restricting architectural register declarations to this module.
    pub trait Sealed {}

    /// The seals for register permission descriptions.
    pub mod access {
        /// A seal implemented only for the supported `Gated` descriptions.
        pub trait Sealed {}
    }
}

#[cfg(test)]
mod tests {
    //! Check image transport and authorization without executing privileged instructions.

    use core::mem;

    use nekor_register::prelude::{Ro, Rw, Unaccessible, Wo};

    use crate::x86::{
        fred::FredCapability,
        msr::{
            self, Authorize, Msr, Readable, Writable,
            efer::MsrEfer,
            fred::{
                MsrFredConfig, MsrFredLevels, MsrFredRsp0, MsrFredRsp1, MsrFredRsp2, MsrFredRsp3, MsrFredSsp1,
                MsrFredSsp2, MsrFredSsp3,
            },
            segment::{MsrFsBase, MsrGsBase, MsrKernelGsBase},
            syscall::{MsrCStar, MsrFMask, MsrLStar, MsrStar},
            sysenter::{MsrSysEnterCs, MsrSysEnterIp, MsrSysEnterSp},
            xstate::{MsrXfd, MsrXfdErr, MsrXss},
        },
        privilege::Cpl,
        xstate::{XfdCapability, XsaveSupervisorCapability},
    };

    /// The platform may attach MSR authorization to an existing execution owner.
    // NOTE(invariant): None.
    struct Scope<'scope>(&'scope Cpl<0, ()>);

    // SAFETY: The scope borrows the same CPL0 token accepted by the EFER access contract.
    // Its lifetime cannot outlive that token's execution scope.
    unsafe impl Authorize<MsrEfer> for &Scope<'_> {}

    #[test]
    fn authorization_accepts_direct_and_platform_capabilities() {
        let _: fn(&Cpl<0, ()>) -> MsrEfer = |proof| msr::read::<MsrEfer>(proof);
        let _: fn((&Cpl<0, ()>, &XfdCapability)) -> MsrXfd = |proof| msr::read::<MsrXfd>(proof);
        let _: fn((&Cpl<0, ()>, &XsaveSupervisorCapability)) -> MsrXss = |proof| msr::read::<MsrXss>(proof);
        let _: fn((&Cpl<0, ()>, &FredCapability)) -> MsrFredConfig = |proof| msr::read::<MsrFredConfig>(proof);
        let _: fn(&Scope<'_>) = |scope| {
            let Scope(privilege) = scope;
            let _: &Cpl<0, ()> = privilege;
        };
        let _: fn(&Scope<'_>) -> MsrEfer = |proof| msr::read::<MsrEfer>(proof);
    }

    #[test]
    fn permission_descriptions_preserve_the_image_and_index() {
        fn readable<A>()
        where
            A: Readable,
        {
        }
        fn writable<A>()
        where
            A: Writable,
        {
        }

        readable::<Ro<MsrEfer, u32>>();
        readable::<Rw<MsrEfer, u32>>();
        writable::<Wo<MsrEfer, u32>>();
        writable::<Rw<MsrEfer, u32>>();

        let register = Unaccessible::<MsrEfer, u32>::register(MsrEfer::ADDRESS);

        assert_eq!(register.address(), MsrEfer::ADDRESS);
    }

    /// All register images preserve bits through the generic conversion contract.
    fn image<R>()
    where
        R: Msr,
    {
        assert_eq!(mem::size_of::<R>(), mem::size_of::<u64>());
        assert_eq!(mem::align_of::<R>(), mem::align_of::<u64>());

        for target_value in [u64::MIN, u64::MAX, 1, 1 << 63, 0xa5a5_0123_ffff_8001] {
            assert_eq!(R::new(target_value).raw(), target_value);
        }
    }

    #[test]
    fn every_image_preserves_layout_and_arbitrary_bits() {
        image::<MsrEfer>();
        image::<MsrFsBase>();
        image::<MsrGsBase>();
        image::<MsrKernelGsBase>();
        image::<MsrStar>();
        image::<MsrLStar>();
        image::<MsrCStar>();
        image::<MsrFMask>();
        image::<MsrSysEnterCs>();
        image::<MsrSysEnterSp>();
        image::<MsrSysEnterIp>();
        image::<MsrXfd>();
        image::<MsrXfdErr>();
        image::<MsrXss>();
        image::<MsrFredConfig>();
        image::<MsrFredLevels>();
        image::<MsrFredRsp0>();
        image::<MsrFredRsp1>();
        image::<MsrFredRsp2>();
        image::<MsrFredRsp3>();
        image::<MsrFredSsp1>();
        image::<MsrFredSsp2>();
        image::<MsrFredSsp3>();
    }

    #[test]
    fn image_construction_and_extraction_remain_const() {
        const BITS: u64 = 0xa5a5_0123_ffff_8001;
        const EFER: u64 = MsrEfer::new(BITS).raw();
        const XSS: u64 = MsrXss::new(BITS).raw();
        const FRED: u64 = MsrFredRsp1::new(BITS).raw();

        assert_eq!([EFER, XSS, FRED], [BITS; 3]);
    }
}
