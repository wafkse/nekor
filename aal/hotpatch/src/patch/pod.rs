//! Plain-old-data modeling.

/// A trait that describes a `Plain Old Data` type.
///
/// In other words, this is a marker trait for a type that is:
///
/// - *Trivially copyable*.
/// - *Can be shared between threads* (i.e., a [`Sync`]-implementor).
/// - *Non-lifetime-dependant* (i.e., it must outlive the `'static` lifetime).
///
/// This trait is blanket implemented for all elegible types.
#[diagnostic::on_unimplemented(
    message = "{Self} cannot serve as a `Delegator` and/or plain dependency"
)]
pub trait Pod: Copy + Sync + 'static {}

/// A blanket implementation of [`Pod`] for all elegible types.
impl<D> Pod for D where D: Copy + Sync + 'static {}
