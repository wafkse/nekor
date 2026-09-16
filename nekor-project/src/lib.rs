#![no_std]

//! Pin projection traits and derive support.
//!
//! This crate provides the [`Project`] derive macro and the traits implemented
//! by its generated code. A projection is a borrowing view into a pinned value.
//! It exposes each source field without moving the source or changing an active
//! enum variant.
//!
//! The crate is `no_std`. Generated code refers only to `core` and requires no
//! allocation.
//!
//! Runnable examples in `examples/` show stack-pinned named fields, enum
//! variants, and pin-aware destruction without heap allocation. Run them with
//! `cargo run -p nekor-project --example stack_projection`,
//! `cargo run -p nekor-project --example enum_projection`, or
//! `cargo run -p nekor-project --example pinned_drop`.
//!
//! # Basic use
//!
//! ```rust
//! use core::{marker::PhantomPinned, pin::Pin};
//!
//! use nekor_project::Project;
//!
//! #[derive(Project)]
//! struct Task {
//!     #[project(pin)]
//!     state: PhantomPinned,
//!     priority: usize,
//! }
//!
//! let mut task = Box::pin(Task {
//!     state: PhantomPinned,
//!     priority: 4,
//! });
//!
//! let projection = task.as_ref().project();
//! let _: Pin<&PhantomPinned> = projection.state;
//! assert_eq!(*projection.priority, 4);
//!
//! let projection_mut = task.as_mut().project_mut();
//! let _: Pin<&mut PhantomPinned> = projection_mut.state;
//! *projection_mut.priority = 7;
//! ```
//!
//! # Generated interface
//!
//! Deriving [`Project`] for a type named `Task` generates two borrowing view
//! types named `TaskProjection` and `TaskProjectionMut`. It also generates an
//! unsafe implementation of [`Project`] and the safety enforcement required by
//! the selected helper attributes.
//!
//! `TaskProjection` contains immutable field projections. A pinned field has
//! type `Pin<&T>`. An ordinary field has type `&T`.
//!
//! `TaskProjectionMut` contains mutable field projections. A pinned field has
//! type `Pin<&mut T>`. An ordinary field has type `&mut T`.
//!
//! Projection types preserve the visibility and shape of the source type.
//! Named structs produce named projection structs. Tuple structs produce tuple
//! projection structs. Unit structs produce unit projection structs. Enum
//! projections preserve every variant name and field shape. Explicit enum
//! discriminants are not copied because a projection is a borrowing view and
//! not an alternate representation of the source enum.
//!
//! Source lifetimes, type parameters, const parameters, bounds, defaults, and
//! where clauses are preserved. The macro selects an internal projection
//! lifetime that does not collide with a source lifetime.
//!
//! # Helper attribute syntax
//!
//! Each helper attribute contains exactly one option. Type-level unsafe clauses
//! may be repeated when a type needs more than one opt-out.
//!
//! ```rust
//! use core::marker::PhantomPinned;
//!
//! use nekor_project::Project;
//!
//! #[derive(Project)]
//! #[project(unsafe = Drop)]
//! #[project(unsafe = Deref)]
//! struct Wrapper {
//!     #[project(pin)]
//!     value: PhantomPinned,
//! }
//! ```
//!
//! `#[project(pin)]` is valid only on fields. Every `unsafe = Trait` clause is
//! valid only on a struct or enum. Helper attributes on enum variants are
//! rejected. Duplicate options are rejected rather than silently ignored.
//!
//! # `#[project(pin)]`
//!
//! This field option declares structural pinning. Once the containing value is
//! pinned, the field must remain at the same address until its destructor
//! finishes unless the field type implements [`Unpin`].
//!
//! The immutable projection wraps the field reference in [`Pin`]. The mutable
//! projection wraps the mutable field reference in [`Pin`]. The wrapper lets
//! downstream code use pin-aware APIs without gaining an ordinary mutable
//! reference that could move the field.
//!
//! A field without this option is not structurally pinned. Its projections are
//! ordinary references. Such a field may be replaced or moved through the
//! mutable projection even when its type does not implement [`Unpin`]. This is
//! intentional. The field-level option defines which fields participate in the
//! structural pinning contract.
//!
//! The generated [`Unpin`] implementation depends only on fields marked with
//! this option. Unmarked fields never make the containing type `!Unpin` through
//! the generated implementation.
//!
//! # `#[project(unsafe = Unpin)]`
//!
//! This type option disables the generated structural [`Unpin`] implementation.
//! Without the option, the macro implements [`Unpin`] exactly when every pinned
//! field implements [`Unpin`]. It uses a dependent helper type so concrete
//! pinned fields such as [`core::marker::PhantomPinned`] are handled without
//! requiring unstable trivial bounds.
//!
//! With the option, normal auto-trait behavior and any user-written [`Unpin`]
//! implementation determine whether the source type can move. The macro no
//! longer proves that result from the pinned field set.
//!
//! Selecting this option asserts that every possible move permitted by the
//! resulting [`Unpin`] behavior preserves the validity of all pinned fields.
//! A user-written implementation must include every bound needed for that
//! statement. Implementing [`Unpin`] uses safe Rust syntax, so the helper
//! attribute is the explicit marker that the author accepted this unsafe proof
//! obligation.
//!
//! ```rust
//! use nekor_project::Project;
//!
//! #[derive(Project)]
//! #[project(unsafe = Unpin)]
//! struct Wrapper<T> {
//!     #[project(pin)]
//!     value: T,
//! }
//!
//! impl<T: Unpin> Unpin for Wrapper<T> {}
//! ```
//!
//! # `#[project(unsafe = Drop)]`
//!
//! This type option disables the generated coherence blocker for [`Drop`].
//! Without the option, a type with at least one pinned field cannot also
//! implement [`Drop`]. The restriction prevents destructor code from obtaining
//! `&mut Self` and moving a pinned field before normal field destruction.
//!
//! Selecting this option asserts that the destructor never moves a pinned
//! field, never replaces an active enum variant, and keeps the source at a
//! stable address until normal field destruction completes.
//!
//! Use [`PinnedDrop`] when destructor logic needs projected access. The
//! ordinary [`Drop::drop`] implementation must create a pin for the same value
//! and call [`PinnedDrop::drop`] exactly once. It must not use the value after
//! forwarding in a way that can move a pinned field.
//!
//! ```rust
//! use core::{marker::PhantomPinned, pin::Pin};
//!
//! use nekor_project::{PinnedDrop, Project};
//!
//! #[derive(Project)]
//! #[project(unsafe = Drop)]
//! struct Resource {
//!     #[project(pin)]
//!     state: PhantomPinned,
//!     closed: bool,
//! }
//!
//! unsafe impl PinnedDrop for Resource {
//!     unsafe fn drop(self: Pin<&mut Self>) {
//!         let ResourceProjectionMut { state, closed } = self.project_mut();
//!         let _: Pin<&mut PhantomPinned> = state;
//!         *closed = true;
//!     }
//! }
//!
//! impl Drop for Resource {
//!     fn drop(&mut self) {
//!         // SAFETY
//!         //
//!         // Destructor execution keeps this value in place. This
//!         // implementation forwards exactly once for the same value.
//!         let pinned = unsafe { Pin::new_unchecked(self) };
//!         unsafe { <Self as PinnedDrop>::drop(pinned) };
//!     }
//! }
//! ```
//!
//! # `#[project(unsafe = Deref)]`
//!
//! This type option disables the generated coherence blocker for
//! [`core::ops::Deref`]. Without the option, a type with pinned fields cannot
//! implement that trait.
//!
//! Selecting this option asserts that shared dereferencing does not expose a
//! path that can move pinned data. This includes paths through interior
//! mutability. A dereference target must not let safe code obtain movable
//! access to a pinned field or replace storage that contains one.
//!
//! The option affects only the [`core::ops::Deref`] blocker. It does not
//! disable the [`core::ops::DerefMut`] or [`Drop`] blockers. Each additional
//! trait needs its own explicit unsafe clause.
//!
//! # `#[project(unsafe = DerefMut)]`
//!
//! This type option disables the generated coherence blocker for
//! [`core::ops::DerefMut`]. Without the option, a type with pinned fields
//! cannot implement that trait.
//!
//! Selecting this option asserts that mutable dereferencing cannot expose an
//! ordinary mutable reference from which a pinned field can be moved. The
//! dereference target may expose unpinned state, but any pinned field must keep
//! its address and remain inaccessible to operations such as replacement or
//! swapping.
//!
//! This clause does not imply the [`core::ops::Deref`] clause. A type that
//! implements both traits must state both clauses.
//!
//! # Generated safety enforcement
//!
//! For a type with pinned fields, the derive normally generates coherence
//! blockers for [`Drop`], [`core::ops::Deref`], and [`core::ops::DerefMut`]. A
//! matching unsafe clause removes only its corresponding blocker.
//!
//! The generated [`Unpin`] implementation is separate. It is conditional on
//! every pinned field implementing [`Unpin`]. The `unsafe = Unpin` clause omits
//! that implementation entirely.
//!
//! Types without pinned fields receive an unconditional [`Unpin`]
//! implementation. They do not need trait blockers because every projected
//! field is explicitly movable through an ordinary mutable reference.
//!
//! # Unsupported inputs
//!
//! Unions cannot be projected because they do not provide a safely known active
//! field.
//!
//! `repr(packed)` and `repr(packed(N))` are rejected because creating a
//! reference to an unaligned packed field is invalid. Other representation
//! attributes are not copied to projection types.
//!
//! Malformed helper attributes, unknown unsafe targets, invalid placement, and
//! duplicate clauses produce compile errors. The macro never converts a helper
//! parsing failure into default behavior.
//!
//! ```compile_fail
//! use core::marker::PhantomPinned;
//! use nekor_project::Project;
//!
//! #[derive(Project)]
//! struct InvalidDrop {
//!     #[project(pin)]
//!     state: PhantomPinned,
//! }
//!
//! impl Drop for InvalidDrop {
//!     fn drop(&mut self) {}
//! }
//! ```
//!
//! ```compile_fail
//! use nekor_project::Project;
//!
//! #[derive(Project)]
//! #[repr(packed)]
//! struct Packed {
//!     value: u32,
//! }
//! ```

