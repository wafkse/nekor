//! Names and semantic document paths.

use core::{borrow::Borrow, fmt};

/// A node name in a typed configuration document.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Name(String);

impl Name {
    /// Construct a name from its KDL identifier value.
    #[inline]
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Determine the identifier value of the name.
    #[inline]
    #[must_use]
    pub const fn as_str(&self) -> &str {
        let &Self(ref value) = self;

        value.as_str()
    }
}

impl fmt::Display for Name {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A semantic path through a typed configuration document.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Path(Vec<Name>);

impl Path {
    /// Construct an empty document path.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// Construct a path containing one node name.
    #[inline]
    #[must_use]
    pub fn root(name: Name) -> Self {
        Self(vec![name])
    }

    /// Construct a child path without mutating the current path.
    #[inline]
    #[must_use]
    pub fn child(&self, name: Name) -> Self {
        let &Self(ref parts) = self;
        let mut target = parts.clone();

        target.push(name);
        Self(target)
    }

    /// Prefix this path with one parent node name.
    #[inline]
    #[must_use]
    pub fn prefixed(self, name: Name) -> Self {
        let Self(parts) = self;
        let mut target = Vec::with_capacity(parts.len() + 1);

        target.push(name);
        target.extend(parts);
        Self(target)
    }

    /// Determine whether the path is empty.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        let &Self(ref parts) = self;

        parts.is_empty()
    }

    /// Iterate over the names that form this path.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &Name> {
        let &Self(ref parts) = self;

        parts.iter()
    }
}

impl fmt::Display for Path {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut iter = self.iter();

        if let Some(first) = iter.next() {
            write!(formatter, "{first}")?;
        }

        for part in iter {
            write!(formatter, ".{part}")?;
        }

        Ok(())
    }
}

impl AsRef<str> for Name {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for Name {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}
