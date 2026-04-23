//! Atomic implementation diversion.

use core::{arch, convert, ffi, marker, mem, pin::Pin, sync::atomic::Ordering};

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use core::sync::atomic::AtomicU64;

use crate::patch::{
    choose::Chosen,
    delegate::{Delegated, Delegator},
    pod::Pod,
};

use nekor_bitwise::prelude::{Counterpart, Field};

/// The architecture-specific diversion instruction used for detours to a
/// [`Delegated`] implementation.
#[derive(Debug, Hash)]
#[repr(transparent)]
pub struct Diversion(
    /// NOTE(x86): We use `jmp rel32` in `IA-32{,e}`, which is 5-bytes in size.
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    u64,
);

/// An uninhabited enum to serve as a namespace for `x86{,-64}`-specific
/// constants.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub enum X86 {}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
impl X86 {
    /// The start bit index for the 32-bit relative immediate operand of a `jmp
    /// rel32` instruction.
    const JMP_REL32_OPERAND_START: usize =
        mem::size_of_val(&Self::OPCODE_JMP_REL32) * u8::BITS as usize;

    /// The end bit index for the 32-bit relative immediate operand of a `jmp
    /// rel32` instruction.
    const JMP_REL32_OPERAND_END: usize = Self::JMP_REL32_OPERAND_START + u32::BITS as usize - 1;

    /// The size of a full `jmp rel32` instruction mnemonic.
    const MNEMONIC_JMP_REL32_SIZE: usize = mem::size_of_val(&Self::OPCODE_JMP_REL32) /* opcode */ + mem::size_of::<u32>() /* rel32 */;

    /// The Operation Code for the `x86` `jmp rel32` instruction mnemonic.
    ///
    /// Reference: <https://www.felixcloutier.com/x86/jmp>
    const OPCODE_JMP_REL32: u8 = 0xE9;

    /// The Operation Code for the `x86` `int3` instruction mnemonic.
    ///
    /// Reference: <https://www.felixcloutier.com/x86/intn:into:int3:int1>
    const OPCODE_INT3: u8 = 0xCC;

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

    /// The associated template instruction for the `x86` architecture.
    ///
    /// This is to be used with the [`JmpRel32`] type.
    const JMP_REL32_UD2_NOP_TEMPLATE: u64 = (Self::OPCODE_NOP as u64)
        << (u8::BITS as usize * (Self::MNEMONIC_JMP_REL32_SIZE + mem::size_of::<u16>()))
        | (Self::OPCODE_UD2 as u64) << (u8::BITS as usize * Self::MNEMONIC_JMP_REL32_SIZE)
        | (Self::OPCODE_JMP_REL32 as u64);
}

/// The [`Field`] for the 32-bit relative immediate operand of a
/// `jmp rel32` instruction.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
type JmpRel32<'a> =
    Field<'a, { X86::JMP_REL32_OPERAND_START }, { X86::JMP_REL32_OPERAND_END }, u64, u32>;

/// The mutable counterpart to [`JmpRel32`].
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
type JmpRel32Mut<'a> = <JmpRel32<'a> as Counterpart>::Mut;

/// A template of the diversion instruction required for the establishment of a
/// [`Delegated`] implementation.
#[derive(Debug)]
#[repr(C)]
pub struct Template<D>(
    pub(crate)  fn(
        <<D as Delegator>::Target as Delegated>::Input,
    ) -> <<D as Delegator>::Target as Delegated>::Output,
    pub(crate) marker::PhantomData<fn() -> D>,
)
where
    D: Delegator + ?Sized;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
impl<D> Template<D> where D: Delegator + ?Sized {}

