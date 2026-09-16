//! x86 model-specific register representations and access.
//!
//! An MSR is represented by its architectural value type. Its register
//! descriptor records the numeric MSR index and its architectural access mode
//! through `nekor-register`.
//!
//! [`read()`] and [`write()`] perform direct `rdmsr` and `wrmsr` instructions.
//! A const generic selects whether processor execution fences surround the MSR
//! access. Neither operation recovers from architectural faults.
//!
//! # Intel SDM references
//!
//! Intel SDM Vol. 2 defines the `RDMSR` and `WRMSR` instruction behavior,
//! privilege checks, and architectural exceptions. Intel SDM Vol. 4 lists the
//! model-specific register indices and per-register access semantics used here.

use core::{arch, hint, mem};

use nekor_bitwise::prelude::{Bit, BitAt, Counterpart, Field, State, U64High32, U64High32Mut, U64Low32, U64Low32Mut};
use nekor_register::prelude::{
    Ro as RegisterRo, Rw as RegisterRw, Unaccessible as RegisterUnaccessible, Wo as RegisterWo,
};

use crate::x86::{
    fred::{Fred as FredCapability, Level as FredLevel},
    privilege::{Cpl, PrivilegeLevel},
    segmentation::{RawSegmentSelector, SegmentSelector, TableIndicator},
    xstate::Xfd as XfdCapability,
};
#[cfg(target_arch = "x86_64")]
use crate::x86_64::paging::{La, LaMode};
#[cfg(target_arch = "x86_64")]
use crate::x86_64::register::{RawRflags, Rflags};

/// A model-specific register value with a statically known MSR index.
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

/// Irreversibly erased 64-bit representation of a typed model-specific
/// register.
///
/// A `RawMsr` can only be created from an existing [`Msr`] value. The public
/// field intentionally exposes the erased payload for hardware and ABI
/// transports without providing a path back to the original typed value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct RawMsr(pub u64);

impl RawMsr {
    /// Erase a typed MSR into its raw 64-bit representation.
    #[inline]
    #[must_use]
    pub const fn take<R>(value: R) -> Self
    where
        R: Msr,
    {
        // SAFETY: `Msr` requires `R` to be transparent over exactly one `u64`
        // and to have identical size and alignment. `R: Copy` means copying the
        // representation does not invalidate `value`.
        unsafe { mem::transmute_copy(&value) }
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

/// Reads the live x86-64 EFER image under a current-CPL0 proof.
///
/// EFER is part of the architectural long-mode environment and is therefore
/// available whenever this x86-64 implementation executes.
#[cfg(target_arch = "x86_64")]
#[inline]
#[must_use]
pub fn read_efer<T>(_cpl0: &Cpl<0, T>) -> Efer {
    // SAFETY: The proof token supplies the RDMSR privilege requirement and
    // x86-64 execution supplies the architectural EFER availability proof.
    unsafe { read::<Efer, false>() }
}

/// Write an x86-64 EFER image under a current-CPL0 proof.
///
/// The processor-maintained LMA field is cleared from the WRMSR input while
/// every other modeled and unmodeled bit from `value` is preserved.
///
/// # Safety
///
/// The writable EFER fields must describe an architectural state accepted by
/// the current processor. In particular, optional enabled features must be
/// supported and long-mode enable must remain compatible with live paging.
#[cfg(target_arch = "x86_64")]
#[inline]
pub unsafe fn write_efer<T>(_cpl0: &Cpl<0, T>, mut value: Efer) {
    let Efer(ref mut bits) = value;

    let mut active = EferLmaMut::wrap(bits);

    active.const_set(State::Cleared);

    // SAFETY: The caller supplies writable EFER validity. LMA has been removed
    // from the WRMSR input because hardware owns that field.
    unsafe { write::<Efer, false>(value) };
}

/// Read the live `IA32_XSS` supervisor component bitmap under a CPL0 proof.
#[inline]
#[must_use]
pub fn read_xss<T>(_cpl0: &Cpl<0, T>) -> Xss {
    // SAFETY: The proof token supplies the RDMSR privilege requirement. The
    // caller already executes on x86 hardware where the modeled MSR is used.
    unsafe { read::<Xss, false>() }
}

/// Write one `IA32_XSS` supervisor component bitmap under a CPL0 proof.
///
/// # Safety
///
/// Every set component must be supported by the processor and enabled through
/// the architectural XSAVE supervisor-state mechanism required by that bit.
#[inline]
pub unsafe fn write_xss<T>(_cpl0: &Cpl<0, T>, value: Xss) {
    // SAFETY: The caller supplies the component-support and XSAVE-state
    // requirements. The proof token supplies WRMSR privilege.
    unsafe { write::<Xss, false>(value) };
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

/// The "System Call Extensions" enable flag in [`Efer`].
pub type EferSce<'value> = Bit<'value, u64, 0>;

/// The "Long Mode Enable" flag in [`Efer`].
pub type EferLme<'value> = Bit<'value, u64, 8>;

/// The processor-maintained "Long Mode Active" flag in [`Efer`].
pub type EferLma<'value> = Bit<'value, u64, 10>;

/// Internal mutable counterpart used only for saved-state reconstruction.
type EferLmaMut<'value> = <EferLma<'value> as Counterpart>::Mut;

/// The "No-Execute Enable" flag in [`Efer`].
pub type EferNxe<'value> = Bit<'value, u64, 11>;

/// Mutable counterpart to [`EferSce`].
pub type EferSceMut<'value> = <EferSce<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`EferLme`].
pub type EferLmeMut<'value> = <EferLme<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`EferNxe`].
pub type EferNxeMut<'value> = <EferNxe<'value> as Counterpart>::Mut;

/// The `IA32_EFER` extended feature-enable MSR.
///
/// Intel SDM Vol. 4 lists `IA32_EFER` at MSR index `0xC000_0080`. This value
/// representation exposes the architectural SCE, LME, LMA, and NXE fields.
/// LMA is processor-maintained and therefore has no mutable field accessor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar preserves the complete architectural EFER image while the
// public mutation API excludes processor-maintained LMA.
pub struct Efer(u64);

/// EFER image proven suitable for restoring an active long-mode context.
///
/// This is deliberately distinct from [`Efer`]. It can represent the
/// processor-maintained LMA bit for virtualization and saved-state restore, but
/// it does not implement [`Msr`] and therefore cannot be passed to ordinary
/// `WRMSR` mechanisms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The wrapped EFER image always has both LME and LMA set. Construction checks
// LME first and then sets processor-maintained LMA only for this restore-specific proof type.
pub struct LongModeEfer(Efer);

/// Raw transport representation of [`LongModeEfer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct RawLongModeEfer(pub u64);

impl LongModeEfer {
    /// Create an active long-mode EFER image when LME is enabled.
    ///
    /// Returns `None` when `efer` does not enable long mode. The returned image
    /// carries the processor-maintained LMA bit for saved-state restoration.
    #[inline]
    #[must_use]
    pub const fn new(mut efer: Efer) -> Option<Self> {
        match efer.flag_lme().const_state() {
            State::Set => {
                let Efer(ref mut value) = efer;

                let mut lma = EferLmaMut::wrap(value);

                lma.const_set(State::Set);

                Some(Self(efer))
            },
            State::Cleared => None,
        }
    }
}

impl RawLongModeEfer {
    /// Erases a proven active long-mode EFER image for state transport.
    #[inline]
    #[must_use]
    pub const fn take(value: LongModeEfer) -> Self {
        let LongModeEfer(efer) = value;

        let Efer(value) = efer;

        Self(value)
    }
}

impl Efer {
    /// Architectural reset representation used by this model.
    pub const RESET: Self = Self(0);

    /// Determine the system-call enable flag.
    #[inline]
    #[must_use]
    pub const fn flag_sce(&self) -> EferSce<'_> {
        let &Self(ref value) = self;

        EferSce::wrap(value)
    }

    /// Mutably borrow the system-call enable flag.
    #[inline]
    pub const fn flag_sce_mut(&mut self) -> EferSceMut<'_> {
        let &mut Self(ref mut value) = self;

        EferSceMut::wrap(value)
    }

    /// Determine the long-mode enable flag.
    #[inline]
    #[must_use]
    pub const fn flag_lme(&self) -> EferLme<'_> {
        let &Self(ref value) = self;

        EferLme::wrap(value)
    }

