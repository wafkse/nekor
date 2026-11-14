//! Typed configuration inheritance with persistent type requirements and source locations.
//!
//! Local layers contain checked initializers and incoming field policies. Applying
//! inheritance consumes that policy-bearing layer and produces an effective tree
//! whose values must satisfy both inherited and local contracts.
#![no_std]

extern crate alloc;

pub mod layer;
pub mod policy;
pub mod tree;
