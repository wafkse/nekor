//! Checked access to interpreter backtraces.

use core::{mem::MaybeUninit, num::NonZeroU32, slice, str};

use crate::ffi;

/// The exact interpreter return image for a resolved frame.
///
/// Field order follows [Miri's declaration](https://github.com/rust-lang/miri/blob/6185268230bb44cbf00348e69e3d8456bc7e2ddb/tests/utils/miri_extern.rs).
#[repr(C)]
pub struct MiriFrame {
    /// UTF-8 function-name length.
    pub name_len: usize,

    /// UTF-8 filename length.
    pub filename_len: usize,

    /// One-based line, or zero when unavailable.
    pub lineno: u32,

    /// One-based column, or zero when unavailable.
    pub colno: u32,

    /// Executing function pointer, distinct from the frame token.
    pub fn_ptr: *mut (),
}

/// Opaque interpreter frame token.
#[derive(Clone, Copy)]
#[repr(transparent)]
// NOTE(invariant): The token is initialized by `miri_get_backtrace` before a `Frame` is exposed.
pub struct Frame(*mut ());

/// Captured initialized tokens in a caller-owned buffer.
pub struct Backtrace<'a> {
    /// Captured token slice.
    frames: &'a [Frame],
}

/// Required frame count for an undersized buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capacity {
    /// Number of tokens required by Miri.
    pub required: usize,
}

impl<'a> Backtrace<'a> {
    /// Capture the current interpreter stack into a caller-owned buffer.
    pub fn capture(buffer: &'a mut [MaybeUninit<Frame>]) -> Result<Self, Capacity> {
        // SAFETY: The fixed flag requests only a count.
        let count = unsafe { ffi::miri_backtrace_size(ffi::BACKTRACE_SIZE) };
        if buffer.len() < count {
            return Err(Capacity { required: count });
        }

        // SAFETY: Frame is transparent over a pointer and the buffer has space for count tokens.
        unsafe { ffi::miri_get_backtrace(ffi::GET_BACKTRACE, buffer.as_mut_ptr().cast::<*mut ()>()) };
        // SAFETY: Miri initializes exactly count pointer tokens on successful return.
        let frames = unsafe { slice::from_raw_parts(buffer.as_ptr().cast::<Frame>(), count) };
        Ok(Self { frames })
    }

    /// Inspect the captured opaque tokens.
    #[inline]
    pub const fn frames(&self) -> &[Frame] {
        let Self { frames } = self;

        frames
    }
}

/// A resolved frame retaining its opaque token for name retrieval.
// NOTE(invariant): `frame` is resolved from `token`, which is retained for name retrieval.
pub struct Resolved {
    /// The original frame token.
    token: Frame,

    /// The exact interpreter return image.
    frame: MiriFrame,
}

impl Frame {
    /// Resolve this token to source coordinates and name lengths.
    #[inline]
    pub fn resolve(self) -> Resolved {
        let Self(token) = self;

        // SAFETY: This token was initialized by the backtrace shim.
        let frame = unsafe { ffi::miri_resolve_frame(token, ffi::RESOLVE_FRAME) };
        Resolved { token: self, frame }
    }
}

/// Required byte counts for both name buffers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NameCapacity {
    /// Required function-name bytes.
    pub name: usize,

    /// Required filename bytes.
    pub filename: usize,
}

/// UTF-8 names borrowed from caller-owned buffers.
pub struct Names<'a> {
    /// Function name.
    pub name: &'a str,

    /// Source filename.
    pub filename: &'a str,
}

impl Resolved {
    /// Inspect the exact return image.
    #[inline]
    pub const fn frame(&self) -> &MiriFrame {
        let Self { frame, .. } = self;

        frame
    }

    /// Interpret zero as an unavailable line.
    #[inline]
    pub const fn line(&self) -> Option<NonZeroU32> {
        let Self { frame, .. } = self;

        NonZeroU32::new(frame.lineno)
    }

    /// Interpret zero as an unavailable column.
    #[inline]
    pub const fn column(&self) -> Option<NonZeroU32> {
        let Self { frame, .. } = self;

        NonZeroU32::new(frame.colno)
    }

    /// Write the UTF-8 names after checking both buffer lengths.
    pub fn names<'a>(&self, name: &'a mut [u8], filename: &'a mut [u8]) -> Result<Names<'a>, NameCapacity> {
        let Self { token, frame } = self;

        let required = NameCapacity {
            name: frame.name_len,
            filename: frame.filename_len,
        };
        if name.len() < required.name || filename.len() < required.filename {
            return Err(required);
        }

        // SAFETY: Both buffers have the exact required writable capacity and the token is retained.
        unsafe {
            ffi::miri_resolve_frame_names(
                token.0,
                ffi::RESOLVE_NAMES,
                name.as_mut_ptr().cast::<()>(),
                filename.as_mut_ptr().cast::<()>(),
            )
        };
        // SAFETY: The interpreter writes UTF-8 bytes of the lengths it reported.
        let name = unsafe { str::from_utf8_unchecked(&name[..required.name]) };
        // SAFETY: The interpreter writes UTF-8 bytes of the lengths it reported.
        let filename = unsafe { str::from_utf8_unchecked(&filename[..required.filename]) };
        Ok(Names { name, filename })
    }
}
