//! Baseline *Boot Protocol* declaration.

use nekor_bitwise::prelude::Le;

/// The slug of a boot-protocol [`Block`].
///
/// This is used to identify a particular [`Block`] dynamically.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Slug(Le<u64>);

impl Slug {
    /// Determine the [`Slug`] value that corresponds to a standalone buffer.
    #[inline]
    pub const fn standalone<const N: usize>(target_buffer: [u8; N]) -> Self {
        // NOTE: These parameters were sourced from `https://en.wikipedia.org/wiki/Fowler%E2%80%93Noll%E2%80%93Vo_hash_function#FNV-1a_hash`.

        /// The FNV offset-basis used for the `FNV-1a` hash.
        const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;

        /// The FNV prime used for the `FNV-1a` hash.
        const FNV_PRIME: u64 = 0x100000001b3;

        let mut hash_state = FNV_OFFSET_BASIS;

        {
            let mut target_index = 0;

            while target_index < N {
                let hash_byte = target_buffer[target_index];

                hash_state ^= hash_byte as u64;
                hash_state = hash_state.wrapping_mul(FNV_PRIME);

                target_index += 1;
            }
        }

        Self(Le::<u64>::new(hash_state))
    }
}

impl PartialEq for Slug {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0.raw() == other.0.raw()
    }
}

impl Eq for Slug {}

/// A marker trait that describes a singular protocol structure.
///
/// # Safety
///
/// * The implementor type of this trait is required to be `repr(C)`, for a
///   stable Application Binary Interface.
pub unsafe trait Protocol {}

/// A trait that describes a boot-protocol request-response structure sequence.
///
/// # Safety
///
/// * The associated slug must be unique.
pub unsafe trait BootRequest: Protocol {
    /// The unique [`Slug`] that identifies this bootloader request.
    const ID: Slug;

    /// The type of the response type block.
    type Response: BootResponse;
}

/// A marker trait that describles a boot-protocol response structure.
///
/// # Safety
///
/// * The associated slug must be unique.
pub unsafe trait BootResponse: Protocol {
    /// The unique [`Slug`] that identifies this bootloader response.
    const ID: Slug;
}
