//! Source-declared reference membership is checked without an external resource registry.

use alloc::sync::Arc;

use aldebaran::{
    prelude::Report,
    report::annotated::{InlineAnnotations, label::Label},
};
use fack::prelude::Error;

use crate::{domain, source::Origin};

/// A decoded reference string admitted by its nominal namespace.
#[derive(Debug, Clone, PartialEq, Eq)]
// NOTE(invariant): The namespace's member set, when present, contains `name`.
pub struct Reference {
    /// Source declaration defining the nominal reference namespace.
    domain: Arc<domain::Reference>,

    /// Exact decoded member name.
    name: Arc<str>,
}

impl Reference {
    /// Admit a decoded member, distinguishing absent membership from an empty set.
    #[inline]
    pub fn new(domain: Arc<domain::Reference>, name: impl Into<Arc<str>>, origin: Origin) -> Result<Self, Membership> {
        let name = name.into();

        if domain.contains(&name) {
            Ok(Self { domain, name })
        } else {
            let labels = InlineAnnotations::new(
                Label::new(
                    "this string is not a member of the declared reference namespace",
                    origin.span(),
                ),
                [],
            );

            Err(Membership {
                origin,
                domain,
                name,
                labels,
            })
        }
    }

    /// Borrow the nominal namespace, including its independently qualified declaration.
    #[inline]
    pub const fn domain(&self) -> &Arc<domain::Reference> {
        let Self { domain, .. } = self;

        domain
    }

    /// Borrow the exact admitted reference name.
    #[inline]
    pub fn name(&self) -> &str {
        let Self { name, .. } = self;

        name
    }
}

/// A decoded string is absent from its source-declared exhaustive member set.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("reference is not a declared namespace member")]
#[report(title = "unknown reference member", annotations = labels)]
// NOTE(invariant): The labels belong to `origin`, independently of the namespace's source.
pub struct Membership {
    /// Source-qualified occurrence of the rejected reference string.
    origin: Origin,

    /// Declaration containing the exhaustive member set.
    domain: Arc<domain::Reference>,

    /// Rejected decoded reference name.
    name: Arc<str>,

    /// Primary reference location.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

impl Membership {
    /// Retrieve the canonical source and coordinates of the rejected reference.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        let Self { origin, .. } = self;

        origin
    }

    /// Borrow the source-qualified declaration establishing membership.
    #[inline]
    pub const fn domain(&self) -> &Arc<domain::Reference> {
        let Self { domain, .. } = self;

        domain
    }

    /// Borrow the rejected name independently of the diagnostic message.
    #[inline]
    pub fn name(&self) -> &str {
        let Self { name, .. } = self;

        name
    }
}

#[cfg(test)]
mod tests {
    use alloc::{collections::BTreeSet, sync::Arc};

    use aldebaran::prelude::Span;

    use super::Reference;
    use crate::{
        domain,
        source::{Id, Origin},
    };

    #[test]
    fn reference_values_preserve_decoded_name_membership_and_namespace() {
        let declaration = Origin::new(Id::new("namespace"), Span::unit(0));
        let origin = Origin::new(Id::new("value"), Span::unit(4));
        let members = BTreeSet::from([Arc::from("uart")]);
        let domain = Arc::new(domain::Reference::new(declaration.clone(), Some(members)));
        let value = Reference::new(Arc::clone(&domain), "uart", origin.clone()).expect("member");
        let error = Reference::new(domain, "UART", origin.clone()).expect_err("case sensitive");
        let empty = Arc::new(domain::Reference::new(declaration.clone(), Some(BTreeSet::new())));
        let unrestricted = Arc::new(domain::Reference::new(declaration.clone(), None));

        assert_eq!(value.name(), "uart");
        assert_eq!(value.domain().origin(), &declaration);
        assert_eq!(error.origin(), &origin);
        assert_eq!(error.domain().origin(), &declaration);
        assert_eq!(error.name(), "UART");
        assert!(Reference::new(empty, "uart", origin.clone()).is_err());
        assert!(Reference::new(unrestricted, "anything", origin).is_ok());
    }
}
