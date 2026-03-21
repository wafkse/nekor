//! Mutable and immutable counterparts to existing field types.

/// A marker trait to indicate the *mutable or immutable* alternative of either
/// an immutable or mutable wrapper type.
pub trait Counterpart {
    /// The mutable counterpart to this wrapper.
    ///
    /// In the case of this wrapper being already mutable, this is
    /// self-referential.
    type Mut;

    /// The immutable counterpart to this wrapper.
    ///
    /// In the case of this wrapper being already immutable, this is
    /// self-referential.
    type Immut;
}
