//! Atomic implementation diversion.

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use core::sync::atomic::AtomicU64;
use core::{arch, convert, ffi, hint::unreachable_unchecked, marker, mem, pin::Pin, ptr, sync::atomic::Ordering};

use nekor_bitwise::prelude::{Counterpart, Field};

use crate::patch::{
    choose::Chosen,
    delegate::{Delegated, Delegator, Eventual},
    pod::Pod,
};

/// The input type associated with a delegated implementation.
type DelegatedInput<D> = <<D as Delegator>::Target as Delegated>::Input;

/// The output type associated with a delegated implementation.
type DelegatedOutput<D> = <<D as Delegator>::Target as Delegated>::Output;

/// The patchsite type associated with a delegator.
type PatchsiteFor<D> = Patchsite<D, DelegatedInput<D>, DelegatedOutput<D>>;

/// A static reference to a delegator's patchsite.
type StaticPatchsite<D> = Pin<&'static PatchsiteFor<D>>;

/// The identity function type used to reserve patchsite storage.
type PatchIdentity<D, I, O> = fn() -> (D, I, O);

/// The architecture-specific diversion instruction used for detours to a
/// [`Delegated`] implementation.
#[derive(Debug, Hash)]
#[repr(transparent)]
// NOTE(invariant): The word contains the architecture-specific diversion instruction image.
pub struct Diversion(
    /// NOTE(x86): We use `jmp rel32` in `IA-32{,e}`, which is 5-bytes in size.
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    u64,
);

/// An uninhabited enum to serve as a namespace for `x86{,-64}`-specific
/// constants.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub enum X86 {
    /// An impossible marker variant. `X86` is used only as a namespace.
    __Variant(convert::Infallible),
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
impl X86 {
    /// The end bit index for the 32-bit relative immediate operand of a `jmp
    /// rel32` instruction.
    const JMP_REL32_OPERAND_END: usize = Self::JMP_REL32_OPERAND_START + u32::BITS as usize - 1;
    /// The start bit index for the 32-bit relative immediate operand of a `jmp
    /// rel32` instruction.
    const JMP_REL32_OPERAND_START: usize = mem::size_of_val(&Self::OPCODE_JMP_REL32) * u8::BITS as usize;
    /// The associated template instruction for the `x86` architecture.
    ///
    /// This is to be used with the [`JmpRel32`] type.
    const JMP_REL32_UD2_NOP_TEMPLATE: u64 = (Self::OPCODE_NOP as u64)
        << (u8::BITS as usize * (Self::MNEMONIC_JMP_REL32_SIZE + mem::size_of::<u16>()))
        | (Self::OPCODE_UD2 as u64) << (u8::BITS as usize * Self::MNEMONIC_JMP_REL32_SIZE)
        | (Self::OPCODE_JMP_REL32 as u64);
    /// The size of a full `jmp rel32` instruction mnemonic.
    const MNEMONIC_JMP_REL32_SIZE: usize = mem::size_of_val(&Self::OPCODE_JMP_REL32) + mem::size_of::<u32>();
    /// The Operation Code for the `x86` `int3` instruction mnemonic.
    ///
    /// Reference: <https://www.felixcloutier.com/x86/intn:into:int3:int1>
    const OPCODE_INT3: u8 = 0xCC;
    /// The Operation Code for the `x86` `jmp rel32` instruction mnemonic.
    ///
    /// Reference: <https://www.felixcloutier.com/x86/jmp>
    const OPCODE_JMP_REL32: u8 = 0xE9;
    /// The Operation Code for the `x86` `nop` instruction mnemonic.
    ///
    /// Reference: <https://www.felixcloutier.com/x86/nop>
    const OPCODE_NOP: u8 = 0x90;
    /// The Operation Code for the `x86` `ud2` instruction mnemonic.
    ///
    /// This resides in the non-default `0F` extended Operation Code map.
    ///
    /// Reference: <https://www.felixcloutier.com/x86/ud>
    const OPCODE_UD2: u16 = 0x0B0F;
}

/// The [`Field`] for the 32-bit relative immediate operand of a
/// `jmp rel32` instruction.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
type JmpRel32<'a> = Field<'a, { X86::JMP_REL32_OPERAND_START }, { X86::JMP_REL32_OPERAND_END }, u64, u32>;

/// The mutable counterpart to [`JmpRel32`].
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
type JmpRel32Mut<'a> = <JmpRel32<'a> as Counterpart>::Mut;