    /// Mutably borrow the long-mode enable flag.
    #[inline]
    pub const fn flag_lme_mut(&mut self) -> EferLmeMut<'_> {
        let &mut Self(ref mut value) = self;

        EferLmeMut::wrap(value)
    }

    /// Determine the processor-maintained long-mode active flag.
    #[inline]
    #[must_use]
    pub const fn flag_lma(&self) -> EferLma<'_> {
        let &Self(ref value) = self;

        EferLma::wrap(value)
    }

    /// Determine the no-execute enable flag.
    #[inline]
    #[must_use]
    pub const fn flag_nxe(&self) -> EferNxe<'_> {
        let &Self(ref value) = self;

        EferNxe::wrap(value)
    }

    /// Mutably borrow the no-execute enable flag.
    #[inline]
    pub const fn flag_nxe_mut(&mut self) -> EferNxeMut<'_> {
        let &mut Self(ref mut value) = self;

        EferNxeMut::wrap(value)
    }
}

// SAFETY: `Efer` is a transparent `u64` wrapper with no invalid bit patterns.
unsafe impl Msr for Efer {
    type Access = ReadWrite;

    const ADDRESS: u32 = 0xC000_0080;
}

impl private::Sealed for Efer {}

/// Complete `IA32_XFD` disabled-component bitmap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar preserves every architectural IA32_XFD component bit.
// Safe hardware access additionally requires an XFD capability proof.
pub struct Xfd(u64);

impl Xfd {
    /// Empty XFD component bitmap.
    pub const EMPTY: Self = Self(0);

    /// Preserve one complete XFD component bitmap for architecture transport.
    #[inline]
    #[must_use]
    pub const fn new(bits: u64) -> Self {
        Self(bits)
    }

    /// Return the complete XFD component bitmap.
    #[inline]
    #[must_use]
    pub const fn bits(self) -> u64 {
        let Self(bits) = self;

        bits
    }

    /// Borrow one XFD component selected by its architectural index.
    #[inline]
    #[must_use]
    pub const fn component<const N: usize>(&self) -> Bit<'_, u64, N>
    where
        u64: BitAt<N>,
    {
        let &Self(ref bits) = self;

        Bit::wrap(bits)
    }

    /// Mutably borrow one XFD component selected by its architectural index.
    #[inline]
    pub const fn component_mut<const N: usize>(&mut self) -> <Bit<'_, u64, N> as Counterpart>::Mut
    where
        u64: BitAt<N>,
    {
        let &mut Self(ref mut bits) = self;

        <Bit<'_, u64, N> as Counterpart>::Mut::wrap(bits)
    }
}

// SAFETY: `Xfd` is transparent over `u64` and every hardware bitmap remains representable.
unsafe impl Msr for Xfd {
    type Access = ReadWrite;

    const ADDRESS: u32 = 0x0000_01C4;
}

impl private::Sealed for Xfd {}

/// Complete `IA32_XFD_ERR` component bitmap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar preserves every architectural IA32_XFD_ERR component bit.
// Safe hardware access additionally requires an XFD capability proof.
pub struct XfdErr(u64);

impl XfdErr {
    /// Empty XFD error bitmap.
    pub const EMPTY: Self = Self(0);

    /// Preserve one complete XFD error bitmap reported by hardware.
    #[inline]
    #[must_use]
    pub const fn new(bits: u64) -> Self {
        Self(bits)
    }

    /// Return the complete XFD error bitmap.
    #[inline]
    #[must_use]
    pub const fn bits(self) -> u64 {
        let Self(bits) = self;

        bits
    }

    /// Return whether no XFD component caused the sampled exception.
    #[inline]
    #[must_use]
    pub const fn is_empty(self) -> bool {
        let Self(bits) = self;

        bits == 0
    }

    /// Borrow one XFD error component selected by its architectural index.
    #[inline]
    #[must_use]
    pub const fn component<const N: usize>(&self) -> Bit<'_, u64, N>
    where
        u64: BitAt<N>,
    {
        let &Self(ref bits) = self;

        Bit::wrap(bits)
    }

    /// Mutably borrow one XFD error component selected by its architectural index.
    #[inline]
    pub const fn component_mut<const N: usize>(&mut self) -> <Bit<'_, u64, N> as Counterpart>::Mut
    where
        u64: BitAt<N>,
    {
        let &mut Self(ref mut bits) = self;

        <Bit<'_, u64, N> as Counterpart>::Mut::wrap(bits)
    }
}

// SAFETY: `XfdErr` is transparent over `u64` and every hardware bitmap remains representable.
unsafe impl Msr for XfdErr {
    type Access = ReadWrite;

    const ADDRESS: u32 = 0x0000_01C5;
}

impl private::Sealed for XfdErr {}

/// Read the live `IA32_XFD` bitmap after proving that the processor exposes XFD.
#[inline]
#[must_use]
pub fn read_xfd<T>(_cpl0: &Cpl<0, T>, _xfd: &XfdCapability) -> Xfd {
    // SAFETY: The CPL proof supplies privilege and the XFD proof supplies architectural MSR
    // availability for this processor.
    unsafe { read::<Xfd, false>() }
}

