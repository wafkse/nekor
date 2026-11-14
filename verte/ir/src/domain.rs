//! Nominal resource declarations independent of host hardware metadata.

use alloc::{collections::BTreeSet, sync::Arc};

use num_bigint::BigUint;

use crate::source::Origin;

/// A source-declared byte-address width in the language-defined range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
// NOTE(invariant): The width is in 1..=128.
pub struct Width(u8);

impl Width {
    /// Admit widths supported by the `addr` domain category.
    #[inline]
    pub const fn new(bits: u8) -> Option<Self> {
        match bits {
            1..=128 => Some(Self(bits)),
            _ => None,
        }
    }

    /// Retrieve the coordinate width in bits.
    #[inline]
    pub const fn bits(self) -> u8 {
        let Self(bits) = self;

        bits
    }
}

/// A flat byte-addressed nominal space with source-declared width.
#[derive(Debug, Clone, PartialEq, Eq)]
// NOTE(invariant): None.
pub struct Address {
    /// Canonical declaration identifying the nominal space.
    origin: Origin,

    /// Source-declared coordinate width.
    width: Width,
}

impl Address {
    /// Establish a nominal space without consulting compilation-host address widths.
    #[inline]
    pub const fn new(origin: Origin, width: Width) -> Self {
        Self { origin, width }
    }

    /// Retrieve the canonical domain declaration.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        let Self { origin, .. } = self;

        origin
    }

    /// Compute the mathematical exclusive boundary, which is not a domain coordinate.
    #[inline]
    pub fn boundary(&self) -> BigUint {
        BigUint::from(1_u8) << usize::from(self.width().bits())
    }

    /// Retrieve the declared width independently of host pointer size.
    #[inline]
    pub const fn width(&self) -> Width {
        let &Self { width, .. } = self;

        width
    }
}

/// A nominal discrete count identified by its source declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
// NOTE(invariant): None.
pub struct Count {
    /// Canonical declaration identity.
    origin: Origin,

    /// Source-declared name used by nominal unit suffix lookup.
    name: Arc<str>,
}

impl Count {
    /// Establish a distinct count domain at its canonical declaration.
    #[inline]
    pub fn new(origin: Origin, name: impl Into<Arc<str>>) -> Self {
        let name = name.into();

        Self { origin, name }
    }

    /// Retrieve the declaration used to distinguish this count from other counts.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        let Self { origin, .. } = self;

        origin
    }

    /// Borrow the source-declared domain name.
    #[inline]
    pub fn name(&self) -> &str {
        let Self { name, .. } = self;

        name
    }
}

/// The intrinsic category of a nominal integral resource domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexKind {
    /// Interrupt identifiers.
    Interrupt,

    /// General-purpose input/output identifiers.
    Gpio,
}

/// A nominal nonnegative integral resource namespace.
#[derive(Debug, Clone, PartialEq, Eq)]
// NOTE(invariant): None.
pub struct Index {
    /// Canonical declaration of the resource namespace.
    origin: Origin,

    /// Intrinsic category, distinct from its declaration identity.
    kind: IndexKind,
}

impl Index {
    /// Establish an integral resource namespace.
    #[inline]
    pub const fn new(origin: Origin, kind: IndexKind) -> Self {
        Self { origin, kind }
    }

    /// Retrieve the canonical resource declaration.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        let Self { origin, .. } = self;

        origin
    }

    /// Retrieve the intrinsic resource category.
    #[inline]
    pub const fn kind(&self) -> IndexKind {
        let &Self { kind, .. } = self;

        kind
    }
}

/// A nominal string reference namespace with an optional exhaustive member set.
#[derive(Debug, Clone, PartialEq, Eq)]
// NOTE(invariant): None.
pub struct Reference {
    /// Canonical namespace declaration.
    origin: Origin,

    /// Exhaustive source-defined membership, when supplied.
    members: Option<BTreeSet<Arc<str>>>,
}

impl Reference {
    /// Establish membership without consulting an external resource database.
    #[inline]
    pub const fn new(origin: Origin, members: Option<BTreeSet<Arc<str>>>) -> Self {
        Self { origin, members }
    }

    /// Retrieve the canonical namespace declaration.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        let Self { origin, .. } = self;

        origin
    }

    /// Test source-defined membership, distinguishing an absent set from an empty one.
    #[inline]
    pub fn contains(&self, name: &str) -> bool {
        let Self { members, .. } = self;

        match members {
            None => true,
            Some(members) => members.contains(name),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::collections::BTreeSet;

    use aldebaran::prelude::Span;

    use super::{Address, Count, Reference, Width};
    use crate::source::{Id, Origin};

    #[test]
    fn identity_comes_from_the_declaration_and_width_is_checked() {
        let first = Origin::new(Id::new("host:first"), Span::unit(0));
        let second = Origin::new(Id::new("host:second"), Span::unit(0));
        let width = Width::new(64).expect("language address width");

        assert!(Width::new(0).is_none());
        assert!(Width::new(129).is_none());
        assert!(Width::new(128).is_some());
        assert_ne!(Address::new(first.clone(), width), Address::new(second.clone(), width));
        assert_ne!(Count::new(first, "page"), Count::new(second, "page"));
    }

    #[test]
    fn an_empty_membership_set_is_not_an_unrestricted_namespace() {
        let origin = Origin::new(Id::new("clocks"), Span::unit(0));
        let unrestricted = Reference::new(origin.clone(), None);
        let empty = Reference::new(origin, Some(BTreeSet::new()));

        assert!(unrestricted.contains("uart"));
        assert!(!empty.contains("uart"));
    }
}
