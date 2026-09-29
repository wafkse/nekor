//! Private Miri interpreter ABI.
//!
//! Signatures and symbol contracts follow [Miri's extern declarations] at
//! revision `6185268230bb44cbf00348e69e3d8456bc7e2ddb`. The corresponding
//! [shim dispatch] checks the allocation, diagnostic, thread, and output ABIs.
//! The installed Rust backtrace shim confirms the four fixed flag values below.
//!
//! [Miri's extern declarations]: https://github.com/rust-lang/miri/blob/6185268230bb44cbf00348e69e3d8456bc7e2ddb/tests/utils/miri_extern.rs
//! [shim dispatch]: https://github.com/rust-lang/miri/blob/6185268230bb44cbf00348e69e3d8456bc7e2ddb/src/shims/foreign_items.rs

use core::ffi::c_char;

use crate::trace::MiriFrame;

/// Count frames without writing a buffer.
pub(crate) const BACKTRACE_SIZE: u64 = 0;
/// Write opaque tokens to the supplied buffer.
pub(crate) const GET_BACKTRACE: u64 = 1;
/// Resolve an opaque token to the C return image.
pub(crate) const RESOLVE_FRAME: u64 = 1;
/// Write the resolved names to caller-owned buffers.
pub(crate) const RESOLVE_NAMES: u64 = 0;

// Miri's raw pointer types erase their pointee. The wrappers cast typed pointers at this boundary.
unsafe extern "Rust" {
    /// Miri-provided extern function to allocate memory from the
    /// interpreter.
    ///
    /// This is useful when no fundamental way of allocating memory is
    /// available, e.g. when using `no_std` + `alloc`.
    /// The returned bytes are uninitialized and the request must use a valid Rust layout.
    pub(crate) fn miri_alloc(size: usize, align: usize) -> *mut ();

    /// Deallocate interpreter memory at its base with the original size and alignment.
    pub(crate) fn miri_dealloc(ptr: *mut (), size: usize, align: usize);

    /// Miri-provided extern function to mark the block `ptr` points to as a
    /// "root" for some static memory. This memory and everything
    /// reachable by it is not considered leaking even if it still
    /// exists when the program terminates.
    ///
    /// `ptr` has to point to the beginning of an allocated block.
    pub(crate) fn miri_static_root(ptr: *const ());

    /// Report allocation events for the allocation identified by `ptr`.
    /// Miri discovers the allocation ID from this pointer at runtime. The
    /// corresponding command-line option is `-Zmiri-track-alloc-id=<id>`.
    pub(crate) fn miri_track_alloc(ptr: *const ());

    /// Return the current frame count. The flag must be zero.
    pub(crate) fn miri_backtrace_size(flags: u64) -> usize;

    /// Write opaque frame tokens into a buffer with room for the reported frame count.
    /// The flag must be one. Tokens are inputs to `miri_resolve_frame`.
    pub(crate) fn miri_get_backtrace(flags: u64, buf: *mut *mut ());

    /// Resolve an opaque token. The flag must be one.
    /// Miri permits resolution on a different thread from capture.
    pub(crate) fn miri_resolve_frame(frame: *mut (), flags: u64) -> MiriFrame;

    /// Write UTF-8 function and file names for the retained token.
    /// Each buffer needs the corresponding length from `MiriFrame`. The flag must be zero.
    pub(crate) fn miri_resolve_frame_names(frame: *mut (), flags: u64, name: *mut (), filename: *mut ());

    /// Return Miri's allocation ID for a pointer with live allocation provenance.
    /// Invalid pointers abort interpretation. Capture the ID before printing because
    /// producing another pointer can change the borrow state. This shim is unstable.
    pub(crate) fn miri_get_alloc_id(ptr: *const ()) -> u64;

    /// Print interpreter borrow state for an allocation ID.
    /// Stacked Borrows prints tag stacks with the bottom tag at the left.
    /// Tree Borrows prints range permissions and tag trees.
    /// `show_unnamed` controls unnamed Tree Borrows tags and has no effect under
    /// Stacked Borrows. The output format and shim are unstable. Provenance GC can
    /// change which tags remain visible. `-Zmiri-provenance-gc=0` disables that GC.
    pub(crate) fn miri_print_borrow_state(id: u64, show_unnamed: bool);

    /// Name the selected parent of a pointer tag for Tree Borrows diagnostics.
    /// A nonzero parent index can name a tag that is no longer directly reachable.
    /// This operation is a no-op under Stacked Borrows.
    pub(crate) fn miri_pointer_name(ptr: *const (), parent: u8, name: &[u8]);

    /// Run provenance garbage collection at a chosen test point.
    pub(crate) fn miri_run_provenance_gc();

    /// Promise symbolic pointer alignment, checked by Miri against the address.
    /// Invalid alignment or an unaligned address fails interpretation. Concrete
    /// alignment checking does not use this promise.
    pub(crate) fn miri_promise_symbolic_alignment(ptr: *const (), align: usize);

    /// Spawn an interpreter thread with a Rust ABI callback and return its ID.
    /// The caller owns the callback data lifetime and cross-thread synchronization.
    pub(crate) fn miri_thread_spawn(callback: extern "Rust" fn(*mut ()), data: *mut ()) -> usize;

    /// Join a spawned interpreter thread. An invalid ID returns false.
    pub(crate) fn miri_thread_join(id: usize) -> bool;

    /// Notify Miri that this thread is spinning so another thread may run.
    /// Miri treats this as a yield hint.
    pub(crate) fn miri_spin_loop();

    /// Write borrowed bytes to the interpreter's stdout stream.
    pub(crate) fn miri_write_to_stdout(bytes: &[u8]);

    /// Write borrowed bytes to the interpreter's stderr stream.
    pub(crate) fn miri_write_to_stderr(bytes: &[u8]);

    /// Convert a NUL-terminated host path into target path syntax.
    /// Isolation must be disabled. `output` must have `size` writable bytes.
    /// The result includes a NUL terminator. Zero means success. Any other
    /// return value is the required buffer length.
    pub(crate) fn miri_host_to_target_path(input: *const c_char, output: *mut c_char, size: usize) -> usize;
}
