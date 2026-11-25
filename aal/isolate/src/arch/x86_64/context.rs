//! Save and restore x86-64 machine contexts at a SysV64 call boundary.

use core::{arch, fmt, mem, ptr::NonNull};

use nekor_aal_arch::{
    x86::segmentation::{RawCodeSegment, RawDataSegment},
    x86_64::register::{Base, Gpr, RawRflags, Rflags},
};
use nekor_aal_hotpatch::prelude::Patch;

use crate::arch::x86_64::fsgsbase::{ReadFsgsbaseDelegator, WriteFsgsbaseDelegator};

/// An Interrupt Return Frame.
///
/// This is pushed by the processor on Interrupt Service Routine entry when in
/// `x86-64` mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
// NOTE(invariant): Field order matches the processor's interrupt return frame layout.
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
    #[must_use]
    pub const fn rip(&self) -> &Gpr {
        let &Self { ref rip, .. } = self;

        rip
    }

    /// Resolve a mutable reference to the 64-bit extended "ip" register.
    #[inline]
    pub const fn rip_mut(&mut self) -> &mut Gpr {
        let &mut Self { ref mut rip, .. } = self;

        rip
    }

    /// The "cs" segment selector.
    #[inline]
    #[must_use]
    pub const fn cs(&self) -> &RawCodeSegment {
        let &Self { ref cs, .. } = self;

        cs
    }

    /// Resolve a mutable reference to the "cs" segment selector.
    #[inline]
    pub const fn cs_mut(&mut self) -> &mut RawCodeSegment {
        let &mut Self { ref mut cs, .. } = self;

        cs
    }

    /// The architectural flags register.
    #[inline]
    #[must_use]
    pub const fn rflags(&self) -> &Rflags {
        let &Self { ref rflags, .. } = self;

        rflags
    }

    /// Resolve a mutable reference to the architectural flags register.
    #[inline]
    pub const fn rflags_mut(&mut self) -> &mut Rflags {
        let &mut Self { ref mut rflags, .. } = self;

        rflags
    }

    /// The 64-bit extended "sp" register.
    #[inline]
    #[must_use]
    pub const fn rsp(&self) -> &Gpr {
        let &Self { ref rsp, .. } = self;

        rsp
    }

    /// Resolve a mutable reference to the 64-bit extended "sp" register.
    #[inline]
    pub const fn rsp_mut(&mut self) -> &mut Gpr {
        let &mut Self { ref mut rsp, .. } = self;

        rsp
    }

    /// The "ss" segment selector.
    #[inline]
    #[must_use]
    pub const fn ss(&self) -> &RawDataSegment {
        let &Self { ref ss, .. } = self;

        ss
    }

    /// Resolve a mutable reference to the "ss" segment selector.
    #[inline]
    pub const fn ss_mut(&mut self) -> &mut RawDataSegment {
        let &mut Self { ref mut ss, .. } = self;

        ss
    }
}

detail::context!(
    /// An architectural context for the `x86-64` architecture.
    ///
    /// This contains all the captured state from an individual thread of execution.
    ///
    /// # What is an architectural context?
    ///
    /// An architectural context is the structure that principally identifies a particular execution
    /// context in Nekor.
    ///
    /// # Self-captured Machine Contexts
    ///
    /// The initial path enters the callback while [`Context::save`] stays suspended. Restoring
    /// the saved image completes that call through an assembly return sequence. The caller
    /// therefore observes a single return, with execution continuing after `save`.
    ///
    /// The callback shares the caller's stack. Resumption ends its execution and allows its stack
    /// space to be reused. State needed afterward must outlive the callback, and any required
    /// cleanup must finish before resumption, including cleanup through `Drop`.
    ///
    /// Copying the image preserves references to the same stack and storage. Independent execution
    /// requires its own stack, entry point, and entry arguments.

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    #[repr(C, align(8))]
    // NOTE(invariant): The instruction-level implementation of GPR context switches make use of this
    // layout, exactly.
    pub struct Context {
        // NOTE: The extended segment register base addresses can either be sourced through a `rdmsr`
        // or `rd{gs,fs}base` from the `FSGSBASE` extension, which both require either a scratch
        // register ("%rax", in this case), or the "%eax", "%ecx", and "%edx" general-purpose register
        // triplet.
        /// The "gs" extended segment register base address.
        gsbase: Base,

        /// The "fs" extended segment register base address.
        fsbase: Base,

        become {
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
        }

        // NOTE: To be used as our scratch-save register, it is pushed first.
        /// The 64-bit extended "ax" register.
        rax: Gpr,

        /// The Interrupt Return Frame contained within this trap context.
        interrupt_frame: IRetFrame,
    }
);