/// A template of the diversion instruction required for the establishment of a
/// [`Delegated`] implementation.
#[derive(Debug)]
#[repr(C)]
// NOTE(invariant): The function pointer has the delegated input and output types associated with
// `D`.
pub struct Template<D>(
    // NOTE(rationale): The sibling selection module builds templates for this patch module.
    pub(crate) extern "sysv64-unwind" fn(DelegatedInput<D>) -> DelegatedOutput<D>,
    // NOTE(rationale): The sibling selection module constructs the typed template marker.
    pub(crate) marker::PhantomData<fn() -> D>,
)
where
    D: Delegator;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
impl<D> Template<D> where D: Delegator {}

impl<D> Template<D>
where
    D: Delegator,
{
    /// Assemble the architecture-specific instruction required for the delegate
    /// diversion.
    #[inline]
    unsafe fn assemble(self, target_patchsite: StaticPatchsite<D>) -> Diversion {
        let Self(delegate_address, ..) = self;

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        return {
            let base_address = ptr::from_ref(target_patchsite.get_ref()).addr();

            // NOTE: This is the architectural Program Counter value for `x86`.
            let base_address = base_address.wrapping_add(X86::MNEMONIC_JMP_REL32_SIZE);

            let target_address = delegate_address as usize;

            // NOTE: The small code model keeps the target within the signed
            // 32-bit displacement range required by `jmp rel32`.
            let relative_address = if target_address >= base_address {
                i64::try_from(target_address - base_address)
            } else {
                i64::try_from(base_address - target_address).map(|offset| -offset)
            };

            let relative_address = relative_address.and_then(i32::try_from);

            let Ok(relative_address) = relative_address else {
                // SAFETY: The small code model guarantees that the linked
                // addresses differ by a signed 32-bit displacement.
                unsafe { unreachable_unchecked() }
            };

            let relative_address = u32::from_ne_bytes(relative_address.to_ne_bytes());

            let mut encoded_instr = X86::JMP_REL32_UD2_NOP_TEMPLATE;

            JmpRel32Mut::wrap(&mut encoded_instr).merge(relative_address);

            Diversion(encoded_instr)
        };
    }
}

/// A patched [`Delegator`] and [`Delegated`] pair for some implementation.
#[derive(Debug, Hash)]
#[repr(transparent)]
// NOTE(invariant): The function pointer implements the input and output contract of delegator `D`.
pub struct Patched<D, I, O>(extern "sysv64-unwind" fn(I) -> O, marker::PhantomData<fn() -> D>)
where
    D: Delegator,
    D::Target: Delegated<Input = I, Output = O>;

/// The patchsite for a [`Delegator`] and [`Delegated`] pair.
#[derive(Debug)]
#[repr(transparent)]
// NOTE(invariant): Pinning fixes the address of this typed atomic diversion word.
pub struct Patchsite<D, I, O>(
    /// NOTE(x86): We use `jmp rel32` in `IA-32{,e}`, which is 5-bytes in size.
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    AtomicU64,
    marker::PhantomPinned,
    marker::PhantomData<PatchIdentity<D, I, O>>,
)
where
    D: Delegator,
    D::Target: Delegated<Input = I, Output = O>;

