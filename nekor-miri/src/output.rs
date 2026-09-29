//! Interpreter output streams.

use crate::ffi;

/// Write bytes to interpreter stdout.
#[inline]
pub fn stdout(bytes: &[u8]) {
    // SAFETY: The borrowed bytes remain valid for the call.
    unsafe { ffi::miri_write_to_stdout(bytes) };
}

/// Write bytes to interpreter stderr.
#[inline]
pub fn stderr(bytes: &[u8]) {
    // SAFETY: The borrowed bytes remain valid for the call.
    unsafe { ffi::miri_write_to_stderr(bytes) };
}