/// Read the live `IA32_XFD_ERR` bitmap after proving that the processor exposes XFD.
#[inline]
#[must_use]
pub fn read_xfd_err<T>(_cpl0: &Cpl<0, T>, _xfd: &XfdCapability) -> XfdErr {
    // SAFETY: The CPL proof supplies privilege and the XFD proof supplies architectural MSR
    // availability for this processor.
    unsafe { read::<XfdErr, false>() }
}

/// Clear the live `IA32_XFD` bitmap after proving that the processor exposes XFD.
#[inline]
pub fn clear_xfd<T>(_cpl0: &Cpl<0, T>, _xfd: &XfdCapability) {
    // SAFETY: The XFD proof establishes register availability and the all-zero bitmap is
    // architecturally valid regardless of supported state-component details.
    unsafe { write::<Xfd, false>(Xfd::EMPTY) };
}

/// Clear the live `IA32_XFD_ERR` bitmap after proving that the processor exposes XFD.
#[inline]
pub fn clear_xfd_err<T>(_cpl0: &Cpl<0, T>, _xfd: &XfdCapability) {
    // SAFETY: The XFD proof establishes register availability and the all-zero bitmap is
    // architecturally valid regardless of supported state-component details.
    unsafe { write::<XfdErr, false>(XfdErr::EMPTY) };
}

/// Current stack level in [`FredConfig`].
pub type FredConfigLevel<'value> = Field<'value, 0, 1, u64>;

/// Reserved FRED configuration bit two.
pub type FredConfigReserved2<'value> = Bit<'value, u64, 2>;

/// Same-stack shadow-stack decrement control in [`FredConfig`].
pub type FredConfigShadow<'value> = Bit<'value, u64, 3>;

/// Reserved FRED configuration bits four and five.
pub type FredConfigReserved4_5<'value> = Field<'value, 4, 5, u64>;

/// Same-stack regular-stack decrement count in [`FredConfig`].
pub type FredConfigRegular<'value> = Field<'value, 6, 8, u64>;

/// Ring-zero maskable-interrupt stack level in [`FredConfig`].
pub type FredConfigInterrupt<'value> = Field<'value, 9, 10, u64>;

/// Reserved FRED configuration bit eleven.
pub type FredConfigReserved11<'value> = Bit<'value, u64, 11>;

/// Event-handler page number in [`FredConfig`].
pub type FredConfigHandler<'value> = Field<'value, 12, 63, u64>;

/// Low page offset used to classify a FRED handler page.
type FredHandlerOffset<'value> = Field<'value, 0, 11, u64>;

/// Mutable ring-zero maskable-interrupt level.
type FredConfigInterruptMut<'value> = <FredConfigInterrupt<'value> as Counterpart>::Mut;

/// Mutable event-handler page number.
type FredConfigHandlerMut<'value> = <FredConfigHandler<'value> as Counterpart>::Mut;

/// The `IA32_FRED_CONFIG` register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): Safe construction clears every reserved field, starts at stack level zero,
// selects no same-stack decrement, and accepts only one canonical page-aligned handler address.
pub struct FredConfig(u64);

impl FredConfig {
    /// Create a baseline FRED configuration for a handler page and stack level.
    ///
    /// Returns `None` when `handler` is not aligned to a 4 KiB page boundary.
    #[cfg(target_arch = "x86_64")]
    #[inline]
    #[must_use]
    pub const fn new(handler: La, interrupt: FredLevel) -> Option<Self> {
        let address = handler.bits();

        match FredHandlerOffset::wrap(&address).const_value() {
            0 => {
                let mut value = u64::MIN;
                let page = FredConfigHandler::wrap(&address).const_value();
                let mut handler = FredConfigHandlerMut::wrap(&mut value);

                handler.const_merge(page);

                let mut interrupt_level = FredConfigInterruptMut::wrap(&mut value);

                interrupt_level.const_merge(interrupt.raw());

                Some(Self(value))
            },
            _ => None,
        }
    }

    /// Return the processor-maintained current stack level.
    #[inline]
    #[must_use]
    pub const fn level(&self) -> FredLevel {
        let &Self(ref value) = self;

        let raw = FredConfigLevel::wrap(value).const_value();

        match FredLevel::lift(raw) {
            Some(level) => level,
            // SAFETY: The source is a two-bit field and Level declares all four encodings.
            None => unsafe { hint::unreachable_unchecked() },
        }
    }

    /// Return the configured ring-zero maskable-interrupt stack level.
    #[inline]
    #[must_use]
    pub const fn interrupt(&self) -> FredLevel {
        let &Self(ref value) = self;

        let raw = FredConfigInterrupt::wrap(value).const_value();

        match FredLevel::lift(raw) {
            Some(level) => level,
            // SAFETY: The source is a two-bit field and Level declares all four encodings.
            None => unsafe { hint::unreachable_unchecked() },
        }
    }

    /// Return the configured handler page when it is canonical under `M`.
    ///
    /// Returns `None` when the encoded handler page is not canonical for the
    /// selected linear-address mode.
    #[cfg(target_arch = "x86_64")]
    #[inline]
    #[must_use]
    pub const fn handler<M>(&self) -> Option<La>
    where
        M: LaMode,
    {
        let &Self(ref value) = self;

        let page = FredConfigHandler::wrap(value).const_value();
        let mut address = u64::MIN;
        let mut handler = FredConfigHandlerMut::wrap(&mut address);

        handler.const_merge(page);

        La::new::<M>(address)
    }

    /// Return the same-stack regular decrement count in 64-byte units.
    #[inline]
    #[must_use]
    pub const fn regular(&self) -> u8 {
        let &Self(ref value) = self;

        FredConfigRegular::wrap(value).const_value()
    }

    /// Return whether same-stack shadow delivery decrements SSP by eight bytes.
    #[inline]
    #[must_use]
    pub const fn shadow(&self) -> State {
        let &Self(ref value) = self;

        FredConfigShadow::wrap(value).const_state()
    }
}

// SAFETY: `FredConfig` is transparent over `u64` and every hardware image remains representable.
unsafe impl Msr for FredConfig {
    type Access = ReadWrite;

    const ADDRESS: u32 = 0x0000_01D4;
}

impl private::Sealed for FredConfig {}
impl private::FredSealed for FredConfig {}
impl FredMsr for FredConfig {}

/// NMI stack-level field in [`FredLevels`].
pub type FredLevelsNmi<'value> = Field<'value, 4, 5, u64>;

/// Double-fault stack-level field in [`FredLevels`].
pub type FredLevelsDf<'value> = Field<'value, 16, 17, u64>;

/// Machine-check stack-level field in [`FredLevels`].
pub type FredLevelsMc<'value> = Field<'value, 36, 37, u64>;

