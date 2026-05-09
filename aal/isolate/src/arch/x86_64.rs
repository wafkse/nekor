//! Architecture-specific code to the `x86-64` architecture.

pub mod context;

pub mod register;

pub mod xstate;

pub mod fsgsbase;

use self::register::{Base, Gpr, Rflags};

use core::{arch, mem};

use nekor_aal_arch::x86::segmentation::{RawCodeSegment, RawDataSegment};

use nekor_aal_hotpatch::prelude::Patch;

/// A helper macro to define a Xsave area with a feature-guided size.
macro_rules! area {
    () => {};
    (
        $(
            #[$target_meta:meta]
        )*
        use $target_feature:literal for $target_vis:vis $target_ident:ident($($target_size:literal),+ $(,)?)
    ) => {
        tokel::stream!(
            $(
                #[$target_meta]
            )*
            $target_vis struct $target_ident (
                [
                    u8;
                    const {
                        #[deny(unreachable_code, reason = "mutual exclusion of xsave-related crate features")]
                        'a: {
                            $(
                                // NOTE: Flatten due to None-delimited groups.
                                #[cfg(feature = [< $target_feature "-" [< $target_size >]:to_string >]:flatten:concatenate)] break 'a $target_size;
                            )+
                        }
                    }
                ]
            );

            #[cfg(
                all(
                   $(
                       // NOTE: Flatten due to None-delimited groups.
                       not(feature = [< $target_feature "-" [< $target_size >]:to_string >]:flatten:concatenate)
                   ),+
                )
            )]
            compile_error!(
                [<
                    "a single feature from (" $([< " `" [< $target_feature "-" [< $target_size >]:to_string >]:flatten:concatenate "` " >]:concatenate)+ ") must be present"
                >]:concatenate);
        );
    };

}

// TODO: Change these sizes to be macro-guided. Determine whether sizes should
// be independent of common xstate feature sizes or just stepped using a more
// granular exponent-of-2 series.

// TODO: The stack space used by this is too big. Move to per-xstate crate
// features. Then, calculate the xsave state size for it. Or make the kernel
// assign it a dynamic size. (Make it an UnsafeCell<[u8]> wrapper)

// On-stack, we use the dynamic size, and for Isolates, we hold a complete xsave
// area context for the enabled features. (move from size-oriented to
// feature-oriented)

// NOTE: Implement the XsaveArea structure.
area!(
    /// An area for the `xsave` family of instructions.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    #[repr(C, align(64) /* architecturally required to be 64-byte aligned */)]
    use "arch-x86_64-xsave" for pub XsaveArea(512, 1024, 2048, 4096, 8192, 16384)
);

/// An Interrupt Return Frame.
///
/// This is pushed by the processor on Interrupt Service Routine entry when in
/// `x86-64` mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct IRetFrame {
    // NOTE: These are put here to have the CPU build our trap context directly through the ISR
    // entry layout.
    /// The 64-bit extended "ip" register.
    rip: Gpr,

    /// The "cs" segment selector.
    cs: RawCodeSegment,

    /// The architectural flags register.
    rflags: Rflags,

    /// The 64-bit extended "sp" register.
    rsp: Gpr,

    /// The "ss" segment selector.
    ss: RawDataSegment,
}

impl IRetFrame {
    /// The 64-bit extended "ip" register.
    #[inline]
    pub const fn rip(&self) -> &Gpr {
        let IRetFrame { rip, .. } = self;

        rip
    }

    /// Resolve a mutable reference to the 64-bit extended "ip" register.
    #[inline]
    pub const fn rip_mut(&mut self) -> &mut Gpr {
        let IRetFrame { rip, .. } = self;

        rip
    }

    /// The "cs" segment selector.
    #[inline]
    pub const fn cs(&self) -> &RawCodeSegment {
        let IRetFrame { cs, .. } = self;

        cs
    }

    /// Resolve a mutable reference to the "cs" segment selector.
    #[inline]
    pub const fn cs_mut(&mut self) -> &mut RawCodeSegment {
        let IRetFrame { cs, .. } = self;

        cs
    }

    /// The architectural flags register.
    #[inline]
    pub const fn rflags(&self) -> &Rflags {
        let IRetFrame { rflags, .. } = self;

        rflags
    }

    /// Resolve a mutable reference to the architectural flags register.
    #[inline]
    pub const fn rflags_mut(&mut self) -> &mut Rflags {
        let IRetFrame { rflags, .. } = self;

        rflags
    }

