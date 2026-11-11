//! Nominal values identified by the declarations that give their magnitudes meaning.
//!
//! Resource indices and counts use nonnegative mathematical integers. References
//! carry decoded strings admitted by their declaration's member set. Additional
//! source predicates on resource domains are checked by the evaluation layer.

pub mod count;
pub mod index;
pub mod reference;