/// Mutable NMI stack-level field.
type FredLevelsNmiMut<'value> = <FredLevelsNmi<'value> as Counterpart>::Mut;

/// Mutable double-fault stack-level field.
type FredLevelsDfMut<'value> = <FredLevelsDf<'value> as Counterpart>::Mut;

/// Mutable machine-check stack-level field.
type FredLevelsMcMut<'value> = <FredLevelsMc<'value> as Counterpart>::Mut;

/// The `IA32_FRED_STKLVLS` exception and NMI stack-level register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The machine-safety constructor leaves every vector at level zero except NMI at
// level one, double fault at level two, and machine check at level three.
pub struct FredLevels(u64);

impl FredLevels {
    /// Construct the revision-one machine-safety level assignment.
    #[inline]
    #[must_use]
    pub const fn machine_safety() -> Self {
        let mut value = u64::MIN;
        let mut nmi = FredLevelsNmiMut::wrap(&mut value);

        nmi.const_merge(FredLevel::One.raw());

        let mut df = FredLevelsDfMut::wrap(&mut value);

        df.const_merge(FredLevel::Two.raw());

        let mut mc = FredLevelsMcMut::wrap(&mut value);

        mc.const_merge(FredLevel::Three.raw());

        Self(value)
    }

    /// Return the NMI stack level.
    #[inline]
    #[must_use]
    pub const fn nmi(&self) -> FredLevel {
        let &Self(ref value) = self;

        let raw = FredLevelsNmi::wrap(value).const_value();

        match FredLevel::lift(raw) {
            Some(level) => level,
            // SAFETY: The source is a two-bit field and Level declares all four encodings.
            None => unsafe { hint::unreachable_unchecked() },
        }
    }

    /// Return the double-fault stack level.
    #[inline]
    #[must_use]
    pub const fn df(&self) -> FredLevel {
        let &Self(ref value) = self;

        let raw = FredLevelsDf::wrap(value).const_value();

        match FredLevel::lift(raw) {
            Some(level) => level,
            // SAFETY: The source is a two-bit field and Level declares all four encodings.
            None => unsafe { hint::unreachable_unchecked() },
        }
    }

    /// Return the machine-check stack level.
    #[inline]
    #[must_use]
    pub const fn mc(&self) -> FredLevel {
        let &Self(ref value) = self;

        let raw = FredLevelsMc::wrap(value).const_value();

        match FredLevel::lift(raw) {
            Some(level) => level,
            // SAFETY: The source is a two-bit field and Level declares all four encodings.
            None => unsafe { hint::unreachable_unchecked() },
        }
    }
}

// SAFETY: `FredLevels` is transparent over `u64` and every hardware image remains representable.
unsafe impl Msr for FredLevels {
    type Access = ReadWrite;

    const ADDRESS: u32 = 0x0000_01D0;
}

impl private::Sealed for FredLevels {}
impl private::FredSealed for FredLevels {}
impl FredMsr for FredLevels {}

/// Low alignment field shared by all FRED regular-stack MSRs.
type FredRspOffset<'value> = Field<'value, 0, 5, u64>;

/// Define one typed FRED regular-stack MSR with shared alignment semantics.
macro_rules! fred_rsp {
    ($name:ident, $address:expr, $level:literal) => {
        #[doc = concat!("The `IA32_FRED_RSP", stringify!($level), "` regular-stack register.")]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(transparent)]
        // NOTE(invariant): Safe construction accepts only canonical 64-byte-aligned regular-stack
        // addresses while trusted hardware reads may preserve any complete register image.
        pub struct $name(u64);

        impl $name {
            /// Architectural zero value used while this stack level is unreachable.
            pub const ZERO: Self = Self(0);

            /// Create a FRED regular-stack pointer from an aligned linear address.
            ///
            /// Returns `None` when `address` is not aligned to the architectural
            /// 64-byte stack boundary.
            #[cfg(target_arch = "x86_64")]
            #[inline]
            #[must_use]
            pub const fn from_la(address: La) -> Option<Self> {
                let bits = address.bits();

                match FredRspOffset::wrap(&bits).const_value() {
                    0 => Some(Self(bits)),
                    _ => None,
                }
            }

            /// Return the regular-stack pointer when it is canonical under `M`.
            ///
            /// Returns `None` when the encoded pointer is not canonical for the
            /// selected linear-address mode.
            #[cfg(target_arch = "x86_64")]
            #[inline]
            #[must_use]
            pub const fn la<M>(&self) -> Option<La>
            where
                M: LaMode,
            {
                let &Self(ref value) = self;

                La::new::<M>(*value)
            }
        }

        // SAFETY: The type is transparent over `u64` and every hardware image remains representable.
        unsafe impl Msr for $name {
            type Access = ReadWrite;

            const ADDRESS: u32 = $address;
        }

        impl private::Sealed for $name {}
        impl private::FredSealed for $name {}
        impl FredMsr for $name {}
    };
}

fred_rsp!(FredRsp0, 0x0000_01CC, 0);
fred_rsp!(FredRsp1, 0x0000_01CD, 1);
fred_rsp!(FredRsp2, 0x0000_01CE, 2);
fred_rsp!(FredRsp3, 0x0000_01CF, 3);

/// Read one FRED architectural MSR after proving processor support and CPL0 execution.
#[inline]
#[must_use]
pub fn read_fred<R, T>(_cpl0: &Cpl<0, T>, _fred: &FredCapability) -> R
where
    R: FredMsr,
    R::Access: Readable,
{
    // SAFETY: The CPL proof supplies privilege and the FRED proof supplies architectural register
    // availability for every implementor of the sealed FredMsr family.
    unsafe { read::<R, false>() }
}

/// Write one FRED architectural MSR after proving processor support and CPL0 execution.
///
/// # Safety
///
/// `value` must satisfy the register-specific architectural requirements and every referenced
/// handler or stack address must remain valid for all FRED transitions that can consume it.
#[inline]
pub unsafe fn write_fred<R, T>(_cpl0: &Cpl<0, T>, _fred: &FredCapability, value: R)
where
    R: FredMsr,
    R::Access: Writable,
{
    // SAFETY: The caller supplies register-specific validity and lifetime while the capability and
    // CPL proofs establish architectural availability and privilege.
    unsafe { write::<R, false>(value) };
}

/// Processor Trace supervisor state in [`Xss`].
pub type XssPt<'value> = Bit<'value, u64, 8>;

/// Process-address-space identifier supervisor state in [`Xss`].
pub type XssPasid<'value> = Bit<'value, u64, 10>;

/// User CET supervisor-managed state in [`Xss`].
pub type XssCetUser<'value> = Bit<'value, u64, 11>;

