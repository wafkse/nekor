//! Interpreter provenance and symbolic alignment controls.

use crate::ffi;

/// A checked, nonzero power-of-two alignment.
#[derive(Clone, Copy, PartialEq, Eq)]
// NOTE(invariant): The stored byte count is a nonzero power of two.
pub struct Alignment(usize);

impl Alignment {
    /// Accept only nonzero powers of two.
    #[inline]
    pub const fn new(bytes: usize) -> Option<Self> {
        if bytes.is_power_of_two() {
            Some(Self(bytes))
        } else {
            None
        }
    }

    /// Return the checked byte alignment.
    #[inline]
    pub const fn bytes(&self) -> usize {
        let &Self(bytes) = self;

        bytes
    }
}

/// Ask Miri to verify actual pointer alignment for symbolic checks.
#[inline]
#[expect(
    clippy::not_unsafe_ptr_arg_deref,
    reason = "Miri checks the address and never dereferences the pointer"
)]
pub fn promise(ptr: *const (), alignment: Alignment) {
    let bytes = alignment.bytes();

    // SAFETY: Alignment has a valid shape. Miri checks the pointer's actual address.
    unsafe { ffi::miri_promise_symbolic_alignment(ptr, bytes) };
}

/// Run the interpreter's provenance garbage collection.
#[inline]
pub fn collect() {
    // SAFETY: The shim has no caller-side preconditions.
    unsafe { ffi::miri_run_provenance_gc() };
}
