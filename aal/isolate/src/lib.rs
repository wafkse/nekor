#![cfg_attr(not(any(usermode)), no_std)]
#![forbid(
    clippy::all,
    clippy::perf,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::panic,
    clippy::pedantic,
    rustdoc::all
)]
//! # Execution Context Isolates
//!
//! An [`Isolate`] provides a reentrant, hardware-backed execution environment.
//! It serves as the primitive for context-switching and preemption across the
//! kernel.
//!
//! In this model, an [`Isolate`] physically decouples the execution state from
//! the core scheduler, enforcing strict boundaries between the execution stack
//! and hardware context.
//!
//! ## Memory Separation
//!
//! The [`Isolate`] architecture mandates the separation of memory into distinct
//! regions to ensure zero-margin bounds and immunity to stack overflow:
//!
//! - **Stack Area**: A contiguous memory region exclusively dedicated to
//!   synchronous call frames and local variables.
//! - **CPU Context**: An out-of-band, statically allocated block managing the
//!   suspended processor state, including General Purpose Registers (GPRs) and
//!   extended states (e.g., `xsave`).
//! - **Interrupt Stacks**: Asynchronous hardware events are strictly routed to
//!   dedicated Interrupt Service Routine (ISR) stacks, guaranteeing zero
//!   interference with the isolate's stack area.
//!
//! ## Execution Lifecycle
//!
//! Entering an [`Isolate`] transfers processor control to the target context.
//! The execution yields back to the caller through two distinct pathways:
//!
//! - **Finished**: The isolate voluntarily suspended execution.
//! - **Interrupted**: The isolate was forcefully preempted by an asynchronous
//!   hardware event. The processor state is packed into the CPU context, and
//!   control is diverted back to the caller.
//!
//! ## Inter-Context Communication
//!
//! Communication across the isolate boundary is achieved via a zero-copy
//! bidirectional channel. A typed payload (`&mut MaybeUninit<T>`) is passed
//! directly through processor registers, maintaining type safety and avoiding
//! dynamic allocation.
//!
//! [`Isolate`]: crate::isolate::Isolate

pub mod arch;

pub mod isolate;

pub mod context;