/// Supervisor CET state in [`Xss`].
pub type XssCetSupervisor<'value> = Bit<'value, u64, 12>;

/// Hardware duty-cycle supervisor state in [`Xss`].
pub type XssHdc<'value> = Bit<'value, u64, 13>;

/// User-interrupt supervisor state in [`Xss`].
pub type XssUintr<'value> = Bit<'value, u64, 14>;

/// Architectural last-branch-record state in [`Xss`].
pub type XssLbr<'value> = Bit<'value, u64, 15>;

/// Hardware-managed performance supervisor state in [`Xss`].
pub type XssHwp<'value> = Bit<'value, u64, 16>;

/// Mutable Processor Trace supervisor state.
pub type XssPtMut<'value> = <XssPt<'value> as Counterpart>::Mut;

/// Mutable process-address-space identifier supervisor state.
pub type XssPasidMut<'value> = <XssPasid<'value> as Counterpart>::Mut;

/// Mutable user CET supervisor-managed state.
pub type XssCetUserMut<'value> = <XssCetUser<'value> as Counterpart>::Mut;

/// Mutable supervisor CET state.
pub type XssCetSupervisorMut<'value> = <XssCetSupervisor<'value> as Counterpart>::Mut;

/// Mutable hardware duty-cycle supervisor state.
pub type XssHdcMut<'value> = <XssHdc<'value> as Counterpart>::Mut;

/// Mutable user-interrupt supervisor state.
pub type XssUintrMut<'value> = <XssUintr<'value> as Counterpart>::Mut;

/// Mutable architectural last-branch-record state.
pub type XssLbrMut<'value> = <XssLbr<'value> as Counterpart>::Mut;

/// Mutable hardware-managed performance supervisor state.
pub type XssHwpMut<'value> = <XssHwp<'value> as Counterpart>::Mut;

/// Complete `IA32_XSS` supervisor component bitmap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar preserves every `IA32_XSS` component bit. Safe field
// inspection never discards unknown component identities.
pub struct Xss(u64);

impl Xss {
    /// Empty supervisor component bitmap.
    pub const EMPTY: Self = Self(0);

    /// Preserve one complete supervisor component bitmap.
    #[inline]
    #[must_use]
    pub const fn new(bits: u64) -> Self {
        Self(bits)
    }

    /// Return the complete supervisor component bitmap.
    #[inline]
    #[must_use]
    pub const fn bits(self) -> u64 {
        let Self(bits) = self;

        bits
    }

    /// Return whether every required component is present.
    #[inline]
    #[must_use]
    pub const fn contains(self, required: Self) -> bool {
        let Self(bits) = self;

        let Self(required) = required;

        bits & required == required
    }

    /// Combine every supervisor component present in either bitmap.
    #[inline]
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        let Self(bits) = self;
        let Self(other) = other;

        Self(bits | other)
    }

    /// Retain only supervisor components present in both bitmaps.
    #[inline]
    #[must_use]
    pub const fn intersection(self, other: Self) -> Self {
        let Self(bits) = self;
        let Self(other) = other;

        Self(bits & other)
    }

    /// Return whether no supervisor component is selected.
    #[inline]
    #[must_use]
    pub const fn is_empty(self) -> bool {
        let Self(bits) = self;

        bits == 0
    }

    /// Borrow one supervisor component selected by its architectural index.
    #[inline]
    #[must_use]
    pub const fn component<const N: usize>(&self) -> Bit<'_, u64, N>
    where
        u64: BitAt<N>,
    {
        let &Self(ref bits) = self;

        Bit::wrap(bits)
    }

    /// Mutably borrow one supervisor component selected by its architectural index.
    #[inline]
    pub const fn component_mut<const N: usize>(&mut self) -> <Bit<'_, u64, N> as Counterpart>::Mut
    where
        u64: BitAt<N>,
    {
        let &mut Self(ref mut bits) = self;

        <Bit<'_, u64, N> as Counterpart>::Mut::wrap(bits)
    }

    /// Borrow Processor Trace supervisor state.
    #[inline]
    #[must_use]
    pub const fn pt(&self) -> XssPt<'_> {
        {
            let &Self(ref bits) = self;

            XssPt::wrap(bits)
        }
    }

    /// Mutably borrow Processor Trace supervisor state.
    #[inline]
    pub const fn pt_mut(&mut self) -> XssPtMut<'_> {
        {
            let &mut Self(ref mut bits) = self;

            <XssPt<'_> as Counterpart>::Mut::wrap(bits)
        }
    }

    /// Borrow PASID supervisor state.
    #[inline]
    #[must_use]
    pub const fn pasid(&self) -> XssPasid<'_> {
        {
            let &Self(ref bits) = self;

            XssPasid::wrap(bits)
        }
    }

    /// Mutably borrow PASID supervisor state.
    #[inline]
    pub const fn pasid_mut(&mut self) -> XssPasidMut<'_> {
        {
            let &mut Self(ref mut bits) = self;

            <XssPasid<'_> as Counterpart>::Mut::wrap(bits)
        }
    }

    /// Borrow user CET supervisor-managed state.
    #[inline]
    #[must_use]
    pub const fn cet_user(&self) -> XssCetUser<'_> {
        {
            let &Self(ref bits) = self;

            XssCetUser::wrap(bits)
        }
    }

    /// Mutably borrow user CET supervisor-managed state.
    #[inline]
    pub const fn cet_user_mut(&mut self) -> XssCetUserMut<'_> {
        {
            let &mut Self(ref mut bits) = self;

            <XssCetUser<'_> as Counterpart>::Mut::wrap(bits)
        }
    }

    /// Borrow supervisor CET state.
    #[inline]
    #[must_use]
    pub const fn cet_supervisor(&self) -> XssCetSupervisor<'_> {
        {
            let &Self(ref bits) = self;

            XssCetSupervisor::wrap(bits)
        }
    }

    /// Mutably borrow supervisor CET state.
    #[inline]
    pub const fn cet_supervisor_mut(&mut self) -> XssCetSupervisorMut<'_> {
        {
            let &mut Self(ref mut bits) = self;

            <XssCetSupervisor<'_> as Counterpart>::Mut::wrap(bits)
        }
    }

    /// Borrow hardware duty-cycle supervisor state.
    #[inline]
    #[must_use]
    pub const fn hdc(&self) -> XssHdc<'_> {
        {
            let &Self(ref bits) = self;

            XssHdc::wrap(bits)
        }
    }

    /// Mutably borrow hardware duty-cycle supervisor state.
    #[inline]
    pub const fn hdc_mut(&mut self) -> XssHdcMut<'_> {
        {
            let &mut Self(ref mut bits) = self;

            <XssHdc<'_> as Counterpart>::Mut::wrap(bits)
        }
    }

    /// Borrow user-interrupt supervisor state.
    #[inline]
    #[must_use]
    pub const fn uintr(&self) -> XssUintr<'_> {
        {
            let &Self(ref bits) = self;

            XssUintr::wrap(bits)
        }
    }

    /// Mutably borrow user-interrupt supervisor state.
    #[inline]
    pub const fn uintr_mut(&mut self) -> XssUintrMut<'_> {
        {
            let &mut Self(ref mut bits) = self;

            <XssUintr<'_> as Counterpart>::Mut::wrap(bits)
        }
    }

    /// Borrow architectural last-branch-record state.
    #[inline]
    #[must_use]
    pub const fn lbr(&self) -> XssLbr<'_> {
        {
            let &Self(ref bits) = self;

            XssLbr::wrap(bits)
        }
    }

    /// Mutably borrow architectural last-branch-record state.
    #[inline]
    pub const fn lbr_mut(&mut self) -> XssLbrMut<'_> {
        {
            let &mut Self(ref mut bits) = self;

            <XssLbr<'_> as Counterpart>::Mut::wrap(bits)
        }
    }

    /// Borrow hardware-managed performance supervisor state.
    #[inline]
    #[must_use]
    pub const fn hwp(&self) -> XssHwp<'_> {
        {
            let &Self(ref bits) = self;

            XssHwp::wrap(bits)
        }
    }

    /// Mutably borrow hardware-managed performance supervisor state.
    #[inline]
    pub const fn hwp_mut(&mut self) -> XssHwpMut<'_> {
        {
            let &mut Self(ref mut bits) = self;

            <XssHwp<'_> as Counterpart>::Mut::wrap(bits)
        }
    }
}

