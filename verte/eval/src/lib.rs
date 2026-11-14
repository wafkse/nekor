//! Declared-type admission and replacement checks for Verte values.
//!
//! Checked values own their requirements and provenance independently of source
//! providers and syntax trees. Representation changes and later replacements
//! must satisfy the same type requirements.
#![no_std]

extern crate alloc;

pub mod contract;