impl<D> Template<D>
where
    D: Delegator + ?Sized,
{
    /// Assemble the architecture-specific instruction required for the delegate
    /// diversion.
    #[inline]
    fn assemble(
        self,
        target_patchsite: Pin<
            &'static Patchsite<
                D,
                <D::Target as Delegated>::Input,
                <D::Target as Delegated>::Output,
            >,
        >,
    ) -> Diversion {
        let Self(delegate_address, ..) = self;

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        return {
            let base_address = core::ptr::from_ref(target_patchsite.get_ref()).addr();

            // NOTE: This is the architectural Program Counter value for `x86`.
            let base_address = base_address.wrapping_add(X86::MNEMONIC_JMP_REL32_SIZE);

            let target_address = delegate_address as usize;

            // NOTE: The kernel uses `code-model=small`, which guarantees that such
            // {E,R}IP-relative address will never be incorrect, as it resides within a +/-
            // 2 GiB range and thus jumps can be done via the `jmp rel32`
            // instruction.
            let relative_address = target_address.wrapping_sub(base_address) as u32;

            let mut encoded_instr = X86::JMP_REL32_UD2_NOP_TEMPLATE;

            JmpRel32Mut::wrap(&mut encoded_instr).merge(relative_address);

            Diversion(encoded_instr)
        };
    }
}

/// A patched [`Delegator`] and [`Delegated`] pair for some implementation.
#[derive(Debug, Hash)]
#[repr(transparent)]
pub struct Patched<D, I, O>(fn(I) -> O, marker::PhantomData<fn() -> D>)
where
    D: Delegator + ?Sized,
    D::Target: Delegated<Input = I, Output = O>;

/// The patchsite for a [`Delegator`] and [`Delegated`] pair.
#[derive(Debug)]
#[repr(transparent)]
pub struct Patchsite<D, I, O>(
    /// NOTE(x86): We use `jmp rel32` in `IA-32{,e}`, which is 5-bytes in size.
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    AtomicU64,
    marker::PhantomPinned,
    marker::PhantomData<fn() -> (D, I, O)>,
)
where
    D: Delegator + ?Sized,
    D::Target: Delegated<Input = I, Output = O>;

impl<D, I, O> Patchsite<D, I, O>
where
    D: Delegator + ?Sized,
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
    /// This method is unsafe because it assumes that no other thread will see a
    /// stale reference to the [`Delegator`] after the patch.
    #[inline]
    pub unsafe fn stale(
        self: Pin<&'static Self>,
        target_chosen: Chosen<D, I, O>,
    ) -> Result<Diversion, Diversion> {
        let Self(atomic_variable, ..) = self.get_ref();

        let target_template = Chosen::template(target_chosen);

        let Diversion(diversion_sequence) = Template::assemble(target_template, self);

        match atomic_variable.compare_exchange(
            atomic_variable.load(Ordering::Acquire),
            diversion_sequence,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            target_outcome @ (Ok(target_value) | Err(target_value)) => {
                let target_value = Diversion(target_value);

                if target_outcome.is_ok() {
                    Ok(target_value)
                } else {
                    Err(target_value)
                }
            }
        }
    }

    // TODO: Finish modeling the CMC-Publish and CMC-Acquire model.
}

/// A marker type to serve as an umbrella type for [`Delegator`] patchsites.
pub struct Patch<T, P>(
    // FIXME(unstable): Use the `never` (`!`) type explicitly here when stable.
    convert::Infallible,
    marker::PhantomData<fn() -> (T, P)>,
)
where
    T: Delegator,
    P: Pod;