    /// The 64-bit extended "sp" register.
    #[inline]
    pub const fn rsp(&self) -> &Gpr {
        let IRetFrame { rsp, .. } = self;

        rsp
    }

    /// Resolve a mutable reference to the 64-bit extended "sp" register.
    #[inline]
    pub const fn rsp_mut(&mut self) -> &mut Gpr {
        let IRetFrame { rsp, .. } = self;

        rsp
    }

    /// The "ss" segment selector.
    #[inline]
    pub const fn ss(&self) -> &RawDataSegment {
        let IRetFrame { ss, .. } = self;

        ss
    }

    /// Resolve a mutable reference to the "ss" segment selector.
    #[inline]
    pub const fn ss_mut(&mut self) -> &mut RawDataSegment {
        let IRetFrame { ss, .. } = self;

        ss
    }
}

/// An architectural context for the `x86-64` architecture.
///
/// This contains all the captured state from an individual thread of execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C, align(8))]
pub struct Context {
    // NOTE: The extended segment register base addresses can either be sourced through a `rdmsr`
    // or `rd{gs,fs}base` from the `FSGSBASE` extension, which both require either a scratch
    // register ("%rax", in this case), or the "%eax", "%ecx", and "%edx" general-purpose register
    // triplet.
    /// The "gs" extended segment register base address.
    gsbase: Base,

    /// The "fs" extended segment register base address.
    fsbase: Base,

    // NOTE: General-purpose part of the trap context terminates here.
    /// The 64-bit extended "bx" register.
    rbx: Gpr,
    /// The 64-bit extended "cx" register.
    rcx: Gpr,
    /// The 64-bit extended "dx" register.
    rdx: Gpr,

    /// The 64-bit extended "si" register.
    rsi: Gpr,
    /// The 64-bit extended "di" register.
    rdi: Gpr,

    /// The 64-bit extended "bp" register.
    rbp: Gpr,

    /// The 64-bit extended "8" register.
    r8: Gpr,
    /// The 64-bit extended "9" register.
    r9: Gpr,
    /// The 64-bit extended "10" register.
    r10: Gpr,
    /// The 64-bit extended "11" register.
    r11: Gpr,
    /// The 64-bit extended "12" register.
    r12: Gpr,
    /// The 64-bit extended "13" register.
    r13: Gpr,
    /// The 64-bit extended "14" register.
    r14: Gpr,
    /// The 64-bit extended "15" register.
    r15: Gpr,

    // NOTE: To be used as our scratch-save register, it is pushed first.
    /// The 64-bit extended "ax" register.
    rax: Gpr,

    /// The Interrupt Return Frame contained within this trap context.
    interrupt_frame: IRetFrame,
}

impl Context {
    /// Determine the "%gs.base" extended segment register base of this trap
    /// context.
    #[inline]
    pub const fn gsbase(&self) -> &Base {
        let Self { gsbase, .. } = self;

        gsbase
    }

    /// Resolve a mutable reference to the "%gs.base" extended segment register
    /// base of this trap context.
    #[inline]
    pub const fn gsbase_mut(&mut self) -> &mut Base {
        let Self { gsbase, .. } = self;

        gsbase
    }

    /// Determine the "%fs.base" extended segment register base of this trap
    /// context.
    #[inline]
    pub const fn fsbase(&self) -> &Base {
        let Self { fsbase, .. } = self;

        fsbase
    }

    /// Resolve a mutable reference to the "%fs.base" extended segment register
    /// base of this trap context.
    #[inline]
    pub const fn fsbase_mut(&mut self) -> &mut Base {
        let Self { fsbase, .. } = self;

        fsbase
    }

    /// The 64-bit extended "ax" register.
    #[inline]
    pub const fn rax(&self) -> &Gpr {
        let Self { rax, .. } = self;

        rax
    }

    /// Resolve a mutable reference to the 64-bit extended "ax" register.
    #[inline]
    pub const fn rax_mut(&mut self) -> &mut Gpr {
        let Self { rax, .. } = self;

        rax
    }

    /// The 64-bit extended "bx" register.
    #[inline]
    pub const fn rbx(&self) -> &Gpr {
        let Self { rbx, .. } = self;

        rbx
    }

    /// Resolve a mutable reference to the 64-bit extended "bx" register.
    #[inline]
    pub const fn rbx_mut(&mut self) -> &mut Gpr {
        let Self { rbx, .. } = self;

        rbx
    }