impl fmt::Debug for Context {
    #[inline]
    fn fmt(&self, target_formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let &Self {
            gsbase,
            fsbase,
            rbx,
            rcx,
            rdx,
            rsi,
            rdi,
            rbp,
            r8,
            r9,
            r10,
            r11,
            r12,
            r13,
            r14,
            r15,
            rax,
            interrupt_frame:
                IRetFrame {
                    rip,
                    cs,
                    rflags,
                    rsp,
                    ss,
                },
        } = self;
        let RawRflags(rflags) = RawRflags::take(rflags);

        write!(
            target_formatter,
            concat!(
                "%rip: {:#018x}  %rsp: {:#018x}\n",
                "%rax: {:#018x}  %rbx: {:#018x}  %rcx: {:#018x}\n",
                "%rdx: {:#018x}  %rsi: {:#018x}  %rdi: {:#018x}\n",
                "%rbp: {:#018x}   %r8: {:#018x}   %r9: {:#018x}\n",
                "%r10: {:#018x}  %r11: {:#018x}  %r12: {:#018x}\n",
                "%r13: {:#018x}  %r14: {:#018x}  %r15: {:#018x}\n",
                "%fsbase: {:#018x}  %gsbase: {:#018x}\n",
                "%rflags: {:#018x}  %cs: {:#06x}  %ss: {:#06x}",
            ),
            rip.unwrap(),
            rsp.unwrap(),
            rax.unwrap(),
            rbx.unwrap(),
            rcx.unwrap(),
            rdx.unwrap(),
            rsi.unwrap(),
            rdi.unwrap(),
            rbp.unwrap(),
            r8.unwrap(),
            r9.unwrap(),
            r10.unwrap(),
            r11.unwrap(),
            r12.unwrap(),
            r13.unwrap(),
            r14.unwrap(),
            r15.unwrap(),
            fsbase.unwrap(),
            gsbase.unwrap(),
            rflags,
            cs.raw(),
            ss.raw(),
        )
    }
}

impl Context {
    /// Determine the "%gs.base" extended segment register base of this trap
    /// context.
    #[inline]
    #[must_use]
    pub const fn gsbase(&self) -> &Base {
        let &Self { ref gsbase, .. } = self;

        gsbase
    }

    /// Resolve a mutable reference to the "%gs.base" extended segment register
    /// base of this trap context.
    #[inline]
    pub const fn gsbase_mut(&mut self) -> &mut Base {
        let &mut Self { ref mut gsbase, .. } = self;

        gsbase
    }

    /// Determine the "%fs.base" extended segment register base of this trap
    /// context.
    #[inline]
    #[must_use]
    pub const fn fsbase(&self) -> &Base {
        let &Self { ref fsbase, .. } = self;

        fsbase
    }

    /// Resolve a mutable reference to the "%fs.base" extended segment register
    /// base of this trap context.
    #[inline]
    pub const fn fsbase_mut(&mut self) -> &mut Base {
        let &mut Self { ref mut fsbase, .. } = self;

        fsbase
    }

    /// The 64-bit extended "ax" register.
    #[inline]
    #[must_use]
    pub const fn rax(&self) -> &Gpr {
        let &Self { ref rax, .. } = self;

        rax
    }

    /// Resolve a mutable reference to the 64-bit extended "ax" register.
    #[inline]
    pub const fn rax_mut(&mut self) -> &mut Gpr {
        let &mut Self { ref mut rax, .. } = self;

        rax
    }

    /// The 64-bit extended "bx" register.
    #[inline]
    #[must_use]
    pub const fn rbx(&self) -> &Gpr {
        let &Self { ref rbx, .. } = self;

        rbx
    }

    /// Resolve a mutable reference to the 64-bit extended "bx" register.
    #[inline]
    pub const fn rbx_mut(&mut self) -> &mut Gpr {
        let &mut Self { ref mut rbx, .. } = self;

        rbx
    }

    /// The 64-bit extended "cx" register.
    #[inline]
    #[must_use]
    pub const fn rcx(&self) -> &Gpr {
        let &Self { ref rcx, .. } = self;

        rcx
    }

    /// Resolve a mutable reference to the 64-bit extended "cx" register.
    #[inline]
    pub const fn rcx_mut(&mut self) -> &mut Gpr {
        let &mut Self { ref mut rcx, .. } = self;

        rcx
    }

    /// The 64-bit extended "dx" register.
    #[inline]
    #[must_use]
    pub const fn rdx(&self) -> &Gpr {
        let &Self { ref rdx, .. } = self;

        rdx
    }

