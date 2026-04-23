//! Deterministic implementation delegation.

use core::{convert, marker};

use crate::patch::{choose::Chosen, pod::Pod};

/// A trait that describes a *delegated implementation* for an input-to-output
/// transformation.
///
/// # Safety
///
/// The implementor of this trait may require further safety constraints to be
/// upheld before usage.
///
/// See each respective implementation for the safety contract.
///
/// <div class="warning">
///
/// ## Generic [`Delegated`] Implications
///
/// Due to each individual implementation having delegate safety requirements,
/// it must be guarded against when this trait is used in generic code.
/// </div>
pub unsafe trait Delegated: Pod {
    /// The input type to this [`Delegated`].
    type Input;

    /// The output type of this [`Delegated`].
    type Output;

    /// The implementation that was actually delegated.
    fn implementation(target_param: Self::Input) -> Self::Output;

    /// A trampoline to the [`implementation`] of this [`Delegated`] that uses
    /// the `extern "sysv64"` (System V ABI) calling convention.
    ///
    /// This is to be used for callers that cannot
    /// directly make a call to an `extern "Rust"` (native) ABI function, such
    /// as low-level architectural entry to high-level Rust code.
    ///
    /// [`implementation`]: Delegated::implementation
    #[inline]
    extern "sysv64" fn trampoline(target_param: Self::Input) -> Self::Output {
        Self::implementation(target_param)
    }
}

/// A trait that describes a mapping between a specific plain-old-data type and
/// one or more [`Delegated`] implementations.
///
/// # Safety
///
/// For a delegator to be deemed safe, it must satisfy all of the following
/// requirements:
///
/// * The selection based on the input value must be performed in a way that it
///   is deterministic and side-effect free. Particularly, [`Delegator::choose`]
///   must be a *pure function*.
/// * If an output delegate can cause Undefined Behavior due to missmatching
///   machine state or missing architectural features, the selection of such
///   delegate must be guarded from. For instance, if a delegate has a baseline
///   requirement on a specific architectural feature, the respective delegator
///   must guard and verify the existence of such feature.
pub unsafe trait Delegator: Pod {
    /// The type of delegated implementation that is a target of this
    /// [`Delegator`].
    ///
    /// This will serve as the default [`Delegated::implementation`].
    type Target: Delegated;

    /// The value type of the plain dependency of this [`Delegator`].
    type Value: Pod;

    /// Choose a [`Delegated`] that has the same interface as
    /// [`Delegator::Target`].
    fn choose(
        target_value: &'static Self::Value,
    ) -> Chosen<Self, <Self::Target as Delegated>::Input, <Self::Target as Delegated>::Output>;
}

/// A [`Delegator`] that encompasses a single [`Delegated`] and a target value.
#[derive(Debug, Copy, Clone)]
pub struct Single<D, P>(convert::Infallible, marker::PhantomData<fn() -> (D, P)>)
where
    D: Delegated,
    P: Pod;

// SAFETY: The choosing process is guaranteed to be pure, as there is a single
// `Delegated` to choose from.
unsafe impl<D, P> Delegator for Single<D, P>
where
    D: Delegated,
    P: Pod,
{
    type Target = D;

    type Value = P;

    #[inline]
    fn choose(_: &'static Self::Value) -> Chosen<Self, D::Input, D::Output> {
        Chosen::delegated::<D>()
    }
}

/// A [`Delegator`] that selects between two [`Delegated`]s based on a logical
/// condition.
///
/// The default [`Delegate`] is defined by the provided const-generic parameter.
///
/// - If it is `true`, the first [`Delegated`] (`T`) is chosen.
/// - If it is `false`, the second [`Delegated`] (`F`) is chosen.
#[derive(Debug, Copy, Clone)]
pub struct Logical<T, F, const S: bool>(marker::PhantomData<(T, F)>)
where
    T: Delegated,
    F: Delegated<Input = T::Input, Output = T::Output>;

// SAFETY: Implementation is based in a pure, logical relationship.
unsafe impl<T, F> Delegator for Logical<T, F, false>
where
    T: Delegated,
    F: Delegated<Input = T::Input, Output = T::Output>,
{
    type Target = F;

    type Value = bool;

    #[inline]
    fn choose(
        &target_value: &'static Self::Value,
    ) -> Chosen<Self, <Self::Target as Delegated>::Input, <Self::Target as Delegated>::Output> {
        if target_value {
            Chosen::delegated::<T>()
        } else {
            Chosen::delegated::<F>()
        }
    }
}

// SAFETY: Implementation is based in a pure, logical relationship.
unsafe impl<T, F> Delegator for Logical<T, F, true>
where
    T: Delegated,
    F: Delegated<Input = T::Input, Output = T::Output>,
{
    type Target = T;

    type Value = bool;

    #[inline]
    fn choose(
        &target_value: &'static Self::Value,
    ) -> Chosen<Self, <Self::Target as Delegated>::Input, <Self::Target as Delegated>::Output> {
        if target_value {
            Chosen::delegated::<T>()
        } else {
            Chosen::delegated::<F>()
        }
    }
}