    /// The 64-bit extended "cx" register.
    #[inline]
    pub const fn rcx(&self) -> &Gpr {
        let Self { rcx, .. } = self;

        rcx
    }

    /// Resolve a mutable reference to the 64-bit extended "cx" register.
    #[inline]
    pub const fn rcx_mut(&mut self) -> &mut Gpr {
        let Self { rcx, .. } = self;

        rcx
    }

    /// The 64-bit extended "dx" register.
    #[inline]
    pub const fn rdx(&self) -> &Gpr {
        let Self { rdx, .. } = self;

        rdx
    }

    /// Resolve a mutable reference to the 64-bit extended "dx" register.
    #[inline]
    pub const fn rdx_mut(&mut self) -> &mut Gpr {
        let Self { rdx, .. } = self;

        rdx
    }

    /// The 64-bit extended "si" register.
    #[inline]
    pub const fn rsi(&self) -> &Gpr {
        let Self { rsi, .. } = self;

        rsi
    }

    /// Resolve a mutable reference to the 64-bit extended "si" register.
    #[inline]
    pub const fn rsi_mut(&mut self) -> &mut Gpr {
        let Self { rsi, .. } = self;

        rsi
    }

    /// The 64-bit extended "di" register.
    #[inline]
    pub const fn rdi(&self) -> &Gpr {
        let Self { rdi, .. } = self;

        rdi
    }

    /// Resolve a mutable reference to the 64-bit extended "di" register.
    #[inline]
    pub const fn rdi_mut(&mut self) -> &mut Gpr {
        let Self { rdi, .. } = self;

        rdi
    }

    /// The 64-bit extended "bp" register.
    #[inline]
    pub const fn rbp(&self) -> &Gpr {
        let Self { rbp, .. } = self;

        rbp
    }

    /// Resolve a mutable reference to the 64-bit extended "bp" register.
    #[inline]
    pub const fn rbp_mut(&mut self) -> &mut Gpr {
        let Self { rbp, .. } = self;

        rbp
    }

    /// The 64-bit extended "8" register.
    #[inline]
    pub const fn r8(&self) -> &Gpr {
        let Self { r8, .. } = self;

        r8
    }

    /// Resolve a mutable reference to the 64-bit extended "8" register.
    #[inline]
    pub const fn r8_mut(&mut self) -> &mut Gpr {
        let Self { r8, .. } = self;

        r8
    }

    /// The 64-bit extended "9" register.
    #[inline]
    pub const fn r9(&self) -> &Gpr {
        let Self { r9, .. } = self;

        r9
    }

    /// Resolve a mutable reference to the 64-bit extended "9" register.
    #[inline]
    pub const fn r9_mut(&mut self) -> &mut Gpr {
        let Self { r9, .. } = self;

        r9
    }

    /// The 64-bit extended "10" register.
    #[inline]
    pub const fn r10(&self) -> &Gpr {
        let Self { r10, .. } = self;

        r10
    }

    /// Resolve a mutable reference to the 64-bit extended "10" register.
    #[inline]
    pub const fn r10_mut(&mut self) -> &mut Gpr {
        let Self { r10, .. } = self;

        r10
    }

    /// The 64-bit extended "11" register.
    #[inline]
    pub const fn r11(&self) -> &Gpr {
        let Self { r11, .. } = self;

        r11
    }

    /// Resolve a mutable reference to the 64-bit extended "11" register.
    #[inline]
    pub const fn r11_mut(&mut self) -> &mut Gpr {
        let Self { r11, .. } = self;

        r11
    }

    /// The 64-bit extended "12" register.
    #[inline]
    pub const fn r12(&self) -> &Gpr {
        let Self { r12, .. } = self;

        r12
    }

    /// Resolve a mutable reference to the 64-bit extended "12" register.
    #[inline]
    pub const fn r12_mut(&mut self) -> &mut Gpr {
        let Self { r12, .. } = self;

        r12
    }

    /// The 64-bit extended "13" register.
    #[inline]
    pub const fn r13(&self) -> &Gpr {
        let Self { r13, .. } = self;

        r13
    }

    /// Resolve a mutable reference to the 64-bit extended "13" register.
    #[inline]
    pub const fn r13_mut(&mut self) -> &mut Gpr {
        let Self { r13, .. } = self;

        r13
    }

    /// The 64-bit extended "14" register.
    #[inline]
    pub const fn r14(&self) -> &Gpr {
        let Self { r14, .. } = self;

        r14
    }