    /// Resolve a mutable reference to the 64-bit extended "dx" register.
    #[inline]
    pub const fn rdx_mut(&mut self) -> &mut Gpr {
        let &mut Self { ref mut rdx, .. } = self;

        rdx
    }

    /// The 64-bit extended "si" register.
    #[inline]
    #[must_use]
    pub const fn rsi(&self) -> &Gpr {
        let &Self { ref rsi, .. } = self;

        rsi
    }

    /// Resolve a mutable reference to the 64-bit extended "si" register.
    #[inline]
    pub const fn rsi_mut(&mut self) -> &mut Gpr {
        let &mut Self { ref mut rsi, .. } = self;

        rsi
    }

    /// The 64-bit extended "di" register.
    #[inline]
    #[must_use]
    pub const fn rdi(&self) -> &Gpr {
        let &Self { ref rdi, .. } = self;

        rdi
    }

    /// Resolve a mutable reference to the 64-bit extended "di" register.
    #[inline]
    pub const fn rdi_mut(&mut self) -> &mut Gpr {
        let &mut Self { ref mut rdi, .. } = self;

        rdi
    }

    /// The 64-bit extended "bp" register.
    #[inline]
    #[must_use]
    pub const fn rbp(&self) -> &Gpr {
        let &Self { ref rbp, .. } = self;

        rbp
    }

    /// Resolve a mutable reference to the 64-bit extended "bp" register.
    #[inline]
    pub const fn rbp_mut(&mut self) -> &mut Gpr {
        let &mut Self { ref mut rbp, .. } = self;

        rbp
    }

    /// The 64-bit extended "8" register.
    #[inline]
    #[must_use]
    pub const fn r8(&self) -> &Gpr {
        let &Self { ref r8, .. } = self;

        r8
    }

    /// Resolve a mutable reference to the 64-bit extended "8" register.
    #[inline]
    pub const fn r8_mut(&mut self) -> &mut Gpr {
        let &mut Self { ref mut r8, .. } = self;

        r8
    }

    /// The 64-bit extended "9" register.
    #[inline]
    #[must_use]
    pub const fn r9(&self) -> &Gpr {
        let &Self { ref r9, .. } = self;

        r9
    }

    /// Resolve a mutable reference to the 64-bit extended "9" register.
    #[inline]
    pub const fn r9_mut(&mut self) -> &mut Gpr {
        let &mut Self { ref mut r9, .. } = self;

        r9
    }

    /// The 64-bit extended "10" register.
    #[inline]
    #[must_use]
    pub const fn r10(&self) -> &Gpr {
        let &Self { ref r10, .. } = self;

        r10
    }

    /// Resolve a mutable reference to the 64-bit extended "10" register.
    #[inline]
    pub const fn r10_mut(&mut self) -> &mut Gpr {
        let &mut Self { ref mut r10, .. } = self;

        r10
    }

    /// The 64-bit extended "11" register.
    #[inline]
    #[must_use]
    pub const fn r11(&self) -> &Gpr {
        let &Self { ref r11, .. } = self;

        r11
    }

    /// Resolve a mutable reference to the 64-bit extended "11" register.
    #[inline]
    pub const fn r11_mut(&mut self) -> &mut Gpr {
        let &mut Self { ref mut r11, .. } = self;

        r11
    }

    /// The 64-bit extended "12" register.
    #[inline]
    #[must_use]
    pub const fn r12(&self) -> &Gpr {
        let &Self { ref r12, .. } = self;

        r12
    }

    /// Resolve a mutable reference to the 64-bit extended "12" register.
    #[inline]
    pub const fn r12_mut(&mut self) -> &mut Gpr {
        let &mut Self { ref mut r12, .. } = self;

        r12
    }

    /// The 64-bit extended "13" register.
    #[inline]
    #[must_use]
    pub const fn r13(&self) -> &Gpr {
        let &Self { ref r13, .. } = self;

        r13
    }

    /// Resolve a mutable reference to the 64-bit extended "13" register.
    #[inline]
    pub const fn r13_mut(&mut self) -> &mut Gpr {
        let &mut Self { ref mut r13, .. } = self;

        r13
    }

    /// The 64-bit extended "14" register.
    #[inline]
    #[must_use]
    pub const fn r14(&self) -> &Gpr {
        let &Self { ref r14, .. } = self;

        r14
    }

    /// Resolve a mutable reference to the 64-bit extended "14" register.
    #[inline]
    pub const fn r14_mut(&mut self) -> &mut Gpr {
        let &mut Self { ref mut r14, .. } = self;

        r14
    }

