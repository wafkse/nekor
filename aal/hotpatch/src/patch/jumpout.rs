//! Jumpout-style hotpatchable zones.
//!
//! # An Introduction
//!
//! In Nekor, most hotpatchable code requires of an alternative implementation that depends on a
//! specific set of architectural features.
//!
//! This is done via the delegator + delegated implementation model, where a central delegator
//! transfers control to a dynamically selected implementation through a hotpatchable diversion.
//!
//! This is fine for cases where the used ABI is well-known and stable (which is the case of the
//! System V ABI), however, this may prove to be needlessly difficult in cases where the context
//! where use of hotpatching is being done during low-level and even assembly-level implementations,
//! particularly when constrained in the architectural registers available, as each delegate
//! nominally requires the System V ABI.
//!
//! This is important, as we'd like to isolate the patchzones from other regular kernel code, so
//! that it can be kept immutable, as well as avoiding the potential issues with rewriting more than
//! particularly necessary. Ideally, the hotpatching infrastructure should not even possess the
//! capability to modify ordinary Kernel Text.
//!
//! # Yes, whatever, but what the FUCK is a "jumpout"?
//!
//! A jumpout is mechanism similar to how inline alternatives in the Linux kernel are suppossed to
//! work, but without modifying principal kernel text directly. But, instead of patching the code
//! inplace, the code is forced to take a little "excursion" towards a patchsite. Fun, right?
//!
//! This is done via two symbols, and each hotpatchable site is explicitly selected:
//!
//! - The "Forward Edge" symbol: ".nekor.patch.jumpout.forward.{ID}", where `ID` is an unique
//!   unsigned integer, identifying the specific patchzone for this jumpout excursion.
//!
//! - The "Backward Edge" symbol: ".nekor.patch.jumpout.backward.{ID}", where ID conveys the same as
//!   aforementioned, identifying where to come back from the jumpout sequence.
//!
//! These two symbols need not to live on the same program section (a forward edge can live in a
//! patchzone symbol, and a backward edge in any Nekor Kernel Text section), but they may live
//! across widely different program segments (within the in-use code model, that is. E.g., take the
//! small code model on `x86-64`, where a `jmp rel32` can jump to any appropiately placed symbol, or
//! in an APX-enabled processor with 64-bit absolute `jmpabs`, where it may jump to anywhere on the
//! address space)
//!
//! An important distinction is that neither of these edges is, by
//! itself, subject to hotpatching. They establish the immutable
//! control-flow topology of a jumpout, within which the implementation
//! may be independently substituted.
//!
//! A jumpout therefore describes how a patchzone participates in an
//! existing execution stream, rather than how its contents are modified.
//!
//! ## Sure buddy, give me the code
//!
//! Suppose we're implementing a context-switch handler, and we'd like to
//! preserve the FS base of the outgoing context. Depending on the features
//! available on the processor, we may either use RDMSR or RDFSBASE.
//!
//! Now, suppose that RDI already points to the outgoing context, and that
//! RAX, RDX and RCX are available for us to clobber.
//!
//! Instead of rearranging our architectural state to accommodate an ABI
//! that we don't particularly care about, we simply do the following:
//!
//! ```asm
//!     ; Somewhere in regular, immutable Nekor Kernel Text.
//!
//!     jmp .nekor.patch.jumpout.forward.42
//!
//!     .global .nekor.patch.jumpout.backward.42
//!     .hidden .nekor.patch.jumpout.backward.42
//! .nekor.patch.jumpout.backward.42:
//!
//!     ; Carry on with the context switch.
//! ```
//!
//! Somewhere else, within a designated patchzone, we define the other
//! end of the excursion:
//!
//! ```asm
//!     .global .nekor.patch.jumpout.forward.42
//!     .hidden .nekor.patch.jumpout.forward.42
//! .nekor.patch.jumpout.forward.42:
//!
//!     mov ecx, 0xc0000100
//!     rdmsr
//!
//!     mov [rdi + 8], eax
//!     mov [rdi + 12], edx
//!
//!     jmp .nekor.patch.jumpout.backward.42
//! ```
//!
//! If FSGSBASE is available and enabled, we may instead arrange for the
//! patchzone to contain the following implementation:
//!
//! ```asm
//! .nekor.patch.jumpout.forward.42:
//!
//!     rdfsbase rax
//!     mov [rdi + 8], rax
//!
//!     jmp .nekor.patch.jumpout.backward.42
//! ```
//!
//! Notice what we haven't done here:
//!
//! - We haven't issued a CALL or a RET.
//! - We haven't touched the stack.
//! - We haven't rearranged any registers to satisfy a conventional ABI.
//! - We haven't modified the original Kernel Text.
//!
//! As far as the context-switch handler is concerned, the jumpout is
//! semantically equivalent to having the implementation written inline,
//! despite the implementation physically residing somewhere else.
//!
//! The architectural contract is established between the jumpout and
//! its consumer, rather than imposed by a conventional calling
//! convention.
//!
//! How the implementation within the patchzone is actually substituted
//! is an entirely separate concern, and is addressed on a case-by-case basis.
//!
//! # Notes
//!
//! At the moment, this is currently modeled for `x86-64`, but this seems achievable on other
//! ISAs without much change.

// TODO: Implement this SHIT
