//! Architecture-agnostic cacheline information.

use core::mem;

use crate::padded::CachePadded;

/// A cacheline dummy.
///
/// This is a dummy structure to take up a single architecture-specific cache
/// line.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
#[repr(transparent)]
pub struct Cacheline(
    // NOTE(hack): Use the alignment of `CachePadded` for the size itself.
    CachePadded<[u8; mem::align_of::<CachePadded<()>>()]>,
);