    /// The 64-bit extended "15" register.
    #[inline]
    #[must_use]
    pub const fn r15(&self) -> &Gpr {
        let &Self { ref r15, .. } = self;

        r15
    }

    /// Resolve a mutable reference to the 64-bit extended "15" register.
    #[inline]
    pub const fn r15_mut(&mut self) -> &mut Gpr {
        let &mut Self { ref mut r15, .. } = self;

        r15
    }

    /// The 64-bit extended "ip" register.
    #[inline]
    #[must_use]
    pub const fn rip(&self) -> &Gpr {
        let &Self {
            ref interrupt_frame, ..
        } = self;

        interrupt_frame.rip()
    }

    /// Resolve a mutable reference to the 64-bit extended "ip" register.
    #[inline]
    pub const fn rip_mut(&mut self) -> &mut Gpr {
        let &mut Self {
            ref mut interrupt_frame,
            ..
        } = self;

        interrupt_frame.rip_mut()
    }

    /// The "cs" segment selector.
    #[inline]
    #[must_use]
    pub const fn cs(&self) -> &RawCodeSegment {
        let &Self {
            ref interrupt_frame, ..
        } = self;

        interrupt_frame.cs()
    }

    /// Resolve a mutable reference to the "cs" segment selector.
    #[inline]
    pub const fn cs_mut(&mut self) -> &mut RawCodeSegment {
        let &mut Self {
            ref mut interrupt_frame,
            ..
        } = self;

        interrupt_frame.cs_mut()
    }

    /// The architectural flags register.
    #[inline]
    #[must_use]
    pub const fn rflags(&self) -> &Rflags {
        let &Self {
            ref interrupt_frame, ..
        } = self;

        interrupt_frame.rflags()
    }

    /// Resolve a mutable reference to the architectural flags register.
    #[inline]
    pub const fn rflags_mut(&mut self) -> &mut Rflags {
        let &mut Self {
            ref mut interrupt_frame,
            ..
        } = self;

        interrupt_frame.rflags_mut()
    }

    /// The 64-bit extended "sp" register.
    #[inline]
    #[must_use]
    pub const fn rsp(&self) -> &Gpr {
        let &Self {
            ref interrupt_frame, ..
        } = self;

        interrupt_frame.rsp()
    }

    /// Resolve a mutable reference to the 64-bit extended "sp" register.
    #[inline]
    pub const fn rsp_mut(&mut self) -> &mut Gpr {
        let &mut Self {
            ref mut interrupt_frame,
            ..
        } = self;

        interrupt_frame.rsp_mut()
    }

    /// The "ss" segment selector.
    #[inline]
    #[must_use]
    pub const fn ss(&self) -> &RawDataSegment {
        let &Self {
            ref interrupt_frame, ..
        } = self;

        interrupt_frame.ss()
    }

    /// Resolve a mutable reference to the "ss" segment selector.
    #[inline]
    pub const fn ss_mut(&mut self) -> &mut RawDataSegment {
        let &mut Self {
            ref mut interrupt_frame,
            ..
        } = self;

        interrupt_frame.ss_mut()
    }

    /// Determine the Interrupt Return Frame contained within this trap context.
    #[inline]
    #[must_use]
    pub const fn interrupt_frame(&self) -> &IRetFrame {
        let &Self {
            ref interrupt_frame, ..
        } = self;

        interrupt_frame
    }

    /// Resolve a mutable reference to the Interrupt Return Frame contained
    /// within this trap context.
    #[inline]
    pub const fn interrupt_frame_mut(&mut self) -> &mut IRetFrame {
        let &mut Self {
            ref mut interrupt_frame,
            ..
        } = self;

        interrupt_frame
    }
}

mod detail {
    //! The context layout and its switch assembly.

    /// Store a GPR at its offset in the context addressed by `%rdi`.
    macro_rules! save {
        ($assembly_register:ident, $assembly_offset:ident) => {
            concat!(
                "movq %",
                stringify!($assembly_register),
                ", {",
                stringify!($assembly_offset),
                "}(%rdi)"
            )
        };
    }

