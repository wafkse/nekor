#![cfg_attr(not(any(test, miri)), no_std)]

//! Parsing, validation, and expansion infrastructure for `nekor-project`.
//!
//! This crate is the implementation component behind
//! `nekor-project-derive`. Application code should use the `nekor-project`
//! facade. The public items here support the procedural macro boundary and do
//! not replace the facade traits.
//!
//! # Processing model
//!
//! Expansion has three stages.
//!
//! 1. [`attribute`] parses helper attributes without discarding syntax errors
//! 2. [`field`] converts source fields into structural pinning metadata
//! 3. [`item`] validates the complete type and emits projection code
//!
//! Validation happens before token expansion. Malformed helper attributes,
//! duplicate unsafe clauses, invalid placement, unions, and packed
//! representations therefore cannot fall back to less restrictive defaults.
//!
//! # Unsafe option model
//!
//! A `project(unsafe = Trait)` clause is represented by
//! [`attribute::UnsafeClauseTarget`]. Expansion treats each target as an
//! independent opt-out from one generated enforcement rule.
//!
//! `Unpin` omits structural `Unpin` generation. `Drop`, `Deref`, and `DerefMut`
//! each omit one coherence blocker. No clause changes projected field types.
//! Only `project(pin)` determines whether a field projects through `Pin`.
//!
//! # Allocation model
//!
//! The implementation uses `alloc` while constructing syntax trees and token
//! streams on the host. Generated target code uses only `core` and does not
//! allocate.

extern crate alloc;

/// Helper attribute syntax and unsafe clause targets.
pub mod attribute;

/// Source field shapes and structural pinning metadata.
pub mod field;

/// Complete item validation and token expansion.
pub mod item;

pub use item::ProjectItem;
