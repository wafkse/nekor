//! Mutable and immutable views of the same kind of bit field.

/// Associate a bit-field view with its mutable and immutable forms.
pub trait Counterpart {
    /// The mutable view, or `Self` when the view is already mutable.
    type Mut;

    /// The immutable view, or `Self` when the view is already immutable.
    type Immut;
}