// SAFETY: `Xss` is transparent over `u64` and every hardware bitmap remains representable.
unsafe impl Msr for Xss {
    type Access = ReadWrite;

    const ADDRESS: u32 = 0x0000_0DA0;
}

impl private::Sealed for Xss {}

/// The `IA32_FS_BASE` model-specific register.
///
/// In 64-bit mode this register supplies the base used by the FS segment. Its
/// address value is interpreted according to the active linear-address mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar preserves the complete architectural FS-base MSR image.
pub struct FsBase(u64);

impl FsBase {
    /// Create the register value from a linear address.
    #[cfg(target_arch = "x86_64")]
    #[inline]
    #[must_use]
    pub const fn from_la(address: La) -> Self {
        Self(address.bits())
    }

    /// Return the register value when it is canonical under `M`.
    ///
    /// Returns `None` when the encoded value is not canonical for the selected
    /// linear-address mode.
    #[cfg(target_arch = "x86_64")]
    #[inline]
    #[must_use]
    pub const fn la<M>(&self) -> Option<La>
    where
        M: LaMode,
    {
        let &Self(ref value) = self;

        La::new::<M>(*value)
    }
}

// SAFETY: `FsBase` is a transparent `u64` wrapper with no invalid bit patterns.
unsafe impl Msr for FsBase {
    type Access = ReadWrite;

    const ADDRESS: u32 = 0xC000_0100;
}

impl private::Sealed for FsBase {}

/// The `IA32_GS_BASE` model-specific register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar preserves the complete architectural GS-base MSR image.
pub struct GsBase(u64);

impl GsBase {
    /// Create the register value from a linear address.
    #[cfg(target_arch = "x86_64")]
    #[inline]
    #[must_use]
    pub const fn from_la(address: La) -> Self {
        Self(address.bits())
    }

    /// Return the register value when it is canonical under `M`.
    ///
    /// Returns `None` when the encoded value is not canonical for the selected
    /// linear-address mode.
    #[cfg(target_arch = "x86_64")]
    #[inline]
    #[must_use]
    pub const fn la<M>(&self) -> Option<La>
    where
        M: LaMode,
    {
        let &Self(ref value) = self;

        La::new::<M>(*value)
    }
}

// SAFETY: `GsBase` is a transparent `u64` wrapper with no invalid bit patterns.
unsafe impl Msr for GsBase {
    type Access = ReadWrite;

    const ADDRESS: u32 = 0xC000_0101;
}

impl private::Sealed for GsBase {}

/// The `IA32_KERNEL_GS_BASE` model-specific register.
///
/// `swapgs` exchanges this value with `IA32_GS_BASE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar preserves the complete architectural kernel-GS-base MSR
// image.
pub struct KernelGsBase(u64);

impl KernelGsBase {
    /// Create the register value from a linear address.
    #[cfg(target_arch = "x86_64")]
    #[inline]
    #[must_use]
    pub const fn from_la(address: La) -> Self {
        Self(address.bits())
    }

    /// Return the register value when it is canonical under `M`.
    ///
    /// Returns `None` when the encoded value is not canonical for the selected
    /// linear-address mode.
    #[cfg(target_arch = "x86_64")]
    #[inline]
    #[must_use]
    pub const fn la<M>(&self) -> Option<La>
    where
        M: LaMode,
    {
        let &Self(ref value) = self;

        La::new::<M>(*value)
    }
}

// SAFETY: `KernelGsBase` is a transparent `u64` wrapper with no invalid bit
// patterns.
unsafe impl Msr for KernelGsBase {
    type Access = ReadWrite;

    const ADDRESS: u32 = 0xC000_0102;
}

impl private::Sealed for KernelGsBase {}

/// The `IA32_LSTAR` system-call target register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar preserves the complete architectural LSTAR MSR image.
pub struct LStar(u64);

impl LStar {
    /// Create the register value from a linear address.
    #[cfg(target_arch = "x86_64")]
    #[inline]
    #[must_use]
    pub const fn from_la(address: La) -> Self {
        Self(address.bits())
    }

    /// Return the register value when it is canonical under `M`.
    ///
    /// Returns `None` when the encoded value is not canonical for the selected
    /// linear-address mode.
    #[cfg(target_arch = "x86_64")]
    #[inline]
    #[must_use]
    pub const fn la<M>(&self) -> Option<La>
    where
        M: LaMode,
    {
        let &Self(ref value) = self;

        La::new::<M>(*value)
    }
}

// SAFETY: `LStar` is a transparent `u64` wrapper with no invalid bit patterns.
unsafe impl Msr for LStar {
    type Access = ReadWrite;

    const ADDRESS: u32 = 0xC000_0082;
}

impl private::Sealed for LStar {}

/// The `IA32_CSTAR` compatibility-mode system-call target register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar preserves the complete architectural CSTAR MSR image.
pub struct CStar(u64);

impl CStar {
    /// Create the register value from a linear address.
    #[cfg(target_arch = "x86_64")]
    #[inline]
    #[must_use]
    pub const fn from_la(address: La) -> Self {
        Self(address.bits())
    }

