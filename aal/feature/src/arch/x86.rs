//! `x86` architecture support.

use core::marker;

use nekor_bitwise::prelude::{Extract, Field, For2, Size};

use nekor_domain::{
    arch::InitializationStage,
    store::{Static, Store},
};
use nekor_primitive::scalar::Scalar;

#[cfg(target_arch = "x86")]
use core::arch::x86::{__cpuid_count, CpuidResult};

#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{__cpuid_count, CpuidResult};

pub mod qualified;

mod private {
    /// A sealed trait for [`Output`] use.
    ///
    /// [`Output`]: super::Output
    pub trait Sealed {}
}

/// A marker trait for the output of a `cpuid` invocation.
///
/// # Remarks
///
/// This trait is sealed.
pub trait Output: Scalar + private::Sealed {
    /// Grab the desired output from the target [`CpuidResult`].
    fn grab(target_result: &CpuidResult) -> u32;
}

// TODO: Move this trait to its own aalagnostic module.

/// A trait that describes a cacheable low-level operation.
pub trait Cached: Store + Sized {
    /// The output type for this cached type.
    type Output;

    /// Lazily perform the operation, automatically caching or retrieving the
    /// cached output otherwise.
    fn lazy() -> Self::Output;

    /// Attempt to retrieve the cached output.
    ///
    /// This will *NOT* attempt the operation if there is no cached output.
    fn cached() -> Option<Self::Output>;

    /// Unconditionally perform the operation, regardless of cache status.
    fn unconditional() -> Self::Output;
}

/// A newtype over a compile-time `cpuid` invocation.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct Cpuid<const LEAF: u32, const SUBLEAF: u32 = 0>(pub CpuidResult);

impl<const LEAF: u32, const SUBLEAF: u32> Cached for Cpuid<LEAF, SUBLEAF> {
    type Output = Self;

    #[inline]
    fn unconditional() -> Self {
        Self(__cpuid_count(LEAF, SUBLEAF))
    }

    #[inline]
    fn lazy() -> Self {
        *Static::value_with::<Self, _>(Self::unconditional)
    }

    #[inline]
    fn cached() -> Option<Self> {
        if let InitializationStage::Initialized = Static::raw_stage::<Self>() {
            Some(*Static::value_with::<Self, _>(|| unreachable!()))
        } else {
            None
        }
    }
}

/// A helper struct to access and further cache the result of a `cpuid`
/// invocation.
///
/// The const-generic pair (`LEAF`, `SUBLEAF`) correspond to the leaf and the
/// sub-leaf, respectively.
#[derive(Debug, Clone, Copy)]
pub struct CpuidReg<R, const LEAF: u32, const SUBLEAF: u32 = 0>(pub u32, marker::PhantomData<R>)
where
    R: Output;

impl<R, const LEAF: u32, const SUBLEAF: u32> Cached for CpuidReg<R, LEAF, SUBLEAF>
where
    R: Output,
{
    type Output = Self;

    #[inline]
    fn unconditional() -> Self {
        let Cpuid(ref target_value) = Cpuid::<LEAF, SUBLEAF>::unconditional();

        Self(R::grab(target_value), marker::PhantomData::<R>)
    }

    #[inline]
    fn lazy() -> Self {
        let Cpuid(ref target_value) = Cpuid::<LEAF, SUBLEAF>::lazy();

        Self(R::grab(target_value), marker::PhantomData::<R>)
    }

    #[inline]
    fn cached() -> Option<Self::Output> {
        if let Some(Cpuid(ref target_value)) = Cpuid::<LEAF, SUBLEAF>::cached() {
            Some(Self(R::grab(target_value), marker::PhantomData::<R>))
        } else {
            None
        }
    }
}

/// A helper macro to automatically implement the required output registers.
macro_rules! register {
    (
        $($target_vis:vis $target_name:ident),+ $(,)?
    ) => {
        tokel::stream!(
            $(
                #[doc = concat!("The `", stringify!([< $target_name >]:case[[snake]]), "` `cpuid` output register.")]
                #[derive(Debug, PartialOrd, Ord, Eq, PartialEq, Copy, Clone)]
                $target_vis enum [< $target_name >]:case[[pascal]] {}

                impl private::Sealed for [< $target_name >]:case[[pascal]] {}

                impl Output for [< $target_name >]:case[[pascal]] {
                    #[inline]
                    fn grab(&CpuidResult { [< $target_name >]:case[[snake]], .. }: &CpuidResult) -> u32 {
                        [< $target_name >]:case[[snake]]
                    }
                }
            )+
        );
    };
}

register!(pub Eax, pub Ebx, pub Ecx, pub Edx);

/// A bit-level field inside a `cpuid` output register.
#[repr(transparent)]
pub struct CpuidField<const N: usize, const M: usize, R, const LEAF: u32, const SUBLEAF: u32 = 0>(
    marker::PhantomData<R>,
)
where
    R: Output,
    Size: For2<N, M>,
    u32: Extract<N, M, Output = <Size as For2<N, M>>::Target>;

impl<const N: usize, const M: usize, R, const LEAF: u32, const SUBLEAF: u32> Cached
    for CpuidField<N, M, R, LEAF, SUBLEAF>
where
    R: Output,
    Size: For2<N, M>,
    u32: Extract<N, M, Output = <Size as For2<N, M>>::Target>,
{
    type Output = <Size as For2<N, M>>::Target;

    #[inline]
    fn unconditional() -> Self::Output {
        let CpuidReg(ref target_value, ..) = CpuidReg::<R, LEAF, SUBLEAF>::unconditional();

        let ref target_field = Field::wrap(target_value);

        Field::value(target_field)
    }

    #[inline]
    fn lazy() -> Self::Output {
        let CpuidReg(ref target_value, ..) = CpuidReg::<R, LEAF, SUBLEAF>::lazy();

        let ref target_field = Field::wrap(target_value);

        Field::value(target_field)
    }

    #[inline]
    fn cached() -> Option<Self::Output> {
        if let Some(CpuidReg(ref target_value, ..)) = CpuidReg::<R, LEAF, SUBLEAF>::cached() {
            let ref target_field = Field::wrap(target_value);

            Some(Field::value(target_field))
        } else {
            None
        }
    }
}
