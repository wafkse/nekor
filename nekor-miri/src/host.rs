//! Host path conversion available with Miri isolation disabled.

use core::ffi::CStr;

use crate::ffi;

/// Required output bytes, including the terminating NUL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Required {
    /// Number of bytes requested by Miri.
    pub bytes: usize,
}

/// Convert a host path into target path syntax in a caller-owned buffer.
///
/// Miri isolation must be disabled for this operation.
pub fn path<'a>(input: &CStr, output: &'a mut [u8]) -> Result<&'a CStr, Required> {
    // SAFETY: Input is NUL-terminated and output has the supplied writable length.
    let needed = unsafe { ffi::miri_host_to_target_path(input.as_ptr(), output.as_mut_ptr().cast(), output.len()) };
    if needed != 0 {
        return Err(Required { bytes: needed });
    }

    // SAFETY: On success Miri writes a NUL-terminated C string into the output buffer.
    Ok(unsafe { CStr::from_ptr(output.as_ptr().cast()) })
}
