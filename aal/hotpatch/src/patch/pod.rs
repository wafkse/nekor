//! Plain-old-data modeling.

/// Bounds for delegate markers and their static selection dependencies.
///
/// Values can be copied and shared between threads, and cannot borrow temporary data.
/// These bounds say nothing about padding, foreign calling conventions or whether every
/// bit pattern is a valid value.
#[diagnostic::on_unimplemented(message = "{Self} cannot serve as a `Delegator` and/or plain dependency")]
pub trait Pod: Copy + Sync + 'static {}

/// Any type satisfying these bounds can serve as a dependency.
impl<D> Pod for D where D: Copy + Sync + 'static {}