    /// Resolve a mutable reference to the 64-bit extended "14" register.
    #[inline]
    pub const fn r14_mut(&mut self) -> &mut Gpr {
        let Self { r14, .. } = self;

        r14
    }

    /// The 64-bit extended "15" register.
    #[inline]
    pub const fn r15(&self) -> &Gpr {
        let Self { r15, .. } = self;

        r15
    }

    /// Resolve a mutable reference to the 64-bit extended "15" register.
    #[inline]
    pub const fn r15_mut(&mut self) -> &mut Gpr {
        let Self { r15, .. } = self;

        r15
    }

    /// The 64-bit extended "ip" register.
    #[inline]
    pub const fn rip(&self) -> &Gpr {
        let Self {
            interrupt_frame, ..
        } = self;

        interrupt_frame.rip()
    }

    /// Resolve a mutable reference to the 64-bit extended "ip" register.
    #[inline]
    pub const fn rip_mut(&mut self) -> &mut Gpr {
        let Self {
            interrupt_frame, ..
        } = self;

        interrupt_frame.rip_mut()
    }

    /// The "cs" segment selector.
    #[inline]
    pub const fn cs(&self) -> &RawCodeSegment {
        let Self {
            interrupt_frame, ..
        } = self;

        interrupt_frame.cs()
    }

    /// Resolve a mutable reference to the "cs" segment selector.
    #[inline]
    pub const fn cs_mut(&mut self) -> &mut RawCodeSegment {
        let Self {
            interrupt_frame, ..
        } = self;

        interrupt_frame.cs_mut()
    }

    /// The architectural flags register.
    #[inline]
    pub const fn rflags(&self) -> &Rflags {
        let Self {
            interrupt_frame, ..
        } = self;

        interrupt_frame.rflags()
    }

    /// Resolve a mutable reference to the architectural flags register.
    #[inline]
    pub const fn rflags_mut(&mut self) -> &mut Rflags {
        let Self {
            interrupt_frame, ..
        } = self;

        interrupt_frame.rflags_mut()
    }

    /// The 64-bit extended "sp" register.
    #[inline]
    pub const fn rsp(&self) -> &Gpr {
        let Self {
            interrupt_frame, ..
        } = self;

        interrupt_frame.rsp()
    }

    /// Resolve a mutable reference to the 64-bit extended "sp" register.
    #[inline]
    pub const fn rsp_mut(&mut self) -> &mut Gpr {
        let Self {
            interrupt_frame, ..
        } = self;

        interrupt_frame.rsp_mut()
    }

    /// The "ss" segment selector.
    #[inline]
    pub const fn ss(&self) -> &RawDataSegment {
        let Self {
            interrupt_frame, ..
        } = self;

        interrupt_frame.ss()
    }

    /// Resolve a mutable reference to the "ss" segment selector.
    #[inline]
    pub const fn ss_mut(&mut self) -> &mut RawDataSegment {
        let Self {
            interrupt_frame, ..
        } = self;

        interrupt_frame.ss_mut()
    }

    /// Determine the Interrupt Return Frame contained within this trap context.
    #[inline]
    pub const fn interrupt_frame(&self) -> &IRetFrame {
        let Self {
            interrupt_frame, ..
        } = self;

        interrupt_frame
    }

    /// Resolve a mutable reference to the Interrupt Return Frame contained
    /// within this trap context.
    #[inline]
    pub const fn interrupt_frame_mut(&mut self) -> &mut IRetFrame {
        let Self {
            interrupt_frame, ..
        } = self;

        interrupt_frame
    }
}

/// An error-code pushed to the Interrupt Service Routine by the processor.
#[repr(transparent)]
pub struct ErrorCode(u32);

/// A trait that describes an enter point to a *Context Switch*.
pub unsafe trait Switch {
    /// The context-switch handler associated function.
    fn context(target_pair: (&mut Context, Option<&ErrorCode>), target_area: &mut XsaveArea);

    /// A trampoline to the context-switch entry.
    ///
    /// This is used to provide a well-known ABI (particularly, the System V
    /// ABI) to the low-level assembly implementation.
    #[doc(hidden)]
    unsafe extern "sysv64" fn trampoline(
        target_context: &mut Context,
        target_code: Option<&ErrorCode>,
        target_area: &mut XsaveArea,
    ) -> () {
        Self::context((target_context, target_code), target_area)
    }