    /// Declare the context and generate the transfer for its ordinary GPR fields.
    ///
    /// The grouped fields must name GPRs in the assembly. The segment bases and
    /// interrupt frame follow their own transfer rules, and `RAX` is restored last.
    macro_rules! context {
        (
            $(
                #[$target_meta:meta]
            )*

            $target_vis:vis struct $target_name:ident {
                $(#[$target_gs_meta:meta])*
                gsbase: Base,

                $(#[$target_fs_meta:meta])*
                fsbase: Base,

                become {
                    $(
                        $(#[$target_register_meta:meta])*
                        $target_register:ident: Gpr,
                    )+
                }

                $(#[$target_rax_meta:meta])*
                rax: Gpr,

                $(#[$target_frame_meta:meta])*
                interrupt_frame: IRetFrame,
            }
        ) => {
            tokel::stream!(
                $(#[$target_meta])*
                $target_vis struct $target_name {
                    $(#[$target_gs_meta])*
                    gsbase: Base,

                    $(#[$target_fs_meta])*
                    fsbase: Base,

                    $(
                        $(#[$target_register_meta])*
                        $target_register: Gpr,
                    )+
                    $(#[$target_rax_meta])*
                    rax: Gpr,

                    $(#[$target_frame_meta])*
                    interrupt_frame: IRetFrame,
                }

                #[expect(
                    clippy::missing_inline_in_public_items,
                    reason = "Naked functions cannot be made `inline`"
                )]
                impl $target_name {
                    /// Capture the current architectural context.
                    ///
                    /// The registers are captured at the SysV64 call boundary. `RIP` and `RSP`
                    /// describe the return to the caller. The image can be inspected or used to
                    /// initialize another context. Use [`Self::save`] to suspend this call for
                    /// later resumption.
                    ///
                    /// # Safety
                    ///
                    /// * The model-specific register delegate must execute at CPL0.
                    /// * When using the `FSGSBASE` delegate, the processor must support the
                    ///   instructions and `CR4.FSGSBASE` must be set.
                    #[must_use]
                    #[unsafe(naked)]
                    pub unsafe extern "sysv64" fn capture() -> Self {
                        arch::naked_asm!(
                            // NOTE: SysV64 passes the return storage for `Context` in `%rdi`.
                            detail::save!(rax, rax_offset),
                            $(
                                detail::save!($target_register, [< $target_register _offset >]:flatten:concatenate),
                            )+
                            "pushfq",
                            "popq {rflags_offset}(%rdi)",
                            "leaq {return_address_size}(%rsp), %rax",
                            "movq %rax, {rsp_offset}(%rdi)",
                            "movq (%rsp), %rax",
                            "movq %rax, {rip_offset}(%rdi)",
                            "movw %cs, {cs_offset}(%rdi)",
                            "movw %ss, {ss_offset}(%rdi)",

                            // NOTE: Keep the return storage across the delegate call and restore
                            // the caller's `%r12` before returning the storage address in `%rax`.
                            "movq %rdi, %r12",
                            "subq ${return_address_size}, %rsp",
                            "callq {read_segment_base}",
                            "movq %rax, {fs_base_offset}(%r12)",
                            "movq %rdx, {gs_base_offset}(%r12)",
                            "addq ${return_address_size}, %rsp",
                            "movq %r12, %rax",
                            "movq {r12_offset}(%rax), %r12",
                            "retq",

                            $(
                                [< $target_register _offset >]:flatten:concatenate = const mem::offset_of!(Self, $target_register),
                            )+
                            rax_offset = const mem::offset_of!(Self, rax),
                            gs_base_offset = const mem::offset_of!(Self, gsbase),
                            fs_base_offset = const mem::offset_of!(Self, fsbase),
                            rip_offset = const mem::offset_of!(Self, interrupt_frame.rip),
                            cs_offset = const mem::offset_of!(Self, interrupt_frame.cs),
                            rflags_offset = const mem::offset_of!(Self, interrupt_frame.rflags),
                            rsp_offset = const mem::offset_of!(Self, interrupt_frame.rsp),
                            ss_offset = const mem::offset_of!(Self, interrupt_frame.ss),
                            return_address_size = const mem::size_of::<Gpr>(),
                            read_segment_base = sym Patch::<ReadFsgsbaseDelegator, ()>::trampoline,
                            options(att_syntax)
                        );
                    }

                    /// Save the current context and enter the initial callback.
                    ///
                    /// The callback receives the saved context and the optional argument. Restoring
                    /// that context returns from this function. The callback uses the current stack,
                    /// and its execution ends when the saved context is restored.
                    ///
                    /// # Safety
                    ///
                    /// * The context storage must be aligned, exclusively writable, and live until
                    ///   this call returns. Its initial contents may be uninitialized.
                    /// * The callback must satisfy its argument's validity and lifetime requirements.
                    ///   It must release any resources needing cleanup before restoring this context.
                    /// * Resume the saved context once, while this call is suspended, following
                    ///   [`Self::switch`]. Rust unwinding must not cross the callback.
                    /// * Interrupt and trap handlers must tolerate the callback's stack and any
                    ///   segment-base changes made while the caller is suspended.
                    /// * The model-specific register delegate must execute at CPL0.
                    /// * When using the `FSGSBASE` delegate, the processor must support the
                    ///   instructions and `CR4.FSGSBASE` must be set.
                    #[unsafe(naked)]
                    pub unsafe extern "sysv64" fn save<T>(
                        target_context: NonNull<Self>,
                        target_initial: unsafe extern "sysv64" fn(NonNull<Self>, Option<NonNull<T>>) -> !,
                        target_argument: Option<NonNull<T>>,
                    ) {
                        arch::naked_asm!(
                            detail::save!(rax, rax_offset),
                            $(
                                detail::save!($target_register, [< $target_register _offset >]:flatten:concatenate),
                            )+
                            "pushfq",
                            "popq {rflags_offset}(%rdi)",
                            // NOTE: Resumption returns through the original call's return address.
                            "movq %rsp, {rsp_offset}(%rdi)",
                            "leaq 2f(%rip), %rax",
                            "movq %rax, {rip_offset}(%rdi)",
                            "movw %cs, {cs_offset}(%rdi)",
                            "movw %ss, {ss_offset}(%rdi)",

                            // NOTE: The delegate may clobber the callback and argument registers.
                            // Recover them from the saved image after reading the segment bases.
                            "movq %rdi, %r12",
                            "subq ${return_address_size}, %rsp",
                            "callq {read_segment_base}",
                            "movq %rax, {fs_base_offset}(%r12)",
                            "movq %rdx, {gs_base_offset}(%r12)",
                            "movq %r12, %rdi",
                            "movq {rsi_offset}(%rdi), %rax",
                            "movq {rdx_offset}(%rdi), %rsi",
                            "movq {r12_offset}(%rdi), %r12",
                            "callq *%rax",
                            "ud2",

                            "2:",
                            "retq",

                            $(
                                [< $target_register _offset >]:flatten:concatenate = const mem::offset_of!(Self, $target_register),
                            )+
                            rax_offset = const mem::offset_of!(Self, rax),
                            gs_base_offset = const mem::offset_of!(Self, gsbase),
                            fs_base_offset = const mem::offset_of!(Self, fsbase),
                            rip_offset = const mem::offset_of!(Self, interrupt_frame.rip),
                            cs_offset = const mem::offset_of!(Self, interrupt_frame.cs),
                            rflags_offset = const mem::offset_of!(Self, interrupt_frame.rflags),
                            rsp_offset = const mem::offset_of!(Self, interrupt_frame.rsp),
                            ss_offset = const mem::offset_of!(Self, interrupt_frame.ss),
                            return_address_size = const mem::size_of::<Gpr>(),
                            read_segment_base = sym Patch::<ReadFsgsbaseDelegator, ()>::trampoline,
                            options(att_syntax)
                        );
                    }

                    /// Suspend the current SysV64 call and resume the destination with `iretq`.
                    ///
                    /// The source receives the register image at entry, with the return address in
                    /// `RIP` and the post-call stack pointer in `RSP`. The call returns when a later
                    /// switch resumes that image.
                    ///
                    /// # Safety
                    ///
                    /// * The source must point to aligned, exclusively writable `Context` storage.
                    ///   The destination must be a separate, initialized `Context` with canonical
                    ///   FS and GS bases. Keep both accessible through the switch.
                    /// * Leave room for the `iretq` frame on the outgoing stack without overlapping
                    ///   either context. Keep the suspended call frame and its borrows alive. The
                    ///   return frame must be valid in the current address space and at the current
                    ///   privilege level.
                    /// * Enter at CPL0 when using the MSR delegates. When using the `FSGSBASE`
                    ///   delegates, the CPU must support the instructions and `CR4.FSGSBASE` must be set.
                    /// * Disable shadow stacks. Keep the floating-point control state and any other
                    ///   state outside `Context` valid for the resumed call.
                    /// * Any handler that can run during the switch must tolerate the new FS and GS
                    ///   bases on the old stack. Rust unwinding must not cross the switch.
                    #[unsafe(naked)]
                    pub unsafe extern "sysv64" fn switch(_source_context: *mut Self, _destination_context: *const Self) {
                        /// Load a GPR from its offset in the destination context.
                        macro_rules! restore {
                            ($assembly_register:ident, $assembly_offset:ident) => {
                                concat!("movq {", stringify!($assembly_offset), "}(%rax), %", stringify!($assembly_register))
                            };
                        }

                        arch::naked_asm!(
                            detail::save!(rax, rax_offset),
                            $(
                                detail::save!($target_register, [< $target_register _offset >]:flatten:concatenate),
                            )+

                            // NOTE: Save the caller's stack pointer past the return address at its top.
                            "pushfq",
                            "popq {rflags_offset}(%rdi)",
                            "leaq {return_address_size}(%rsp), %rax",
                            "movq %rax, {rsp_offset}(%rdi)",
                            "movq (%rsp), %rax",
                            "movq %rax, {rip_offset}(%rdi)",
                            "movw %cs, {cs_offset}(%rdi)",
                            "movw %ss, {ss_offset}(%rdi)",

                            // NOTE: The delegate calls may use the argument registers. Save both context
                            // pointers in callee-saved registers and align the outgoing stack for the calls.
                            "movq %rdi, %r12",
                            "movq %rsi, %r13",
                            "subq ${return_address_size}, %rsp",
                            "callq {read_segment_base}",
                            "movq %rax, {fs_base_offset}(%r12)",
                            "movq %rdx, {gs_base_offset}(%r12)",
                            "movq {fs_base_offset}(%r13), %rdi",
                            "movq {gs_base_offset}(%r13), %rsi",
                            "callq {write_segment_base}",
                            "addq ${return_address_size}, %rsp",

                            // NOTE: Build the return frame on the outgoing stack. The selector reads
                            // exclude Rust padding, and the pushes place `RIP` at the top.
                            "movq %r13, %rax",
                            "movzwq {ss_offset}(%rax), %rcx",
                            "pushq %rcx",
                            "pushq {rsp_offset}(%rax)",
                            "pushq {rflags_offset}(%rax)",
                            "movzwq {cs_offset}(%rax), %rcx",
                            "pushq %rcx",
                            "pushq {rip_offset}(%rax)",

                            $(
                                restore!($target_register, [< $target_register _offset >]:flatten:concatenate),
                            )+
                            restore!(rax, rax_offset),
                            "iretq",

                            $(
                                [< $target_register _offset >]:flatten:concatenate = const mem::offset_of!(Self, $target_register),
                            )+
                            rax_offset = const mem::offset_of!(Self, rax),
                            gs_base_offset = const mem::offset_of!(Self, gsbase),
                            fs_base_offset = const mem::offset_of!(Self, fsbase),
                            rip_offset = const mem::offset_of!(Self, interrupt_frame.rip),
                            cs_offset = const mem::offset_of!(Self, interrupt_frame.cs),
                            rflags_offset = const mem::offset_of!(Self, interrupt_frame.rflags),
                            rsp_offset = const mem::offset_of!(Self, interrupt_frame.rsp),
                            ss_offset = const mem::offset_of!(Self, interrupt_frame.ss),
                            return_address_size = const mem::size_of::<Gpr>(),
                            read_segment_base = sym Patch::<ReadFsgsbaseDelegator, ()>::trampoline,
                            write_segment_base = sym Patch::<WriteFsgsbaseDelegator, ()>::trampoline,
                            options(att_syntax)
                        );
                    }
                }
            );
        };
    }

    pub(in crate::arch::x86_64::context) use context;
    pub(in crate::arch::x86_64::context) use save;
}

#[cfg(test)]
#[cfg(usermode)]
mod tests {
    use core::{mem, ptr::NonNull};

    use nekor_aal_arch::x86_64::instruction::fsgsbase::{rdfsbase, rdgsbase};

    use super::{Base, Context, Gpr};

    /// The context storage and observations shared with the child.
    // NOTE(invariant): Both context pointers and the child stack stay live until the test finishes.
    struct Exchange {
        /// The context of the suspended test.
        target_parent: NonNull<Context>,
        /// The child's saved context.
        target_child: NonNull<Context>,
        /// The initial child stack pointer.
        target_stack_pointer: Gpr,
        /// The child's GS base.
        target_gsbase: Base,
        /// The number of times the child yielded.
        target_visit: u32,
        /// The GS base observed before yielding.
        target_observed_gsbase: Base,
    }

    /// Enter the child using a copy of the saved parent context.
    unsafe extern "sysv64" fn initial(
        target_context: NonNull<Context>,
        target_argument: Option<NonNull<Exchange>>,
    ) -> ! {
        let target_exchange = target_argument.expect("the test supplies an exchange").as_ptr();
        let mut target_discard = mem::MaybeUninit::<Context>::uninit();

        // SAFETY: `save` initialized the parent image. The exchange, child storage, and stack
        // belong to the suspended test. This callback has no resources requiring cleanup.
        unsafe {
            let mut target_child = target_context.as_ptr().read();
            *target_child.gsbase_mut() = (*target_exchange).target_gsbase;
            *target_child.rdi_mut() = Gpr::address(target_exchange.expose_provenance());
            *target_child.rip_mut() = Gpr::address((child_entry as *const ()).addr());
            *target_child.rsp_mut() = (*target_exchange).target_stack_pointer;
            let target_destination = (*target_exchange).target_child.as_ptr();
            target_destination.write(target_child);
            Context::switch(target_discard.as_mut_ptr(), target_destination);
        }

        std::process::abort();
    }

    /// Yield twice from the child's stack.
    unsafe extern "sysv64" fn child_entry(target_exchange: NonNull<Exchange>) -> ! {
        let target_exchange = target_exchange.as_ptr();

        for target_visit in 1..=2 {
            // SAFETY: The parent is suspended. The test keeps both contexts and stacks live,
            // and the target enables `FSGSBASE`.
            unsafe {
                (*target_exchange).target_visit = target_visit;
                (*target_exchange).target_observed_gsbase = Base::new(rdgsbase());
                Context::switch(
                    (*target_exchange).target_child.as_ptr(),
                    (*target_exchange).target_parent.as_ptr(),
                );
            }
        }

        std::process::abort();
    }

    #[test]
    fn switches_and_resumes_on_a_second_stack() {
        let mut target_parent = mem::MaybeUninit::<Context>::uninit();
        let mut target_child = mem::MaybeUninit::<Context>::uninit();
        let mut target_stack = Box::new([0_u8; 64 * 1024]);
        let target_stack_start = target_stack.as_mut_ptr().expose_provenance();
        let target_stack_end = target_stack_start + target_stack.len();

        // NOTE: A SysV64 function enters one word below a 16-byte stack boundary.
        let target_stack_pointer = (target_stack_end & !15) - mem::size_of::<usize>();
        // SAFETY: The test target enables `FSGSBASE`.
        let (target_fsbase, target_gsbase) = unsafe { (Base::new(rdfsbase()), Base::new(rdgsbase())) };
        let mut target_exchange = Exchange {
            target_parent: NonNull::from(&mut target_parent).cast(),
            target_child: NonNull::from(&mut target_child).cast(),
            target_stack_pointer: Gpr::address(target_stack_pointer),
            target_gsbase: Base::new(target_stack_start as u64),
            target_visit: 0,
            target_observed_gsbase: target_gsbase,
        };

        // SAFETY: The context storage and exchange stay live until both round trips finish.
        // The callback creates the child on its own stack and releases no owned resources.
        unsafe {
            Context::save(
                target_exchange.target_parent,
                initial,
                Some(NonNull::from(&mut target_exchange)),
            );
        }

        for target_visit in 1..=2 {
            assert_eq!(target_exchange.target_visit, target_visit);
            assert_eq!(target_exchange.target_observed_gsbase, target_exchange.target_gsbase);

            // SAFETY: The child yielded after saving its image. `save` initialized the parent.
            let (target_parent, target_child, target_observed_gsbase) = unsafe {
                (
                    target_parent.assume_init_ref(),
                    target_child.assume_init_ref(),
                    Base::new(rdgsbase()),
                )
            };
            assert_eq!(target_observed_gsbase, target_gsbase);
            assert_eq!(target_parent.fsbase(), &target_fsbase);
            assert_eq!(target_parent.gsbase(), &target_gsbase);
            assert_eq!(target_child.fsbase(), &target_fsbase);
            assert_eq!(target_child.gsbase(), &target_exchange.target_gsbase);
            assert!(
                (target_stack_start..target_stack_end).contains(&(target_child.rsp().unwrap() as usize)),
                "the child must yield from its own stack"
            );

            if target_visit == 1 {
                // SAFETY: Both images are initialized and the child's suspended call is live.
                unsafe {
                    Context::switch(
                        target_exchange.target_parent.as_ptr(),
                        target_exchange.target_child.as_ptr(),
                    );
                }
            }
        }
    }

    /// Resume the saved caller after receiving an absent argument.
    unsafe extern "sysv64" fn resume(target_context: NonNull<Context>, target_argument: Option<NonNull<()>>) -> ! {
        assert!(
            target_argument.is_none(),
            "the callback must receive the absent argument"
        );
        let mut target_discard = mem::MaybeUninit::<Context>::uninit();

        // SAFETY: `save` suspended the caller on this stack. Returning to it ends this callback.
        unsafe { Context::switch(target_discard.as_mut_ptr(), target_context.as_ptr()) };
        std::process::abort();
    }

    #[test]
    fn saves_and_resumes_without_an_argument() {
        let mut target_context = mem::MaybeUninit::<Context>::uninit();

        // SAFETY: The storage is exclusively writable. The callback resumes the caller once.
        unsafe { Context::save(NonNull::from(&mut target_context).cast(), resume, None) };
    }
}