extern crate self as nekor_project;

use core::pin::Pin;

#[doc(inline)]
pub use nekor_project_derive::Project;

/// A trait that describes immutable and mutable [`Pin`] projections.
///
/// The derive macro implements this trait for structs and enums. Manual
/// implementations are possible, but they must uphold the complete safety
/// contract below.
///
/// # Safety
///
/// Implementors must ensure that projected references borrow fields from the
/// pinned source value and do not outlive that source borrow. A field projected
/// as [`Pin`] must remain at a stable address unless its type implements
/// [`Unpin`]. Mutable projections must not alias each other or the source
/// value. An enum implementation must not replace or mutate the active
/// discriminant while projected field references exist.
pub unsafe trait Project {
    /// The immutable borrowing view of `Self`.
    ///
    /// Implementations normally use a generated projection structure or enum.
    /// Pinned fields use `Pin<&T>` and ordinary fields use `&T`.
    type Projection<'a>
    where
        Self: 'a;

    /// The mutable borrowing view of `Self`.
    ///
    /// Implementations normally use a generated projection structure or enum.
    /// Pinned fields use `Pin<&mut T>` and ordinary fields use `&mut T`.
    type ProjectionMut<'a>
    where
        Self: 'a;

    /// Borrow the fields of an immutably pinned value.
    ///
    /// The returned projection cannot outlive the source borrow. Moving the
    /// projection value does not move any source field.
    fn project(self: Pin<&Self>) -> Self::Projection<'_>;

    /// Borrow the fields of a mutably pinned value.
    ///
    /// The returned projection contains one disjoint mutable borrow for every
    /// source field. Dropping the projection releases all of those borrows.
    fn project_mut(self: Pin<&mut Self>) -> Self::ProjectionMut<'_>;
}

/// A pin-aware destructor hook for [`Project`] types.
///
/// This trait lets destructor logic access pinned fields without first creating
/// an ordinary mutable projection. It is intended for types that derive
/// [`Project`] with `#[project(unsafe = Drop)]`.
///
/// # Safety
///
/// The type must implement [`Drop`]. Its [`Drop::drop`] implementation must pin
/// the same value without moving it and forward exactly once to
/// [`PinnedDrop::drop`]. It must not access the value after forwarding in a way
/// that moves a pinned field or replaces an active enum variant.
pub unsafe trait PinnedDrop: Project {
    /// Run user-defined destructor logic through a pinned receiver.
    ///
    /// # Safety
    ///
    /// The caller must invoke this hook exactly once from [`Drop::drop`] for
    /// the same value. The value must remain pinned until this hook returns
    /// and normal field destruction completes.
    unsafe fn drop(self: Pin<&mut Self>);
}
