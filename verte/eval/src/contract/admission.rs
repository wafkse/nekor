//! Type-admission failures with separate value and declaration locations.

use alloc::{boxed::Box, format, string::String, sync::Arc};
use core::fmt;

use aldebaran::{
    prelude::{Print, Report},
    report::annotated::{InlineAnnotations, label::Label},
};
use fack::prelude::Error;
use num_bigint::BigUint;
use verte_ir::{address, float, integer, resource::reference, source::Origin, types::Type};

use super::Contract;

/// A value failed a declared structural requirement.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[report(forward)]
pub enum Admission {
    /// The value and requirement share a canonical source.
    #[error(transparent(0))]
    Local(Box<Rejection<1>>),

    /// The requirement belongs to a separately rendered source.
    #[error(transparent(0))]
    Imported(Box<Rejection<0>>),
}

impl Admission {
    /// Build a report for the failure, grouping labels by source identity.
    #[inline]
    pub fn new(required: &Contract, origin: Origin, cause: Failure) -> Self {
        let message = Explanation(format!("{cause}"));
        let declaration = Declaration::new(required);

        if origin.source() == required.origin().source() {
            let labels = InlineAnnotations::new(
                Label::new(message, origin.span()),
                [Label::new(
                    Explanation(String::from("the type requirement is declared here")),
                    required.origin().span(),
                )],
            );

            Self::Local(Box::new(Rejection {
                origin,
                declaration,
                cause,
                labels,
            }))
        } else {
            let labels = InlineAnnotations::new(Label::new(message, origin.span()), []);

            Self::Imported(Box::new(Rejection {
                origin,
                declaration,
                cause,
                labels,
            }))
        }
    }

    /// Retrieve the canonical source and coordinates of the rejected value or member.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        match self {
            Self::Local(report) => report.origin(),
            Self::Imported(report) => report.origin(),
        }
    }

    /// Inspect the independently source-qualified type requirement.
    #[inline]
    pub const fn declaration(&self) -> &Declaration {
        match self {
            Self::Local(report) => report.declaration(),
            Self::Imported(report) => report.declaration(),
        }
    }

    /// Inspect the concrete cause independently of rendered diagnostic wording.
    #[inline]
    pub const fn failure(&self) -> &Failure {
        match self {
            Self::Local(report) => report.failure(),
            Self::Imported(report) => report.failure(),
        }
    }
}

/// An admission failure annotated in the candidate's source.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("value does not satisfy its declared type")]
#[error(source(cause))]
#[report(title = "value does not satisfy its declared type", annotations = labels)]
// NOTE(invariant): All labels belong to `origin`'s canonical source, including any declaration
// label.
pub struct Rejection<const N: usize> {
    /// Source-qualified candidate or conflicting member occurrence.
    origin: Origin,

    /// Type requirement with its own canonical source coordinates.
    declaration: Declaration,

    /// Specific conversion or structural requirement that failed.
    cause: Failure,

    /// Candidate location and any declaration context in the same source.
    labels: InlineAnnotations<Label<Explanation>, N>,
}

impl<const N: usize> Rejection<N> {
    /// Retrieve the source qualifying this report's labels.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        let Self { origin, .. } = self;

        origin
    }

    /// Borrow the requirement report for rendering against its own source text.
    #[inline]
    pub const fn declaration(&self) -> &Declaration {
        let Self { declaration, .. } = self;

        declaration
    }

    /// Inspect the failed requirement before rendering.
    #[inline]
    pub const fn failure(&self) -> &Failure {
        let Self { cause, .. } = self;

        cause
    }
}

/// The source declaration responsible for a rejected value's type requirement.
#[derive(Debug, Clone, Report)]
#[report(title = "required type declaration", annotations = labels)]
// NOTE(invariant): The label uses the source coordinates qualified by `origin`.
pub struct Declaration {
    /// Canonical source and original type-expression coordinates.
    origin: Origin,

    /// Required type, including its structural and nominal constraints.
    ty: Type,

    /// Type declaration location in its own source.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

impl Declaration {
    /// Create a declaration report for attachment to its source text.
    #[inline]
    fn new(required: &Contract) -> Self {
        let origin = required.origin().clone();
        let ty = required.ty();
        let labels = InlineAnnotations::new(Label::new("the type requirement is declared here", origin.span()), []);

        Self { origin, ty, labels }
    }

    /// Retrieve the canonical source qualifying this declaration report.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        let Self { origin, .. } = self;

        origin
    }

    /// Inspect the structural type that rejected the candidate.
    #[inline]
    pub const fn ty(&self) -> &Type {
        let Self { ty, .. } = self;

        ty
    }
}

/// An owned admission explanation printable through Aldebaran's rendering contract.
#[derive(Debug, Clone)]
// NOTE(invariant): None.
pub struct Explanation(String);