    // TODO: Define the baseline segment descriptor indices. We need to define
    // user/uservisor code/data segments, and segments used to contain the offset
    // from a baseline per_cpu base section.
    //
    // We can use the LSL instruction and save the user fs/gsbase pair verbatim into
    // the same Context.
    //
    // We can use a simple 32-bit offset from the percpu base section to find our
    // own per_cpu base address.
    //
    // Use the same offset model used by linux, where gsbase is the section start
    // address + local in-section base pcpu offset so one can index into %gs
    // directly with a symbol.
    //
    // This will require an overhaul over the local aal subsystem. Macros to define
    // such statics will be required. They all go into a template section, and the
    // macros handle implementing a "Local" trait to get easy access to the
    // variable. make a newtype struct with a nonstandard name that shadows the
    // inner static definition so it can be accessed in one go, in an optimized
    // fashion.
    //
    // Define the static inside the impl block.
    //
    // Once we have gotten rid of paranoid entries. Work on unification of
    // error-code and error-code- less ISRs. Add a small structure after the raw
    // hardware frame. One united interrupt entry to reside in the hot interrupt
    // code section. LSL will get rid of swapgs and testing the code segment's RPL


    /// An Interrupt Service Routine entry point.
    ///
    /// The const-generic `E` indicates whether this routine is destined towards
    /// an error-code-pushing interrupt vector.
    ///
    /// # Safety
    ///
    /// TODO
    #[unsafe(naked)]
    // FIXME(unstable): Make this an extern "custom" function when it is stable.
    unsafe extern "C" fn interrupt<const E: bool>() -> ! {
        arch::naked_asm!(
            // FIXME: Paranoid Region - Start
            // NOTE: Context-save start.
            // NOTE: "%rax" is our scratch-save register.
            ".ifeq {has_error} 1",
            // NOTE(alignment): End of `IRetFrame`, `x86-64` processors automatically align the stack to a `16-byte` boundary, and `IRetFrame`
            //                  is composed of `8-byte` fields, so the `Context` structure is also aligned to a `8-byte` boundary.
            //                  In this case, we do not expect the error code to be part of our `IRetFrame`.
            //
            // NOTE: We expect a 8-byte error code to be at the top of the stack.
            //       To preserve the same stack layout across exception-handling Interrupt Service Routines that push an error code,
            //       we swap our `%rax` register with the error code contained within the stack to build a proper `Context` structure.
            //
            //       This is done via simple stack pointer manipulation. This does depend on a microarchitectural "Stack Engine" being implemented for efficiency.
            "pushq (%rsp)",
            "movq %rax, 8(%rsp)",
            "popq %rax",
            ".else",
            // NOTE(alignment): End of `IRetFrame`, `x86-64` processors automatically align the stack to a `16-byte` boundary, and `IRetFrame`
            // is composed of `8-byte` fields, so the `Context` structure is also aligned to a `8-byte` boundary.

            "pushq %rax",
             ".endif",
            //
            "pushq %r15",
            "pushq %r14",
            "pushq %r13",
            "pushq %r12",

            // NOTE: "%r12" is callee-saved as per the System V ABI, so we move
            // the error code there as soon as possible, as we require a function
            // call to save our "%fs.base" and "%gs.base" values.
            "movq %rax, %r12",

            "pushq %r11",
            "pushq %r10",
            "pushq %r9",
            "pushq %r8",
            //
            "pushq %rbp",
            //
            "pushq %rdi",
            "pushq %rsi",
            //
            "pushq %rdx",
            "pushq %rcx",
            "pushq %rbx",
            //
            // NOTE: The "Direction Flag" (DF) must be zero as per the System V ABI.
            "cld",

            // NOTE: Force-Clear the "Alignment Checking" (AC) flag for SMAP purposes.
            //
            //       If the interrupted code had the "Alignment Checking" (AC) flag set, it is restored verbatim at interrupt exit.
            "clac",

            // NOTE: Store "%fs.base" and "%gs.base".
            // NOTE(mitigate): The store patchsite delegates to implementations that do not fetch memory or branch, so mitigation is preserved.
            "callq {store_base}",
            // NOTE: These correspond to the `FsGsBase` structure, both base addresses correspond to the `INTEGER` class, which places them in these registers.
            "pushq %rax",
            "pushq %rdx",
            // NOTE: Context-save end.
            // NOTE: Swap the current "%gs.base" and the `KERNEL_GS_BASE` model-specific register values if this entry point required a CPL change.
            "testb $3, {code_segment}(%rsp)",
            "jz 2f",
            "swapgs",
            "2:",
            // FIXME: Paranoid Region - End

            // FIXME(mitigate): Start work on mitigation design.

            // NOTE: An Xsave Area must always be 64-byte aligned, save `Context` pointer in `%rbx`.
            //
            //       Note that we are only calling System V ABI functions here, so `%rbx` is callee-saved.
            "movq %rsp, %rbx",

            ".ifeq {has_error} 1",
            // NOTE: The "%r12" register is callee-saved as per the System V ABI and contains our error code.
            //       It is pushed to the stack to be able to reference it in a "maybe-missing" manner to the interrupt handler.
            //
            //       We store a pointer to it in the aforementioned register, `%r12`.
            "pushq %r12",
            "movq %rsp, %r12",
            ".else",
            // NOTE: No error code on-stack, this register must be zero to serve as a null-pointer, which corresponds to a `None::<&ErrorCode>` within FFI rules.
            "xorq %r12, %r12",
             ".endif",

            // NOTE: Reserve the Xsave Area on-stack and request all components.
             "subq ${area_size}, %rsp",
            // NOTE: Dynamically align stack to 64-byte boundary.
            //
            //       This can waste some bytes on the stack, but they are restored afterwards.
            "andq $-64, %rsp",

            // NOTE: Perform the `xsaves` (or `xsavec`, in usermode) on the stack.
            "xorl %edx, %edx",
            "xorl %eax, %eax",
            "notl %edx",
            "notl %eax",

            // NOTE: Save using `xsaves` (or a bare `xsavec`, in usermode).
            #[cfg(usermode)]
            "xsavecq (%rsp)",
            #[cfg(not(usermode))]
            "xsavesq (%rsp)",



            // NOTE: Point to the `Context`, (and error code, if pushed) and `XsaveArea` structures we just created on-stack as our first and second parameters.
            //
            //       The functions called on interrupt entry all follow the System V ABI.
            "movq %rbx, %rdi",
            "movq %r12, %rsi",
            "movq %rsp, %rdx",
            "callq {trampoline}",

            // NOTE: Restore using `xrstors` (or a bare `xrstor`, in usermode).
            #[cfg(usermode)]
            "xrstorq (%rsp)",
            #[cfg(not(usermode))]
            "xrstorsq (%rsp)",

            // NOTE: Restore the stack pointer to `Context`.
            "movq %rbx, %rsp",

            // NOTE: Restore "fsbase" and "gsbase".
            //
            //       These need to be popped in an inverted order (i.e., "%gs.base" and then "%fs.base")
            "popq %rsi",
            "popq %rdi",
            "callq {load_base}",

            // NOTE: Restore all registers.
            // FIXME: Paranoid Region - Start
            "2:",
            "popq %rbx",
            "popq %rcx",
            "popq %rdx",
            //
            "popq %rsi",
            "popq %rdi",
            //
            "popq %rbp",
            //
            "popq %r8",
            "popq %r9",
            "popq %r10",
            "popq %r11",
            "popq %r12",
            "popq %r13",
            "popq %r14",
            "popq %r15",
            //
            "popq %rax",
            // NOTE: The `IRetFrame` is still on-stack, which we can use to return directly.
            "iretq",
            // FIXME: Paranoid Region - End
            "3:",
            // NOTE: This is mapped as expected: [true => 1, false => 0].
            has_error = const E as usize,
            // NOTE: This is the size, in bytes, of the Xsave Area to be used.
            area_size = const mem::size_of::<XsaveArea>(),
            // NOTE: Here we calculate the in-stack offset of the Code Segment pushed to the `IRetFrame`, as we need to reference it directly.
            code_segment = const mem::offset_of!(Context, interrupt_frame) + mem::offset_of!(IRetFrame, cs),
            // NOTE: The trampoline to the "fetch fs and gsbase" hotpatchable delegator.
            store_base = sym Patch::<fsgsbase::ReadFsgsbaseDelegator, fsgsbase::RdmsrOrFsgsbase>::trampoline,
            // NOTE: The trampoline to the "set fs and gsbase" hotpatchable delegator.
            load_base = sym Patch::<fsgsbase::WriteFsgsbaseDelegator, fsgsbase::WrmsrOrFsgsbase>::trampoline,
            // The context-switch trampoline.
            trampoline = sym Self::trampoline,
            options(att_syntax)
        )
    }
}