    /// Return the register value when it is canonical under `M`.
    ///
    /// Returns `None` when the encoded value is not canonical for the selected
    /// linear-address mode.
    #[cfg(target_arch = "x86_64")]
    #[inline]
    #[must_use]
    pub const fn la<M>(&self) -> Option<La>
    where
        M: LaMode,
    {
        let &Self(ref value) = self;

        La::new::<M>(*value)
    }
}

// SAFETY: `CStar` is a transparent `u64` wrapper with no invalid bit patterns.
unsafe impl Msr for CStar {
    type Access = ReadWrite;

    const ADDRESS: u32 = 0xC000_0083;
}

impl private::Sealed for CStar {}

/// Kernel selector field in [`Star`].
type StarKernelSelector<'value> = Field<'value, 32, 47, u64>;

/// Mutable counterpart to [`StarKernelSelector`].
type StarKernelSelectorMut<'value> = <StarKernelSelector<'value> as Counterpart>::Mut;

/// User selector field in [`Star`].
type StarUserSelector<'value> = Field<'value, 48, 63, u64>;

/// Mutable counterpart to [`StarUserSelector`].
type StarUserSelectorMut<'value> = <StarUserSelector<'value> as Counterpart>::Mut;

/// The `IA32_STAR` long-mode system-call selector register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): Safe construction stores a ring-zero adjacent code and data
// selector pair plus an adjacent ring-three data and code pair from the GDT.
// Hardware reads may still preserve any complete architectural STAR image.
pub struct Star(u64);

impl Star {
    /// Constructs STAR from the selectors used by long-mode SYSCALL and SYSRET.
    #[inline]
    #[must_use]
    pub fn long_mode(
        kernel_code: SegmentSelector,
        kernel_data: SegmentSelector,
        user_data: SegmentSelector,
        user_code: SegmentSelector,
    ) -> Option<Self> {
        let kernel_code_raw = kernel_code.raw().get();
        let kernel_data_raw = kernel_data.raw().get();
        let user_data_raw = user_data.raw().get();
        let user_code_raw = user_code.raw().get();
        let tables_valid = matches!(kernel_code.table(), TableIndicator::Gdt)
            && matches!(kernel_data.table(), TableIndicator::Gdt)
            && matches!(user_data.table(), TableIndicator::Gdt)
            && matches!(user_code.table(), TableIndicator::Gdt);
        let privileges_valid = matches!(kernel_code.privilege(), PrivilegeLevel::Ring0)
            && matches!(kernel_data.privilege(), PrivilegeLevel::Ring0)
            && matches!(user_data.privilege(), PrivilegeLevel::Ring3)
            && matches!(user_code.privilege(), PrivilegeLevel::Ring3);
        let pairs_valid = kernel_code_raw.checked_add(8) == Some(kernel_data_raw)
            && user_data_raw.checked_add(8) == Some(user_code_raw);
        let user_base = user_data_raw.checked_sub(8);

        match (tables_valid, privileges_valid, pairs_valid, user_base) {
            (true, true, true, Some(user_base)) => {
                let mut kernel = kernel_code.raw();
                let mut kernel_privilege = kernel.requested_privilege_level_mut();

                kernel_privilege.const_merge(0);

                let mut user = RawSegmentSelector::new(user_base);
                let mut user_privilege = user.requested_privilege_level_mut();

                user_privilege.const_merge(0);

                let mut value = u64::MIN;
                let mut kernel_field = StarKernelSelectorMut::wrap(&mut value);

                kernel_field.const_merge(kernel.get());

                let mut user_field = StarUserSelectorMut::wrap(&mut value);

                user_field.const_merge(user.get());

                Some(Self(value))
            },
            _ => None,
        }
    }
}

// SAFETY: `Star` is transparent over `u64` and every hardware image remains
// representable. The safe constructor proves the long-mode selector relation.
unsafe impl Msr for Star {
    type Access = ReadWrite;

    const ADDRESS: u32 = 0xC000_0081;
}

impl private::Sealed for Star {}

/// The `IA32_FMASK` system-call RFLAGS mask register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar is derived from a typed RFLAGS image for
// safe construction while every hardware-provided mask image is representable.
pub struct FMask(u64);

impl FMask {
    /// Constructs the mask from the RFLAGS fields cleared on SYSCALL entry.
    #[cfg(target_arch = "x86_64")]
    #[inline]
    #[must_use]
    pub const fn from_rflags(flags: Rflags) -> Self {
        let RawRflags(value) = RawRflags::take(flags);

        Self(value)
    }
}

// SAFETY: `FMask` is transparent over `u64` and every mask image is valid.
unsafe impl Msr for FMask {
    type Access = ReadWrite;

    const ADDRESS: u32 = 0xC000_0084;
}

impl private::Sealed for FMask {}

/// The `IA32_SYSENTER_ESP` register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar preserves the complete architectural SYSENTER stack-pointer
// MSR image.
pub struct SysEnterSp(u64);

impl SysEnterSp {
    /// Create the register value from a linear address.
    #[cfg(target_arch = "x86_64")]
    #[inline]
    #[must_use]
    pub const fn from_la(address: La) -> Self {
        Self(address.bits())
    }

    /// Return the register value when it is canonical under `M`.
    ///
    /// Returns `None` when the encoded value is not canonical for the selected
    /// linear-address mode.
    #[cfg(target_arch = "x86_64")]
    #[inline]
    #[must_use]
    pub const fn la<M>(&self) -> Option<La>
    where
        M: LaMode,
    {
        let &Self(ref value) = self;

        La::new::<M>(*value)
    }
}

// SAFETY: `SysEnterSp` is a transparent `u64` wrapper with no invalid bit
// patterns.
unsafe impl Msr for SysEnterSp {
    type Access = ReadWrite;

    const ADDRESS: u32 = 0x0000_0175;
}

impl private::Sealed for SysEnterSp {}

/// The `IA32_SYSENTER_EIP` register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar preserves the complete architectural SYSENTER
// instruction-pointer MSR image.
pub struct SysEnterIp(u64);

impl SysEnterIp {
    /// Create the register value from a linear address.
    #[cfg(target_arch = "x86_64")]
    #[inline]
    #[must_use]
    pub const fn from_la(address: La) -> Self {
        Self(address.bits())
    }

    /// Return the register value when it is canonical under `M`.
    ///
    /// Returns `None` when the encoded value is not canonical for the selected
    /// linear-address mode.
    #[cfg(target_arch = "x86_64")]
    #[inline]
    #[must_use]
    pub const fn la<M>(&self) -> Option<La>
    where
        M: LaMode,
    {
        let &Self(ref value) = self;

        La::new::<M>(*value)
    }
}

