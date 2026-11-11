//! Interrupt and GPIO identifiers preserve their declared namespaces without host-width limits.

use alloc::sync::Arc;

use num_bigint::BigUint;

use crate::domain;

/// A nonnegative integral resource identifier in a nominal namespace.
#[derive(Debug, Clone, PartialEq, Eq)]
// NOTE(invariant): None.
pub struct Index {
    /// Source declaration carrying namespace identity and resource category.
    domain: Arc<domain::Index>,

    /// Exact resource identifier, independent of host register widths.
    coordinate: BigUint,
}

impl Index {
    /// Establish an integral resource value before checking its source validity predicate.
    #[inline]
    pub const fn new(domain: Arc<domain::Index>, coordinate: BigUint) -> Self {
        Self { domain, coordinate }
    }

    /// Borrow the namespace declaration and intrinsic resource category.
    #[inline]
    pub const fn domain(&self) -> &Arc<domain::Index> {
        let Self { domain, .. } = self;

        domain
    }

    /// Borrow the exact resource coordinate.
    #[inline]
    pub const fn coordinate(&self) -> &BigUint {
        let Self { coordinate, .. } = self;

        coordinate
    }
}

#[cfg(test)]
mod tests {
    use alloc::sync::Arc;

    use aldebaran::prelude::Span;
    use num_bigint::BigUint;

    use super::Index;
    use crate::{
        domain::{self, IndexKind},
        source::{Id, Origin},
    };

    #[test]
    fn identical_coordinates_do_not_erase_resource_namespace_identity() {
        let origin = Origin::new(Id::new("resource"), Span::unit(0));
        let first = Arc::new(domain::Index::new(origin.clone(), IndexKind::Interrupt));
        let second = Arc::new(domain::Index::new(
            Origin::new(Id::new("other"), Span::unit(0)),
            IndexKind::Interrupt,
        ));
        let coordinate = BigUint::from(1_u8) << 256_usize;
        let value = Index::new(first, coordinate.clone());
        let other = Index::new(second, coordinate.clone());

        assert_ne!(value, other);
        assert_eq!(value.coordinate(), &coordinate);
        assert_eq!(value.domain().origin(), &origin);
    }
}
