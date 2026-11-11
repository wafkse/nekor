//! Named members in declaration order, with unique decoded names.
//!
//! Each record member has a declaration location separate from its value's location.
//! This representation is shared by structural types and configuration values,
//! so lookup and composition cannot silently replace duplicate declarations.

use alloc::{
    sync::Arc,
    vec::{IntoIter, Vec},
};
use core::{convert::Infallible, slice::Iter};

use crate::source::Origin;

pub mod duplicate;

use duplicate::Duplicate;

/// A decoded member name, its declaration location, and its semantic value.
#[derive(Debug, Clone, PartialEq, Eq)]
// NOTE(invariant): None.
pub struct Member<T> {
    /// Decoded member name without normalization.
    name: Arc<str>,

    /// Source-qualified key occurrence.
    origin: Origin,

    /// Meaning assigned to this member.
    value: T,
}

impl<T> Member<T> {
    /// Associate a semantic member with its decoded source name.
    #[inline]
    pub fn new(name: impl Into<Arc<str>>, origin: Origin, value: T) -> Self {
        let name = name.into();

        Self { name, origin, value }
    }

    /// Borrow the exact decoded member name.
    #[inline]
    pub fn name(&self) -> &str {
        let Self { name, .. } = self;

        name
    }

    /// Retrieve the key's source identity and original coordinates.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        let Self { origin, .. } = self;

        origin
    }

    /// Inspect the meaning associated with this member.
    #[inline]
    pub const fn value(&self) -> &T {
        let Self { value, .. } = self;

        value
    }

    /// Consume the member while transferring its semantic value.
    #[inline]
    pub fn into_value(self) -> T {
        let Self { value, .. } = self;

        value
    }

    /// Transform a member's value without changing its name or source location.
    #[inline]
    pub fn map<U>(self, operation: impl FnOnce(T) -> U) -> Member<U> {
        let Self { name, origin, value } = self;

        let value = operation(value);

        Member { name, origin, value }
    }
}

/// Unique named members whose iteration follows declaration order.
#[derive(Debug, Clone, PartialEq, Eq)]
// NOTE(invariant): Decoded names are unique, and members remain in insertion order.
pub struct Record<T>(Vec<Member<T>>);

impl<T> Record<T> {
    /// Merge colliding meanings while keeping inherited positions and appending new names.
    #[inline]
    pub fn merge(self, other: Self, mut operation: impl FnMut(&str, T, T) -> T) -> Self {
        match self.try_merge(other, |name, left, right| {
            Ok::<T, Infallible>(operation(name, left, right))
        }) {
            Ok(record) => record,
            Err(never) => match never {},
        }
    }

    /// Combine collisions fallibly without exposing a way to rename or duplicate members.
    #[inline]
    pub fn try_merge<E>(self, other: Self, mut operation: impl FnMut(&str, T, T) -> Result<T, E>) -> Result<Self, E> {
        let Self(parent) = self;

        let Self(child) = other;

        let mut child = child.into_iter().map(Some).collect::<Vec<_>>();
        let mut members = Vec::new();

        for member in parent {
            let Member { name, origin, value } = member;

            let incoming = child
                .iter_mut()
                .find(|incoming| incoming.as_ref().is_some_and(|member| member.name() == name.as_ref()))
                .and_then(Option::take);
            let value = match incoming {
                Some(incoming) => operation(&name, value, incoming.into_value())?,
                None => value,
            };

            members.push(Member { name, origin, value });
        }

        members.extend(child.into_iter().flatten());

        Ok(Self(members))
    }

    /// Transform member meanings without changing decoded names, order, or key locations.
    #[inline]
    pub fn map<U>(self, mut operation: impl FnMut(&str, &Origin, T) -> U) -> Record<U> {
        match self.try_map(|name, origin, value| Ok::<U, Infallible>(operation(name, origin, value))) {
            Ok(record) => record,
            Err(never) => match never {},
        }
    }

    /// Transform member meanings fallibly while preserving their established uniqueness.
    #[inline]
    pub fn try_map<U, E>(self, mut operation: impl FnMut(&str, &Origin, T) -> Result<U, E>) -> Result<Record<U>, E> {
        let Self(members) = self;

        let members = members
            .into_iter()
            .map(|member| {
                let Member { name, origin, value } = member;

                let value = operation(&name, &origin, value)?;

                Ok(Member { name, origin, value })
            })
            .collect::<Result<Vec<_>, E>>()?;

        Ok(Record(members))
    }

    /// Start an empty record without reserving an artificial member limit.
    #[inline]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// Append a member, rejecting a name already present in the record.
    #[inline]
    pub fn insert(&mut self, member: Member<T>) -> Result<(), Duplicate> {
        let Self(members) = self;

        match members.iter().find(|existing| existing.name() == member.name()) {
            Some(previous) => Err(Duplicate::new(member.origin().clone(), previous.origin().clone())),
            None => {
                members.push(member);

                Ok(())
            },
        }
    }

    /// Find a member by its exact decoded name.
    #[inline]
    pub fn get(&self, name: &str) -> Option<&Member<T>> {
        let Self(members) = self;

        members.iter().find(|member| member.name() == name)
    }

    /// Traverse members in declaration order.
    #[inline]
    pub fn iter(&self) -> Iter<'_, Member<T>> {
        let Self(members) = self;

        members.iter()
    }

    /// Retrieve the number of unique members.
    #[inline]
    pub const fn len(&self) -> usize {
        let Self(members) = self;

        members.len()
    }

    /// Test whether the record has any members.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        let Self(members) = self;

        members.is_empty()
    }
}

impl<T> Default for Record<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T> IntoIterator for Record<T> {
    type IntoIter = IntoIter<Member<T>>;
    type Item = Member<T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        let Self(members) = self;

        members.into_iter()
    }
}

impl<'record, T> IntoIterator for &'record Record<T> {
    type IntoIter = Iter<'record, Member<T>>;
    type Item = &'record Member<T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use aldebaran::prelude::Span;

    use super::{Member, Record};
    use crate::source::{Id, Origin};

    #[test]
    fn declaration_order_survives_lookup_and_owned_traversal() {
        let origin = Origin::new(Id::new("record"), Span::unit(0));
        let mut record = Record::new();

        record.insert(Member::new("z", origin.clone(), 1)).expect("first");
        record.insert(Member::new("a", origin.clone(), 2)).expect("second");
        record.insert(Member::new("m", origin, 3)).expect("third");

        assert_eq!(record.get("a").expect("member").value(), &2);
        assert_eq!(record.get("absent"), None);
        assert_eq!(record.iter().map(Member::name).collect::<Vec<_>>(), ["z", "a", "m"]);
        assert_eq!(
            record.into_iter().map(Member::into_value).collect::<Vec<_>>(),
            [1, 2, 3]
        );
    }

    #[test]
    fn duplicate_admission_preserves_the_first_member_and_both_source_locations() {
        let first = Origin::new(Id::new("first"), Span::unit(1));
        let second = Origin::new(Id::new("second"), Span::unit(2));
        let mut record = Record::new();

        record.insert(Member::new("key", first.clone(), 1)).expect("first");

        let error = record
            .insert(Member::new("key", second.clone(), 2))
            .expect_err("duplicate");

        assert_eq!(error.origin(), &second);
        assert_eq!(error.previous(), &first);
        assert_eq!(record.len(), 1);
        assert_eq!(record.get("key").expect("original member").value(), &1);
    }
}