impl<T, P> Patch<T, P>
where
    T: Delegator,
    P: Pod,
{
    /// Trampoline directly into the target [`Delegated::implementation`] for
    /// this [`Patch`].
    ///
    /// # Safety
    ///
    /// The *fn-pointer* sourced from this item must be transmuted to use the
    /// "Rust" ABI instead. This is safe as it is a simple trampoline.
    #[unsafe(link_section = concat!(env!("KERNEL_PATCH_STORAGE_SECTION"), ".delegate.trampoline"))]
    #[unsafe(naked)]
    // FIXME(unstable): Use "custom" ABI when available.
    pub unsafe extern "sysv64-unwind" fn trampoline(
        target_value: <T::Target as Delegated>::Input,
    ) -> <T::Target as Delegated>::Output {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        arch::naked_asm!(
            concat!("jmp", ' ', env!("KERNEL_PATCH_PATCHSITE_SYMBOL"), "_{target_symbol}_", "patchsite"),
            target_symbol = sym Self::patchsite,
            options(att_syntax)
        );
    }

    /// Run the target [`Delegated::implementation`] for this [`Patch`].
    #[inline(always)]
    pub fn run(target_value: <T::Target as Delegated>::Input) -> <T::Target as Delegated>::Output {
        // SAFETY: This is safe, as it is a trampoline to a function with the
        // correct parameters and ABI.
        unsafe { Self::trampoline(target_value) }
    }

    /// The patchsite of the [`Delegator`] and [`Pod`] association.
    ///
    /// # Safety
    ///
    /// This associated function must never be called.
    #[unsafe(link_section = concat!(env!("KERNEL_PATCH_STORAGE_SECTION"), ".delegate.offset"))]
    #[unsafe(naked)]
    #[allow(
        named_asm_labels,
        reason = "inline assembler defines globally-visible symbols"
    )]
    // FIXME: As we are dealing with a mixed code model, we need to have all
    // sections and subsections under the "KERNEL_PATCH_STORAGE_SECTION" reside in
    // RAM and not memory-mapped flash to be able to actually patch properly.
    // XIP WILL RAPE US IN THE ASSHOLE!
    // FIXME(unstable): Use "custom" ABI when available.
    pub unsafe extern "sysv64-unwind" fn patchsite(
        target_value: <T::Target as Delegated>::Input,
    ) -> <T::Target as Delegated>::Output {
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

                default_symbol = sym <T::Target as Delegated>::implementation,

                X86_OPCODE_JMP_REL32 = const X86::OPCODE_JMP_REL32,
                X86_MNEMONIC_JMP_REL32_SIZE = const X86::MNEMONIC_JMP_REL32_SIZE,

                X86_OPCODE_INT3 = const X86::OPCODE_INT3,

                options(att_syntax)
            )
        }
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cmc::Publish,
        patch::{
            choose::Chosen,
            delegate::{Delegated, Delegator, Single},
        },
    };

    /// A simple delegated implementation for testing.
    #[derive(Debug, Copy, Clone)]
    struct TestImpl;

    impl Delegated for TestImpl {
        type Input = u32;
        type Output = u32;

        fn implementation(input: Self::Input) -> Self::Output {
            input.wrapping_add(42)
        }
    }

    /// A simple delegated implementation for testing.
    #[derive(Debug, Copy, Clone)]
    struct TestImpl2;

    impl Delegated for TestImpl2 {
        type Input = u32;
        type Output = u32;

        fn implementation(input: Self::Input) -> Self::Output {
            input.wrapping_add(52)
        }
    }

    /// A simple Pod value for testing.
    #[derive(Debug, Copy, Clone)]
    struct TestPod;

    type TestDelegator = Single<TestImpl, TestPod>;

    #[derive(Copy, Clone, Debug)]
    struct TestDelegator2 {}

    unsafe impl Delegator for TestDelegator2 {
        type Target = TestImpl;

        type Value = ();

        fn choose(
            target_value: &'static Self::Value,
        ) -> Chosen<Self, <Self::Target as Delegated>::Input, <Self::Target as Delegated>::Output>
        {
            Chosen::delegated::<TestImpl2>()
        }
    }

    #[test]
    fn patchsite_and_delegator_integration() {
        dbg!(Patch::<TestDelegator2, ()>::run(10));

        // Verify delegator choice mechanism
        let chosen = TestDelegator2::choose(&());

        // Verify patchsite can be retrieved
        let patchsite = Chosen::<TestDelegator2, u32, u32>::patchsite();

        println!("{patchsite:?}");

        let _ = unsafe { Patchsite::stale(patchsite, chosen) };

        dbg!(Patch::<TestDelegator2, ()>::run(10));
    }
}
*/