impl<D, I, O> Patchsite<D, I, O>
where
    D: Delegator,
    D::Target: Delegated<Input = I, Output = O>,
{
    /// Attempt to patch the [`Delegator`] with the [`Delegated`]
    /// implementation.
    ///
    /// For contention measure, this performs a single-try atomic
    /// *Compare-and-Swap* operation. In the case of contention (but not
    /// architectural spurious failure), it will *NOT* retry the operation.
    ///
    /// # Safety
    ///
    /// The [`Eventual`] contract permits callers to continue executing either
    /// diversion while the new instruction stream becomes visible. The caller
    /// must ensure that patch storage is writable and that the architecture's
    /// cross-modifying-code publication requirements are met.
    /// The caller must also admit the selected delegate on every CPU that may
    /// execute it. Any previous delegate must remain safe to execute while
    /// its diversion can still be observed.
    ///
    /// # Errors
    ///
    /// Returns the previously installed diversion when another contender
    /// changes the patchsite before this operation completes.
    #[inline]
    pub unsafe fn stale(self: Pin<&'static Self>, target_chosen: Chosen<D, I, O>) -> Result<Diversion, Diversion>
    where
        D: Eventual,
    {
        let Self(atomic_variable, ..) = self.get_ref();

        let target_template = Chosen::template(target_chosen);

        // SAFETY: Patch storage and delegate code use the small code model, so
        // their relative displacement is representable by `jmp rel32`.
        let Diversion(diversion_sequence) = unsafe { Template::assemble(target_template, self) };

        atomic_variable
            .compare_exchange(
                atomic_variable.load(Ordering::Acquire),
                diversion_sequence,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map(Diversion)
            .map_err(Diversion)
    }

    // TODO: Finish modeling the CMC-Publish and CMC-Acquire model.
}

/// A marker type to serve as an umbrella type for [`Delegator`] patchsites.
// NOTE(invariant): The uninhabited field prevents constructing an unsupported patch namespace
// value.
pub struct Patch<T, P>(
    // FIXME(unstable): Use the `never` (`!`) type explicitly here when stable.
    convert::Infallible,
    marker::PhantomData<fn() -> (T, P)>,
)
where
    T: Delegator,
    P: Pod;

#[expect(clippy::missing_inline_in_public_items, reason = "naked functions cannot use inline")]
impl<T, P> Patch<T, P>
where
    T: Delegator,
    P: Pod,
{
    /// Trampoline directly into the target [`Delegated::trampoline`] for
    /// this [`Patch`].
    ///
    /// # Safety
    ///
    /// The installed delegate must satisfy the input and output contract of
    /// `T`. The calling CPU must satisfy its architectural requirements.
    ///
    /// The patch slot must divert to resident delegate code.
    ///
    /// Assembly callers must supply the System V ABI state for `T::Target`.
    /// The jump preserves that state and the caller's return address.
    #[unsafe(link_section = concat!(env!("KERNEL_PATCH_STORAGE_SECTION"), ".delegate.trampoline"))]
    #[unsafe(naked)]
    pub unsafe extern "sysv64-unwind" fn trampoline(
        _target_value: <T::Target as Delegated>::Input,
    ) -> <T::Target as Delegated>::Output {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        arch::naked_asm!(
            concat!("jmp", ' ', env!("KERNEL_PATCH_PATCHSITE_SYMBOL"), "_{target_symbol}_", "patchsite"),
            target_symbol = sym Self::patchsite,
            options(att_syntax)
        );
    }

    /// Run the target [`Delegated::implementation`] for this [`Patch`].
    #[inline]
    pub fn run(target_value: <T::Target as Delegated>::Input) -> <T::Target as Delegated>::Output {
        // SAFETY: Every diversion targets an adapter with this System V unwind signature.
        //
        // Installation preserves the delegate's admission and lifetime contract.
        unsafe { Self::trampoline(target_value) }
    }

    /// The patchsite of the [`Delegator`] and [`Pod`] association.
    ///
    /// # Safety
    ///
    /// This associated function must never be called.
    #[unsafe(link_section = concat!(env!("KERNEL_PATCH_STORAGE_SECTION"), ".delegate.offset"))]
    #[unsafe(naked)]
    // NOTE: The platform linker scripts place all patch subsections in
    // page-isolated writable storage.
    //
    // The loader must back this storage with RAM, including the relative
    // pointers and trampolines.
    pub unsafe extern "custom" fn patchsite() {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            use nekor_aal_agnostic::rel_ptr::RelPtrMut;
            use nekor_aal_cache::line::Cacheline;

            /// A helper macro to name the patchsite symbol.
            macro_rules! patchsite {
                () => {
                    concat!(
                        env!("KERNEL_PATCH_PATCHSITE_SYMBOL"),
                        "_{patchsite_symbol}_",
                        "patchsite"
                    )
                };
            }

            arch::naked_asm!(
                ".balign {pointer_alignment}",
                ".{pointer_size}byte 2f - .",
                concat!(
                    ".pushsection",
                    ' ',
                    env!("KERNEL_PATCH_STORAGE_SECTION"),
                    ".delegate",
                    // NOTE: This is a RWX section, which will reside in the isolated program segment dedicated to patchzones.
                    //
                    // However, in systems with W^X enforcement, this is re-mapped as read-execute but pagetable-aliased to another read-write page for hotpatching.
                    ", \"awx\", @progbits"
                ),

                concat!(".global", ' ', patchsite!()),
                concat!(".type", ' ', patchsite!(), ',', "@function"),

                ".balign {cacheline_alignment}, {X86_OPCODE_INT3}",

                "2:",
                concat!(patchsite!(), ':'),

                // NOTE: We emit this jump manually too, as the assembler may emit relative jumps with smaller immediates, which would break the whole thing.
                ".byte {X86_OPCODE_JMP_REL32}",
                concat!(
                    ".4byte {default_symbol}",
                    " - (",
                    patchsite!(),
                    " + {X86_MNEMONIC_JMP_REL32_SIZE})"
                ),
                "ud2",
                "nop",

                concat!(".size", ' ', patchsite!(), ',' ,' ', ". - ", patchsite!()),

                ".popsection",
                pointer_alignment = const mem::align_of::<RelPtrMut<ffi::c_void>>(),
                pointer_size = const mem::size_of::<RelPtrMut<ffi::c_void>>(),

                cacheline_alignment = const mem::align_of::<Cacheline>(),

                patchsite_symbol = sym Self::patchsite,

                default_symbol = sym <T::Target as Delegated>::trampoline,

                X86_OPCODE_JMP_REL32 = const X86::OPCODE_JMP_REL32,
                X86_MNEMONIC_JMP_REL32_SIZE = const X86::MNEMONIC_JMP_REL32_SIZE,

                X86_OPCODE_INT3 = const X86::OPCODE_INT3,

                options(att_syntax)
            )
        }
    }
}

#[cfg(test)]
#[cfg(target_arch = "x86_64")]
mod tests {
    use core::{
        any::Any,
        sync::atomic::{AtomicBool, Ordering},
    };
    use std::panic;

    use super::{Patch, Template, X86};
    use crate::patch::{
        choose::Chosen,
        delegate::{Delegated, Single},
    };

    /// A delegate with aggregate arguments and an indirect aggregate return.
    #[derive(Clone, Copy)]
    struct Aggregate;

    // SAFETY: Reversing an owned array has no architectural prerequisites.
    unsafe impl Delegated for Aggregate {
        type Input = [u64; 9];
        type Output = [u64; 9];

        #[inline(never)]
        fn implementation(mut target_value: Self::Input) -> Self::Output {
            target_value.reverse();

            target_value
        }
    }

    /// A delegate that resumes a Rust unwind supplied by its caller.
    #[derive(Clone, Copy)]
    struct Unwind;

    /// A delegate whose adapter differs observably from its Rust implementation.
    #[derive(Clone, Copy)]
    struct Specialized;

    // SAFETY: Both entry points operate on an owned integer without architectural prerequisites.
    unsafe impl Delegated for Specialized {
        type Input = u64;
        type Output = u64;

        #[inline]
        fn implementation(target_value: Self::Input) -> Self::Output {
            target_value
        }

        #[inline]
        extern "sysv64-unwind" fn trampoline(target_value: Self::Input) -> Self::Output {
            target_value.wrapping_add(1)
        }
    }

    /// Records whether unwinding runs the caller's destructor.
    struct Cleanup<'state>(pub &'state AtomicBool);

    impl Drop for Cleanup<'_> {
        #[inline]
        fn drop(&mut self) {
            let Self(cleaned) = self;

            cleaned.store(true, Ordering::Relaxed);
        }
    }

    // SAFETY: Resuming the owned panic payload requires no additional caller guarantees.
    unsafe impl Delegated for Unwind {
        type Input = Box<dyn Any + Send>;
        type Output = ();

        #[inline(never)]
        fn implementation(target_value: Self::Input) -> Self::Output {
            panic::resume_unwind(target_value)
        }
    }

    #[test]
    fn sysv_trampoline_preserves_aggregate_call_state() {
        let target_value = [1, 2, 3, 4, 5, 6, 7, 8, 9];
        let result = Patch::<Single<Aggregate, ()>, ()>::run(target_value);

        assert_eq!(result, [9, 8, 7, 6, 5, 4, 3, 2, 1]);

        let patchsite = Chosen::<Single<Aggregate, ()>, [u64; 9], [u64; 9]>::patchsite();
        let super::Patchsite(diversion, ..) = patchsite.get_ref();
        let bytes = diversion.load(core::sync::atomic::Ordering::Relaxed).to_ne_bytes();

        assert_eq!(bytes[0], X86::OPCODE_JMP_REL32);
    }

    #[test]
    fn initial_and_selected_diversions_use_the_adapter() {
        let initial = Patch::<Single<Specialized, ()>, ()>::run(41);
        let chosen = Chosen::<Single<Specialized, ()>, u64, u64>::delegated::<Specialized>();
        let Template(adapter, ..) = chosen.template();
        let selected = adapter(41);

        assert_eq!(initial, 42);
        assert_eq!(selected, 42);
    }

    #[test]
    fn delegate_unwinds_to_the_original_rust_caller() {
        let cleaned = AtomicBool::new(false);
        let result = panic::catch_unwind(|| {
            let _cleanup = Cleanup(&cleaned);

            Patch::<Single<Unwind, ()>, ()>::run(Box::new(73_u32));
        });

        let payload = result.expect_err("the delegate must resume the supplied unwind");

        assert_eq!(payload.downcast_ref::<u32>(), Some(&73));
        assert!(cleaned.load(Ordering::Relaxed));
    }
}
