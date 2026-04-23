//! Patchsite and diversion template selection.

use core::{ffi, fmt, marker, mem, pin::Pin, ptr::NonNull};

use nekor_aal_agnostic::rel_ptr::RelPtrMut;

use crate::patch::{
    delegate::{Delegated, Delegator},
    patch::{Patch, Patchsite, Template},
};

/// A chosen [`Delegated`] template for a specific [`Delegator`] `D`.
#[repr(transparent)]
pub struct Chosen<D, I, O>(fn(I) -> O, marker::PhantomData<fn() -> D>)
where
    D: Delegator + ?Sized,
    D::Target: Delegated<Input = I, Output = O>;

impl<D, I, O> Chosen<D, I, O>
where
    D: Delegator + ?Sized,
    D::Target: Delegated<Input = I, Output = O>,
{
    /// Construct a new [`Chosen`] selection for the target [`Delegated`] `U`.
    #[inline]
    pub const fn delegated<U>() -> Self
    where
        U: Delegated<Input = I, Output = O>,
    {
        Self(U::implementation, marker::PhantomData)
    }

    /// Construct a [`Template`] from this [`Chosen`].
    #[inline]
    #[must_use]
    pub const fn template(self) -> Template<D> {
        let Self(target_value, ..) = self;

        Template(target_value, marker::PhantomData)
    }

    /// Determine the [`Patchsite`] for the [`Delegator`].
    #[inline]
    #[must_use]
    pub fn patchsite() -> Pin<&'static Patchsite<D, I, O>> {
        let target_patchsite: unsafe extern "sysv64-unwind" fn(
            <D::Target as Delegated>::Input,
        )
            -> <D::Target as Delegated>::Output = Patch::<D, D::Value>::patchsite;
        let target_patchsite: *const ffi::c_void = target_patchsite as *const ffi::c_void;

        let target_address: *mut RelPtrMut<Patchsite<D, I, O>> = target_patchsite.cast_mut().cast();

        // SAFETY: The pointer has been sourced from a function pointer, which
        // cannot be null.
        let target_address = unsafe {
            let aligned_address_offset = target_address
                .cast::<u8>()
                .align_offset(mem::align_of::<RelPtrMut<Patchsite<D, I, O>>>());

            NonNull::new_unchecked(target_address.wrapping_byte_add(aligned_address_offset))
        };

        // SAFETY: The pointer is aligned and will be valid for the rest of the
        // program, as it resides in a handwritten inline assembler section.
        let target_value = unsafe { target_address.as_ref() };

        let relative_pointer = Pin::static_ref(target_value);

        let patchsite_address = RelPtrMut::base(relative_pointer);

        Pin::static_ref(
            // SAFETY: The base-relative pointer cannot be self-referential and
            // the pointer points to valid, aligned memory for
            // `Patchsite`.
            unsafe { patchsite_address.as_ref().unwrap_unchecked() },
        )
    }
}

impl<D, I, O> Copy for Chosen<D, I, O>
where
    D: Delegator + ?Sized,
    D::Target: Delegated<Input = I, Output = O>,
{
}

impl<D, I, O> Clone for Chosen<D, I, O>
where
    D: Delegator + ?Sized,
    D::Target: Delegated<Input = I, Output = O>,
{
    #[inline]
    fn clone(&self) -> Self {
        let &Self(target_left, target_right) = self;

        Self(target_left, target_right)
    }
}

impl<D, I, O> fmt::Debug for Chosen<D, I, O>
where
    D: Delegator + ?Sized,
    D::Target: Delegated<Input = I, Output = O>,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Chosen").finish_non_exhaustive()
    }
}