// SAFETY: `SysEnterIp` is a transparent `u64` wrapper with no invalid bit
// patterns.
unsafe impl Msr for SysEnterIp {
    type Access = ReadWrite;

    const ADDRESS: u32 = 0x0000_0176;
}

impl private::Sealed for SysEnterIp {}

#[cfg(test)]
mod tests {
    use nekor_bitwise::prelude::State;
    use nekor_register::prelude::{Ro, Rw, Unaccessible as RegisterUnaccessible, Wo};

    use super::{
        Efer, FMask, FredConfig, FredLevels, FredRsp0, FredRsp1, FredRsp2, FredRsp3, FsBase, GsBase, KernelGsBase,
        LStar, LongModeEfer, Msr, RawLongModeEfer, RawMsr, ReadOnly, ReadWrite, Star, SysEnterIp, SysEnterSp,
        Unaccessible, WriteOnly, Xfd, XfdErr, Xss,
    };
    use crate::{
        x86::fred::Level as FredLevel,
        x86_64::paging::{La, La48},
    };

    #[derive(Clone, Copy)]
    #[repr(transparent)]
    struct TestRo(pub u64);

    #[derive(Clone, Copy)]
    #[repr(transparent)]
    struct TestRw(pub u64);

    #[derive(Clone, Copy)]
    #[repr(transparent)]
    struct TestWo(pub u64);

    #[derive(Clone, Copy)]
    #[repr(transparent)]
    struct TestNone(pub u64);

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
        let mut value = Efer::RESET;

        value.flag_nxe_mut().const_set(State::Set);

        let RawMsr(value) = RawMsr::take(value);

        assert_eq!(value, 0x800);
    }

    #[test]
    fn msr_descriptors_preserve_architectural_indices() {
        assert_eq!(Efer::REGISTER.address(), <Efer as Msr>::ADDRESS as usize);
        assert_eq!(FsBase::REGISTER.address(), 0xC000_0100);
        assert_eq!(GsBase::REGISTER.address(), 0xC000_0101);
        assert_eq!(KernelGsBase::REGISTER.address(), 0xC000_0102);
        assert_eq!(LStar::REGISTER.address(), 0xC000_0082);
        assert_eq!(Star::REGISTER.address(), 0xC000_0081);
        assert_eq!(FMask::REGISTER.address(), 0xC000_0084);
        assert_eq!(FredConfig::REGISTER.address(), 0x1D4);
        assert_eq!(FredLevels::REGISTER.address(), 0x1D0);
        assert_eq!(FredRsp0::REGISTER.address(), 0x1CC);
        assert_eq!(FredRsp1::REGISTER.address(), 0x1CD);
        assert_eq!(FredRsp2::REGISTER.address(), 0x1CE);
        assert_eq!(FredRsp3::REGISTER.address(), 0x1CF);
        assert_eq!(Xfd::REGISTER.address(), 0x1C4);
        assert_eq!(XfdErr::REGISTER.address(), 0x1C5);
        assert_eq!(Xss::REGISTER.address(), 0xDA0);
        assert_eq!(SysEnterSp::REGISTER.address(), 0x175);
        assert_eq!(SysEnterIp::REGISTER.address(), 0x176);
    }

    #[test]
    fn fred_config_preserves_handler_and_baseline_level_policy() {
        let handler = La::new::<La48>(0xffff_ffff_8000_1000).expect("fixture handler page must be canonical");
        let config = FredConfig::new(handler, FredLevel::Zero).expect("page-aligned handler must configure FRED");

        assert_eq!(config.level(), FredLevel::Zero);
        assert_eq!(config.interrupt(), FredLevel::Zero);
        assert_eq!(config.regular(), 0);
        assert_eq!(config.shadow(), State::Cleared);
        assert_eq!(config.handler::<La48>(), Some(handler));

        let unaligned = La::new::<La48>(handler.bits() + 8).expect("fixture address remains canonical");

        assert!(FredConfig::new(unaligned, FredLevel::Zero).is_none());
    }

    #[test]
    fn fred_machine_safety_levels_are_distinct() {
        let levels = FredLevels::machine_safety();

        assert_eq!(levels.nmi(), FredLevel::One);
        assert_eq!(levels.df(), FredLevel::Two);
        assert_eq!(levels.mc(), FredLevel::Three);
    }

    #[test]
    fn fred_regular_stacks_require_sixty_four_byte_alignment() {
        let aligned = La::new::<La48>(0xffff_ffff_8000_2040).expect("fixture stack top must be canonical");
        let unaligned = La::new::<La48>(aligned.bits() + 8).expect("fixture address remains canonical");
        let rsp = FredRsp1::from_la(aligned).expect("aligned FRED stack top must validate");

        assert_eq!(rsp.la::<La48>(), Some(aligned));
        assert!(FredRsp1::from_la(unaligned).is_none());
    }

    #[test]
    fn long_mode_efer_requires_enable_and_sets_active_restore_state() {
        let inactive = LongModeEfer::new(Efer::RESET);
        let mut enabled = Efer::RESET;

        enabled.flag_lme_mut().const_set(State::Set);

        let active = LongModeEfer::new(enabled);
        let raw = active.map(RawLongModeEfer::take);

        assert_eq!(inactive, None);
        assert_eq!(raw, Some(RawLongModeEfer(0x500)));
    }

    #[test]
    fn efer_fields_are_typed_independently_of_access() {
        let mut efer = Efer::RESET;

        efer.flag_sce_mut().const_set(State::Set);
        efer.flag_lme_mut().const_set(State::Set);
        efer.flag_nxe_mut().const_set(State::Set);

        assert_eq!(efer.flag_sce().state(), State::Set);
        assert_eq!(efer.flag_lme().state(), State::Set);
        assert_eq!(efer.flag_lma().state(), State::Cleared);
        assert_eq!(efer.flag_nxe().state(), State::Set);
    }

    #[test]
    fn xfd_bitmaps_preserve_component_identity() {
        let mut xfd = Xfd::EMPTY;
        let mut error = XfdErr::EMPTY;

        xfd.component_mut::<18>().const_set(State::Set);
        error.component_mut::<18>().const_set(State::Set);

        assert_eq!(xfd.component::<18>().const_state(), State::Set);
        assert_eq!(error.component::<18>().const_state(), State::Set);
        assert!(!error.is_empty());
        assert!(XfdErr::EMPTY.is_empty());
    }

    #[test]
    fn xss_components_are_typed_independently() {
        let mut xss = Xss::EMPTY;

        xss.cet_supervisor_mut().const_set(State::Set);
        xss.lbr_mut().const_set(State::Set);

        assert_eq!(xss.cet_supervisor().const_state(), State::Set);
        assert_eq!(xss.lbr().const_state(), State::Set);
        assert_eq!(xss.hdc().const_state(), State::Cleared);
    }
}
