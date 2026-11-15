//! Type-level wrappers for low-level architectural entry.

use core::marker;

/// A marker trait that describes low-level contextual entry.
///
/// # Safety
///
/// This must be implemented by either the [`LowLevel`] type or a wrapper over
/// [`LowLevel`].
///
/// See the [`LowLevel`] documentation for further information regarding
/// wrappers.
pub unsafe trait Contextual {}

/// A low-level entry token with the ABI of its payload.
///
/// Safe Rust has no constructor for this token because its marker field is private. An
/// assembly entry can nevertheless pass the payload in the registers or stack slots that
/// the ABI assigns to `T`. The entry must establish a valid Rust value and the machine state
/// required by its handler before transferring control to Rust.
///
/// # Entry-specific tokens
///
/// This type achieves a strict safety guarantee across the entire codebase
/// through a combination of Rust's privacy system and Application Binary
/// Interface (ABI) guarantees:
///
/// 1. **Unforgeability:** The internal data is completely private. Because there is no public
///    constructor, it is mathematically impossible for safe Rust code to construct a `LowLevel<T>`.
/// 2. **Hardware Transparency:** Because it is marked `#[repr(transparent)]`, the struct is
///    binary-compatible with `T`.
///
/// This combination creates an "Assembly Loophole." When a naked assembly
/// stub executes a transition (e.g., reading an error code into `%rdi` and
/// issuing a `call`), the hardware moves the raw bits, and the Rust compiler
/// interprets that register directly as `LowLevel<T>`.
///
/// Therefore, if an internal function requires `LowLevel<T>` as an input,
/// the compiler guarantees the caller is either a trusted, `unsafe`
/// constructor within the privileged module, or a direct hardware transition.
///
/// # Isolating Contexts via Wrapper Types
///
/// While `LowLevel<T>` guarantees that data originated from *some* hardware
/// entry point, not all architectural entries are equal. For example, an
/// Interrupt handler and a Context Switch routine are both "low-level", but
/// they have entirely different invariants regarding stack layout, segment
/// registers, and CPU flags.
///
/// To prevent cross-contamination (e.g., passing an exception token to a
/// syscall handler), `LowLevel<T>` should rarely be used directly in function
/// signatures. Instead, it must be encapsulated in **Context-Specific
/// Wrappers**.
///
/// By defining a newtype wrapper around `LowLevel<T>`, you create an
/// unforgeable token tied to a specific execution domain:
///
/// ```
/// use core::marker::PhantomData;
///
/// use nekor_aal_agnostic::low_level::{Contextual, LowLevel};
///
/// /// Payload supplied by an NMI entry.
/// #[repr(transparent)]
/// // NOTE(invariant): Construction is restricted to the NMI entry path.
/// pub struct NmiContext<T>(LowLevel<T>, PhantomData<Self>);
///
/// // SAFETY: This token has the ABI of `LowLevel<T>` because it is transparent
/// // and its additional marker occupies no storage.
/// unsafe impl<T> Contextual for NmiContext<T> {}
/// ```
///
/// # Constraints
///
/// The token is covariant over `T` and is neither `Send` nor `Sync`. This prevents safe Rust
/// from sharing it between threads. Processor affinity, privilege level and register setup
/// still depend on the entry path. The token does not inspect or install that machine state.
#[repr(transparent)]
// NOTE(invariant): The private marker restricts construction to low-level entry paths.
pub struct LowLevel<T>(
    pub T,
    // NOTE(variance): Force `LowLevel` to be covariant over `T`.
    marker::PhantomData<*mut ()>,
);

impl<T> LowLevel<T> {
    /// Borrow the payload established by the architectural entry.
    #[inline]
    #[must_use]
    pub const fn value(&self) -> &T {
        let Self(target_value, ..) = self;

        target_value
    }

    /// Consume the entry proof and extract its payload.
    #[inline]
    #[must_use]
    pub fn take(self) -> T {
        let Self(target_value, ..) = self;

        target_value
    }
}

// SAFETY: `LowLevel` is the base entry representation required by `Contextual`.
// Its transparent layout gives it the ABI of the payload passed by the entry path.
unsafe impl<T> Contextual for LowLevel<T> {}
