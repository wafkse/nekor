//! - *Machine-Stop Critical Sections* (*`MsCS`*):
//!     - Machine-wide interrupt disable.
//!     - Places all other cores in a `halt` loop or makes them momentarily offline.
//!
//! Moderate delay, used for microcode updates and microarchitectural
//! instruction set modifications.
//!
//! ## How does it work?
//!
//! When *`MsCS`* is acquired, the initiating core broadcasts an *IPI*
//! (*Interprocessor Interrupt*) to all other cores.
//!
//! Each core receives the interrupt and immediately transitions to a halt
//! state. They spin in a tight loop waiting for release.
//!
//! Unlike *`SpCS`*, cores do not wait for stablepoints. They halt immediately
//! upon receiving the *IPI*, regardless of their current state.
//!
//! This provides faster synchronization than *`SpCS`* but is more intrusive.
//! Cores are forcibly interrupted rather than reaching safe quiescent points.
//!
//! ## When to use *`MsCS`*?
//!
//! *`MsCS`* is appropriate when:
//!
//! - The machine must stop immediately (microcode updates, critical hardware reconfig)
//! - Stablepoint-based synchronization is too slow
//! - All cores must be in a known state simultaneously
//!
//! *`MsCS`* should not be used when:
//!
//! - Code patching is needed (use *`SpCS`* instead, safer for self-modifying code)
//! - Fine-grained locking suffices (use *GCS* or *LCS*)
//! - The operation can wait for stablepoints

/// An uninhabited type representing a *Machine-Stop Critical Section*.
pub enum MsCs {}
