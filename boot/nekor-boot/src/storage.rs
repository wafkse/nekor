//! Boot Protocol container primitives.

use core::{
    cell::UnsafeCell,
    marker,
    mem::{self, ManuallyDrop, MaybeUninit},
};

use nekor_bitwise::prelude::Le;

use crate::protocol::{BootRequest, Slug};

/// The head structure of a request-response structure.
#[repr(transparent)]
pub struct Head<R>(RawHead, marker::PhantomData<R>)
where
    R: BootRequest;

impl<R> Head<R>
where
    R: BootRequest,
{
    /// Construct a [`Head`] for the target bootloader request `R`.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self(RawHead::new::<R>(), marker::PhantomData::<R>)
    }
}

impl<R> Default for Head<R>
where
    R: BootRequest,
{
    fn default() -> Self {
        Self::new()
    }
}

/// The head structure of a request-response structure.
///
/// This is a low-level structure, and has no specific [`BootRequest`]
/// associated with it.
#[repr(C)]
pub struct RawHead {
    /// The slug identifier of the particular value stored in the encompassed
    /// union.
    slug_id: Slug,

    /// The length in bytes of the totality of the [`Block`] encompassing
    /// the target bootloader request.
    block_size: Le<u64>,
}

impl RawHead {
    /// Construct a [`RawHead`] for the target bootloader request `R`.
    #[inline]
    #[must_use]
    pub const fn new<R>() -> Self
    where
        R: BootRequest,
    {
        let slug_id = R::ID;

        let block_size = Le::<u64>::new(mem::size_of::<Block<R>>() as u64);

        Self { slug_id, block_size }
    }
}

/// A request-response block record.
///
/// This encompasses all state required for bootloader functionality.
#[repr(C)]
pub struct Block<R>
where
    R: BootRequest,
{
    /// The fixed-size header of this request-response block.
    // NOTE(invariant): The layout of this type requires this field to serve as a header.
    block_head: Head<R>,

    /// The request-or-response storage union.
    ///
    /// The underlying value is dictated by the value of the
    /// [`Block::block_head`] header.
    // NOTE(invariant): This is to guarantee that the static memory region is marked as
    // non-`Freeze`, as a `static mut` could be downgraded to a read-only program section when
    // there is no visible program-side write. The response is written in-place before the
    // Rust abstract machine is started and never again during its execution, so it can be used
    // as regular mutable unaliased memory afterwards via a mutable reference.
    block_storage: UnsafeCell<Storage<R>>,
}

/// A storage union type for the request and response structures of a bootloader
/// Request-Response block.
pub union Storage<R>
where
    R: BootRequest,
{
    /// The request made to the bootloader.
    request: ManuallyDrop<MaybeUninit<R>>,

    /// The response provided by the bootloader.
    response: ManuallyDrop<MaybeUninit<R::Response>>,
}

impl<R> Storage<R>
where
    R: BootRequest,
{
    /// Construct a bootloader [`Storage`] that encompasses a singular
    /// constructed request `R`.
    #[inline]
    pub const fn request(request: R) -> Self {
        let request = ManuallyDrop::new(MaybeUninit::new(request));

        Self { request }
    }

    /// Construct a bootloader [`Storage`] that encompasses a singular
    /// constructed response `R`.
    #[inline]
    pub const fn response(response: R::Response) -> Self {
        let response = ManuallyDrop::new(MaybeUninit::new(response));

        Self { response }
    }
}

impl<R> Block<R>
where
    R: BootRequest,
{
    /// Construct a bootloader request.
    #[inline]
    #[must_use = "this request is useless if not embedded into the appropiate program section"]
    pub const fn request(target_value: R) -> Self {
        Self {
            block_head: Head::<R>::new(),
            block_storage: UnsafeCell::new(Storage::<R>::request(target_value)),
        }
    }

    /// Construct a bootloader response.
    #[inline]
    #[must_use = "this response is useless if not embedded into the appropiate program section"]
    pub const fn reponse(target_value: R::Response) -> Self {
        Self {
            block_head: Head::<R>::new(),
            block_storage: UnsafeCell::new(Storage::<R>::response(target_value)),
        }
    }
}