impl Print for Explanation {
    type Context = ();

    #[inline]
    fn print_with_context<W>(&self, writer: &mut W, context: &Self::Context) -> fmt::Result
    where
        W: fmt::Write,
    {
        let Self(message) = self;

        message.as_str().print_with_context(writer, context)
    }
}

/// Structural and representation failures possible during declared-type admission.
#[derive(Debug, Clone, Error)]
#[error(inline)]
pub enum Failure {
    /// The represented value has an incompatible type, precision, or nominal domain.
    #[error("this value has an incompatible type, representation, or nominal domain")]
    Type,

    /// Existing requirements have incompatible underlying types.
    #[error("this earlier type requirement is incompatible with the additional requirement")]
    Conflict,

    /// A fixed array has a different mathematical length.
    #[error("expected {expected} array elements, found {actual}")]
    Length {
        /// Exact source-defined required length.
        expected: BigUint,

        /// Number of evaluated elements.
        actual: usize,
    },

    /// A closed record omits a required decoded member.
    #[error("missing required record member `{0}`")]
    Missing(Arc<str>),

    /// A closed record contains an undeclared decoded member.
    #[error("record member `{0}` is not declared by the required closed type")]
    Unexpected(Arc<str>),

    /// A string is outside the declared enumeration.
    #[error("this string is not a member of the declared enumeration")]
    Enumeration,

    /// A nominal integral resource or count received a negative or fractional magnitude.
    #[error("this nominal resource requires a nonnegative integral magnitude")]
    Natural,

    /// An exact magnitude is outside an integer or mask format.
    #[error(transparent(0))]
    Integer(integer::Representability),

    /// IEEE conversion cannot produce a finite value.
    #[error(transparent(0))]
    Float(float::Failure),

    /// A coordinate is outside the declared address representation.
    #[error(transparent(0))]
    Address(address::Admission),

    /// A reference string is outside its exhaustive namespace.
    #[error(transparent(0))]
    Reference(reference::Membership),
}

#[cfg(test)]
mod tests {
    use alloc::{string::String, sync::Arc, vec};

    use aldebaran::{
        prelude::{Report as _, Span},
        report::{
            annotated::Annotations as _,
            render::{
                RenderMut as _,
                textual::{
                    Present, Textual,
                    offload::fancy::{Fancy, FancySettings},
                },
            },
        },
    };
    use verte_ir::{
        source::{Id, Origin},
        types::{Array, Type},
        value::{Data, Value},
    };

    use super::Admission;
    use crate::contract::Contract;

    #[test]
    fn local_admission_reports_keep_value_and_type_labels() {
        let declaration = Origin::new(Id::new("configuration"), Span::unit(0));
        let origin = Origin::new(Id::new("configuration"), Span::unit(7));
        let required = Arc::new(Contract::new(declaration, Type::String));
        let error = required
            .admit(Value::new(origin, Data::Bool(false)))
            .expect_err("wrong type");

        assert!(matches!(error, Admission::Local(_)));
        assert_eq!(error.annotations().list().expect("locations").len().get(), 2);
    }

    #[test]
    fn imported_requirements_render_against_their_own_source_and_keep_concrete_counts() {
        let declaration = Origin::new(Id::new("definitions"), Span::unit(0));
        let origin = Origin::new(Id::new("configuration"), Span::unit(0));
        let required = Arc::new(Contract::new(
            declaration.clone(),
            Type::Array(Array::new(Type::Bool, Some(3_u8.into()))),
        ));
        let values = vec![
            Value::new(origin.clone(), Data::Bool(true)),
            Value::new(origin.clone(), Data::Bool(false)),
        ];
        let error = required
            .admit(Value::new(origin.clone(), Data::Array(values)))
            .expect_err("fixed length");
        let attached = error.attach("[true, false]\n");
        let related = error.declaration().attach("type items = [bool; 3]\n");
        let mut primary_text = String::new();
        let mut required_text = String::new();

        Textual::<'_, _, Fancy, _>::new(&mut primary_text)
            .render_mut_with_input(Present::with_input(FancySettings::standard()), &attached)
            .expect("primary report");
        Textual::<'_, _, Fancy, _>::new(&mut required_text)
            .render_mut_with_input(Present::with_input(FancySettings::standard()), &related)
            .expect("declaration report");

        assert!(primary_text.contains("expected 3 array elements, found 2"));
        assert!(primary_text.contains("[true, false]"));
        assert!(!primary_text.contains("type items"));
        assert!(required_text.contains("type items = [bool; 3]"));
        assert!(!required_text.contains("[true, false]"));
        assert_eq!(error.origin(), &origin);
        assert_eq!(error.declaration().origin(), &declaration);
    }
}
