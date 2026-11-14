//! Duplicate member reports with separate locations for declarations in different sources.

use alloc::boxed::Box;

use aldebaran::{
    prelude::Report,
    report::annotated::{InlineAnnotations, label::Label},
};
use fack::prelude::Error;

use crate::source::Origin;

/// A decoded member name was already declared in its record.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[report(forward)]
pub enum Duplicate {
    /// Both declarations can be annotated in the same source report.
    #[error(transparent(0))]
    Local(Box<Local>),

    /// The prior declaration needs its own source report.
    #[error(transparent(0))]
    Imported(Box<Imported>),
}

impl Duplicate {
    /// Select annotations according to the declarations' canonical source identities.
    #[inline]
    pub fn new(origin: Origin, previous: Origin) -> Self {
        if origin.source() == previous.source() {
            let labels = InlineAnnotations::new(
                Label::new("this member name is already declared", origin.span()),
                [Label::new("the first declaration is here", previous.span())],
            );

            Self::Local(Box::new(Local {
                origin,
                previous,
                labels,
            }))
        } else {
            let labels = InlineAnnotations::new(Label::new("this member name is already declared", origin.span()), []);
            let previous = Previous::new(previous);

            Self::Imported(Box::new(Imported {
                origin,
                previous,
                labels,
            }))
        }
    }

    /// Retrieve the duplicate key's canonical source and original coordinates.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        match self {
            Self::Local(report) => report.origin(),
            Self::Imported(report) => report.origin(),
        }
    }

    /// Retrieve the first declaration independently of the duplicate's source.
    #[inline]
    pub const fn previous(&self) -> &Origin {
        match self {
            Self::Local(report) => report.previous(),
            Self::Imported(report) => report.previous().origin(),
        }
    }
}

/// Duplicate declarations in the same canonical source.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("duplicate record member")]
#[report(title = "duplicate record member", annotations = labels)]
// NOTE(invariant): Both origins share a canonical source, which qualifies both labels.
pub struct Local {
    /// Source-qualified duplicate key occurrence.
    origin: Origin,

    /// Earlier key occurrence in the same source.
    previous: Origin,

    /// Duplicate and original key locations.
    labels: InlineAnnotations<Label<&'static str>, 1>,
}

impl Local {
    /// Retrieve the source qualifying both annotations.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        let Self { origin, .. } = self;

        origin
    }

    /// Retrieve the first declaration of the duplicate name.
    #[inline]
    pub const fn previous(&self) -> &Origin {
        let Self { previous, .. } = self;

        previous
    }
}

/// A duplicate member whose first declaration belongs to another source.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("duplicate record member from another source")]
#[report(title = "duplicate record member", annotations = labels)]
// NOTE(invariant): The primary labels belong to `origin`; `previous` owns its separate source.
pub struct Imported {
    /// Source-qualified duplicate key occurrence.
    origin: Origin,

    /// Related report qualified by the earlier declaration's source.
    previous: Previous,

    /// Duplicate key location only.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

impl Imported {
    /// Retrieve the source qualifying this report's primary label.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        let Self { origin, .. } = self;

        origin
    }

    /// Borrow the related report for rendering against its own source text.
    #[inline]
    pub const fn previous(&self) -> &Previous {
        let Self { previous, .. } = self;

        previous
    }
}

/// The source-qualified first declaration in a cross-source name conflict.
#[derive(Debug, Clone, Report)]
#[report(title = "previous member declaration", annotations = labels)]
// NOTE(invariant): The label uses the coordinates qualified by `origin`.
pub struct Previous {
    /// Canonical source and original coordinates of the earlier declaration.
    origin: Origin,

    /// Earlier declaration's location.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

impl Previous {
    /// Create a separate source report for the earlier declaration.
    #[inline]
    const fn new(origin: Origin) -> Self {
        let labels = InlineAnnotations::new(Label::new("the first declaration is here", origin.span()), []);

        Self { origin, labels }
    }

    /// Retrieve the canonical source qualifying this report's annotation.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        let Self { origin, .. } = self;

        origin
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::String;

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

    use super::Duplicate;
    use crate::source::{Id, Origin};

    #[test]
    fn annotation_forwarding_keeps_local_labels_and_separates_imported_coordinates() {
        let first = Origin::new(Id::new("local"), Span::unit(1));
        let second = Origin::new(Id::new("local"), Span::unit(4));
        let local = Duplicate::new(second.clone(), first.clone());

        assert_eq!(local.annotations().list().expect("labels").len().get(), 2);

        let imported = Origin::new(Id::new("imported"), Span::unit(1));
        let error = Duplicate::new(second.clone(), imported.clone());

        match error {
            Duplicate::Imported(report) => {
                assert_eq!(report.annotations().list().expect("primary").len().get(), 1);
                assert_eq!(report.previous().annotations().list().expect("related").len().get(), 1);
                assert_eq!(report.origin(), &second);
                assert_eq!(report.previous().origin(), &imported);
            },
            Duplicate::Local(_) => unreachable!("different canonical sources"),
        }
    }

    #[test]
    fn rendered_import_conflicts_attach_each_part_to_its_own_source_text() {
        let previous = Origin::new(Id::new("definitions"), Span::unit(0));
        let origin = Origin::new(Id::new("configuration"), Span::unit(0));
        let error = Duplicate::new(origin, previous);

        match error {
            Duplicate::Imported(report) => {
                let primary = report.attach("key = false\n");
                let related = report.previous().attach("key = true\n");
                let mut rendered_primary = String::new();
                let mut rendered_related = String::new();

                Textual::<'_, _, Fancy, _>::new(&mut rendered_primary)
                    .render_mut_with_input(Present::with_input(FancySettings::standard()), &primary)
                    .expect("primary report");
                Textual::<'_, _, Fancy, _>::new(&mut rendered_related)
                    .render_mut_with_input(Present::with_input(FancySettings::standard()), &related)
                    .expect("related report");

                assert!(rendered_primary.contains("key = false"));
                assert!(!rendered_primary.contains("key = true"));
                assert!(rendered_related.contains("key = true"));
                assert!(!rendered_related.contains("key = false"));
                assert!(rendered_primary.contains("this member name is already declared"));
                assert!(rendered_related.contains("the first declaration is here"));
            },
            Duplicate::Local(_) => unreachable!("different canonical sources"),
        }
    }
}
