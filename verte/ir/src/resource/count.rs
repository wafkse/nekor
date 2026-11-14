//! Integral nominal counts can become quantities without losing their declaration identity.

use alloc::sync::Arc;

use num_bigint::BigUint;

use crate::{
    dimension::{Dimension, Tag},
    domain,
    number::Number,
    quantity::Quantity,
};

/// A nonnegative integral magnitude in a source-declared count domain.
#[derive(Debug, Clone, PartialEq, Eq)]
// NOTE(invariant): None.
pub struct Count {
    /// Source declaration identifying the count kind.
    domain: Arc<domain::Count>,

    /// Exact number of nominal items.
    magnitude: BigUint,
}

impl Count {
    /// Associate an intrinsically nonnegative integral magnitude with its domain.
    #[inline]
    pub const fn new(domain: Arc<domain::Count>, magnitude: BigUint) -> Self {
        Self { domain, magnitude }
    }

    /// Borrow the source-declared nominal kind.
    #[inline]
    pub const fn domain(&self) -> &Arc<domain::Count> {
        let Self { domain, .. } = self;

        domain
    }

    /// Borrow the exact number of nominal items.
    #[inline]
    pub const fn magnitude(&self) -> &BigUint {
        let Self { magnitude, .. } = self;

        magnitude
    }

    /// Express the count as a quantity identified by the same nominal tag.
    #[inline]
    pub fn quantity(&self) -> Quantity {
        let Self { domain, magnitude } = self;

        let dimension = Dimension::tag(Tag::Count(domain.origin().clone()));
        let magnitude = Number::integer(magnitude.clone());

        Quantity::linear(magnitude, dimension)
    }
}

#[cfg(test)]
mod tests {
    use alloc::sync::Arc;

    use aldebaran::prelude::Span;
    use num_bigint::BigUint;

    use super::Count;
    use crate::{
        domain,
        number::Number,
        source::{Id, Origin},
        unit::Unit,
    };

    #[test]
    fn nominal_count_quantities_match_suffixes_only_for_the_same_declaration() {
        let origin = Origin::new(Id::new("count"), Span::unit(0));
        let domain = Arc::new(domain::Count::new(origin.clone(), "page"));
        let magnitude = BigUint::from(1_u8) << 256_usize;
        let count = Count::new(Arc::clone(&domain), magnitude.clone());
        let unit = Unit::lookup("page").expect("intrinsic count suffix");
        let quantity = unit
            .apply(&Number::integer(magnitude), Some(&domain), origin.clone())
            .expect("nominal quantity");
        let other = Count::new(
            Arc::new(domain::Count::new(Origin::new(Id::new("other"), Span::unit(0)), "page")),
            1_u8.into(),
        );

        assert_eq!(count.quantity(), quantity);
        assert_ne!(count.quantity().signature(), other.quantity().signature());
        assert_eq!(count.domain().origin(), &origin);
    }
}
