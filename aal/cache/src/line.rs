//! Architecture-agnostic cacheline information.

use core::mem;

use crate::padded::CachePadded;

/// A byte block whose size and alignment follow the target's cache-padding choice.
///
/// This supplies storage for line-oriented operations without measuring the processor's
/// actual cache-line geometry.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
#[repr(transparent)]
// NOTE(invariant): The wrapped padding occupies one target cache line at its required alignment.
pub struct Cacheline(
    // NOTE(hack): Use the alignment of `CachePadded` for the size itself.
    CachePadded<[u8; mem::align_of::<CachePadded<()>>()]>,
);
