//! Interpreter calls used by the memory, borrow, and alignment helpers.
//!
//! These declarations follow [Miri's extern definitions]. See [Miri's license]
//! for the upstream terms.
//!
//! [Miri's extern definitions]: https://github.com/rust-lang/miri/blob/6185268230bb44cbf00348e69e3d8456bc7e2ddb/tests/utils/miri_extern.rs
//! [Miri's license]: https://github.com/rust-lang/miri/blob/6185268230bb44cbf00348e69e3d8456bc7e2ddb/LICENSE-MIT

// Wrappers erase the pointee type when calling these functions.
unsafe extern "Rust" {
    /// Allocate memory with the given size and alignment.
    ///
    /// The returned memory is uninitialized.
    pub(crate) fn miri_alloc(size: usize, align: usize) -> *mut ();

    /// Deallocate memory previously returned by `miri_alloc`.
    ///
    /// Use the original size and alignment.
    pub(crate) fn miri_dealloc(ptr: *mut (), size: usize, align: usize);

    /// Treat an allocation and everything reachable from it as a static root.
    ///
    /// `ptr` must point to the start of the allocation.
    pub(crate) fn miri_static_root(ptr: *const ());

    /// Track the allocation containing `ptr`.
    pub(crate) fn miri_track_alloc(ptr: *const ());

    /// Return the allocation ID for `ptr`.
    ///
    /// Miri aborts if `ptr` does not point into an allocation.
    pub(crate) fn miri_get_alloc_id(ptr: *const ()) -> u64;

    /// Print the borrow state for an allocation.
    ///
    /// `show_unnamed` includes unnamed borrows. The output format is unstable.
    pub(crate) fn miri_print_borrow_state(id: u64, show_unnamed: bool);

    /// Promise the given alignment for symbolic checks.
    ///
    /// Miri rejects an unaligned pointer or an alignment that is not a power of two.
    pub(crate) fn miri_promise_symbolic_alignment(ptr: *const (), align: usize);
}
