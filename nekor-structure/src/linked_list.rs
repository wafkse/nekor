//! An intrusive doubly-linked list where list operations are not unsafe.
//!
//! Unsafety is removed through the introduction of an [`Inserted`]
//! pseudo-handle.
//!
//! To access a value in a list, you must first borrow the totality of the list.

use core::{
    marker,
    ops::{Deref, DerefMut},
    pin::Pin,
    ptr::{self, NonNull},
};

/// A trait for a forward-only linked list traversal through intrusive pointers.
///
/// # Safety
///
/// For a sound implementation to be achieved, the following must be satisfied
/// at any given time:
///
/// - The type must not be a part of any other list simultaneously.
///
/// - The type must have a [`Backward::prior`] that points to `self`.
///
/// - The type must have a [`Forward::next`] who back-references to `self`.
///
/// Furthermore, the node must unlink itself from the list when its lifetime is
/// due.
pub unsafe trait Forward: Linked {
    /// Determine the next node in the linked list.
    #[inline]
    #[must_use]
    fn next(self: Pin<&Self>) -> Option<NonNull<Self>> {
        *Self::next_slot(self)
    }

    /// Determine the next node in the linked list or fallback to a default.
    #[inline]
    #[must_use]
    fn next_or(self: Pin<&Self>, default: Pin<&Self>) -> NonNull<Self> {
        let next_default: &Self = &default;

        self.next()
            .unwrap_or_else(|| NonNull::from_ref(next_default))
    }

    /// Retrieve a reference to the pointer to the next node in the list.
    fn next_slot(self: Pin<&Self>) -> &Option<NonNull<Self>>;

    /// Retrieve a reference to the pointer to the next node in the list, in a
    /// mutable manner.
    fn next_slot_mut(self: Pin<&mut Self>) -> &mut Option<NonNull<Self>>;

    /// Replace the next node in the list with the provided value.
    ///
    /// Returns the previous next node, if any.
    ///
    /// # Safety
    ///
    /// - The value must not be a part of any other list.
    ///
    /// - The value must have a [`Backward::prior`] that points to `self`.
    #[inline]
    #[must_use]
    unsafe fn replace_next(self: Pin<&mut Self>, value: NonNull<Self>) -> Option<NonNull<Self>> {
        self.next_slot_mut().replace(value)
    }
}

/// A trait for a backward-only linked list traversal through intrusive
/// pointers.
///
/// # Safety
///
/// For a sound implementation to be achieved, the following must be satisfied
/// at any given time:
///
/// - The node must not be a part of any other list simultaneously.
///
/// - The node must have a [`Backward::prior`] that points to `self`.
///
/// - The node must have a [`Forward::next`] who back-references to `self`.
///
/// Furthermore, the node must unlink itself from the list when its lifetime is
/// due.
pub unsafe trait Backward: Linked {
    /// Determine the node prior to this one in the linked list.
    #[inline]
    #[must_use]
    fn prior(self: Pin<&Self>) -> Option<NonNull<Self>> {
        *Self::prior_slot(self)
    }

    #[inline]
    #[must_use]
    fn prior_or(self: Pin<&Self>, default: Pin<&Self>) -> NonNull<Self> {
        let prior_default: &Self = &default;

        self.prior()
            .unwrap_or_else(|| NonNull::from_ref(prior_default))
    }

    /// Retrieve a reference to the pointer to the next node in the list.
    fn prior_slot(self: Pin<&Self>) -> &Option<NonNull<Self>>;

    /// Retrieve a reference to the pointer to the next node in the list, in a
    /// mutable manner.
    fn prior_slot_mut(self: Pin<&mut Self>) -> &mut Option<NonNull<Self>>;

    /// Replace the prior node in the list with the provided value.
    ///
    /// Returns the previous prior node, if any.
    ///
    /// # Safety
    ///
    /// - The value must not be a part of any other list.
    ///
    /// - The value must have a [`Forward::next`] that points to `self`.
    #[inline]
    #[must_use]
    unsafe fn replace_prior(self: Pin<&mut Self>, value: NonNull<Self>) -> Option<NonNull<Self>> {
        self.prior_slot_mut().replace(value)
    }
}

/// A trait that describes an intrusive node with bidirectional links.
pub trait Bidirectional: Backward + Forward {}

impl<B> Bidirectional for B where B: Backward + Forward + ?Sized {}

/// A trait that determines whether a type can be included in a doubly linked
/// list.
///
/// # Safety
///
/// This trait imposes a strict requirement where an embedder `T` must be
/// logically disjoint from its embedding [`Link`].
///
/// Particularly, this implies that the [`Link`] can never be accessed by
/// non-unsafe APIs exposed from within `T`.
pub unsafe trait Linked {
    /// [`Pin`]-project to the [`Link`] in this [`Linked`] node in an immutable
    /// manner.
    ///
    /// # Safety
    ///
    /// The returned reference must only be used by a well-behaving linked list
    /// implementation.
    ///
    /// Particularly, it must abide by the safety contract contained in the
    /// [`Backward`], and [`Forward`] traits.
    unsafe fn link(self: Pin<&Self>) -> Pin<&Link<Self>>;

    /// [`Pin`]-project to the [`Link`] in this [`Linked`] node in a mutable
    /// manner.
    ///
    /// # Safety
    ///
    /// The returned reference must only be used by a well-behaving linked list
    /// implementation.
    ///
    /// Particularly, it must abide by the safety contract contained in the
    /// [`Backward`], and [`Forward`] traits.
    unsafe fn link_mut(self: Pin<&mut Self>) -> Pin<&mut Link<Self>>;
}

/// A self-contained link between two distinct nodes in a doubly-linked list.
///
/// # Pinned
///
/// This type is pinned to ensure that the list pointers are not invalidated by
/// accident.
#[derive(Debug)]
pub struct Link<T>
where
    T: ?Sized,
{
    /// The next node in the list.
    next_node: Option<NonNull<T>>,

    /// The previous node in the list.
    prior_node: Option<NonNull<T>>,

    /// A marker to make sure that any instance of this type is pinned.
    _marker: marker::PhantomPinned,
}

impl<T> Link<T>
where
    T: ?Sized,
{
    /// A new [`Link`] that links nothing together.
    #[inline]
    #[must_use]
    pub const fn nothing() -> Self {
        let next_node = None;
        let prior_node = None;

        Self {
            next_node,
            prior_node,
            _marker: marker::PhantomPinned,
        }
    }

    /// A new [`Link`] that is constructed from its bare components.
    #[inline]
    #[must_use]
    pub const fn raw((prior_node, next_node): (Option<NonNull<T>>, Option<NonNull<T>>)) -> Self {
        Self {
            next_node,
            prior_node,
            ..Self::nothing()
        }
    }

    /// A new [`Link`] that links to a single forward node.
    #[inline]
    #[must_use]
    pub const fn forward(target_node: NonNull<T>) -> Self {
        let next_node = Some(target_node);

        Self {
            next_node,
            ..Self::nothing()
        }
    }

    /// A new [`Link`] that links to a single backward node.
    #[inline]
    #[must_use]
    pub const fn backward(target_node: NonNull<T>) -> Self {
        let prior_node = Some(target_node);

        Self {
            prior_node,
            ..Self::nothing()
        }
    }

    /// A new [`Link`] that links to two nodes together.
    #[inline]
    #[must_use]
    pub const fn both(prior: NonNull<T>, next: NonNull<T>) -> Self {
        let next_node = Some(next);
        let prior_node = Some(prior);

        Self {
            next_node,
            prior_node,
            ..Self::nothing()
        }
    }
}

impl<T> Link<T> {
    /// Determine the prior node in the list relative to this link.
    #[inline]
    #[must_use]
    pub const fn prior(&self) -> Option<NonNull<T>> {
        let &Self { prior_node, .. } = self;

        prior_node
    }

    /// Determine the next node in the list relative to this link.
    #[inline]
    #[must_use]
    pub const fn next(&self) -> Option<NonNull<T>> {
        let &Self { next_node, .. } = self;

        next_node
    }

    /// Mutate the prior node in the list relative to this link.
    #[inline]
    pub const fn prior_mut(&mut self) -> &mut Option<NonNull<T>> {
        let &mut Self {
            ref mut prior_node, ..
        } = self;

        prior_node
    }

    /// Mutate the next node in the list relative to this link.
    #[inline]
    pub const fn next_mut(&mut self) -> &mut Option<NonNull<T>> {
        let &mut Self {
            ref mut next_node, ..
        } = self;

        next_node
    }

    /// Retrieve the pair of nodes that this link connects, as a 2-tuple.
    #[inline]
    #[must_use]
    pub const fn pair(&self) -> (Option<NonNull<T>>, Option<NonNull<T>>) {
        let &Self {
            next_node,
            prior_node,
            ..
        } = self;

        (next_node, prior_node)
    }

    /// Retrieve the `next` slot of this link.
    #[inline]
    #[must_use]
    pub const fn next_slot(&self) -> &Option<NonNull<T>> {
        let Self { next_node, .. } = self;

        next_node
    }

    /// Retrieve the `prior` slot of this link.
    #[inline]
    #[must_use]
    pub const fn prior_slot(&self) -> &Option<NonNull<T>> {
        let Self { prior_node, .. } = self;

        prior_node
    }

    /// Mutate the `next` slot of this link.
    #[inline]
    pub const fn next_slot_mut(&mut self) -> &mut Option<NonNull<T>> {
        let &mut Self {
            ref mut next_node, ..
        } = self;

        next_node
    }

    /// Mutate the `prior` slot of this link.
    #[inline]
    pub const fn prior_slot_mut(&mut self) -> &mut Option<NonNull<T>> {
        let &mut Self {
            ref mut prior_node, ..
        } = self;

        prior_node
    }
}

/// A handle to a value of type `T` that has been inserted into an
/// [`IntrusiveList`].
#[derive(Debug)]
pub struct Inserted<'a, T>
where
    T: Bidirectional + ?Sized,
{
    /// The address to the owning list.
    list_address: NonNull<IntrusiveList<T>>,

    /// The node that is being held inside the list.
    node_address: NonNull<T>,

    /// Marker to indicate ownership of a mutable reference to `T`.
    marker: marker::PhantomData<&'a mut T>,
}

impl<T> Inserted<'_, T>
where
    T: Bidirectional + ?Sized,
{
    /// Determine the [`NonNull<T>`] pointer to the inserted value.
    #[inline]
    #[must_use]
    pub const fn pointer(&self) -> NonNull<T> {
        let &Self { node_address, .. } = self;

        node_address
    }

    /// Determine the [`NonNull<T>`] pointer to the list used.
    #[inline]
    #[must_use]
    pub const fn list(&self) -> NonNull<IntrusiveList<T>> {
        let &Self { list_address, .. } = self;

        list_address
    }
}

/// An intrusive, doubly-linked list where the lifetime of the incorporated
/// elements is not managed by the list.
#[derive(Debug)]
pub struct IntrusiveList<T>
where
    T: Bidirectional + ?Sized,
{
    /// The head node in the linked list.
    // NOTE(invariant): `head_node->prior` must be unpopulated.
    head_node: Option<NonNull<T>>,

    /// The tail node in the linked list.
    // NOTE(invariant): `tail_node->next` must be unpopulated.
    tail_node: Option<NonNull<T>>,

    /// The amount of nodes incorporated in this linked list.
    node_count: usize,
}

impl<T> IntrusiveList<T>
where
    T: Bidirectional + ?Sized,
{
    /// Construct a brand-new, completely empty [`IntrusiveList`].
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        let head_node = None;
        let tail_node = None;
        let node_count = usize::MIN;

        Self {
            head_node,
            tail_node,
            node_count,
        }
    }
}

impl<T> Default for IntrusiveList<T>
where
    T: Bidirectional + ?Sized,
{
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T> IntrusiveList<T>
where
    T: Bidirectional + ?Sized,
{
    /// Attempt to insert the target value `T` at the back of the list.
    ///
    /// This will unequivocally fail if the value is already present in another
    /// list.
    ///
    /// # Safety
    ///
    /// The [`Inserted`] handle must be consumed again into this
    /// [`IntrusiveList`] with a proper [`IntrusiveList::try_remove`]
    /// invocation.
    #[must_use]
    pub unsafe fn try_insert_back<'a>(
        &mut self,
        target_value: Pin<&'a mut T>,
    ) -> Option<Inserted<'a, T>> {
        let &mut Self {
            head_node: ref mut list_head,
            tail_node: ref mut list_tail,
            ref mut node_count,
        } = self;

        match (target_value.as_ref().prior(), target_value.as_ref().next()) {
            (Some(..), ..) | (.., Some(..)) => return None,
            (None, None) => (),
        }

        // SAFETY: The contained `&mut T` is never unpinned, it is only coerced
        // to a pointer.
        let target_value = unsafe { Pin::into_inner_unchecked(target_value) };

        // SAFETY: Reference to non-null pointer coercion is always safe.
        let node_address = unsafe { NonNull::new_unchecked(target_value) };

        if let &mut Some(mut tail_address) = list_tail {
            // SAFETY: Was originally pinned.
            let tail_node = unsafe { Pin::new_unchecked(tail_address.as_mut()) };

            // SAFETY: Not part of any other list. Respective backlink
            // underway.
            let _ = unsafe { tail_node.replace_next(node_address) };

            // SAFETY: `target_value` was originally pinned. Link invariants
            // are satisfied at this point too.
            let _ = unsafe { Pin::new_unchecked(target_value).replace_prior(tail_address) };
        } else {
            let _ = list_head.replace(node_address);
        }
        let _ = list_tail.replace(node_address);

        *node_count += 1;

        // SAFETY: Reference to non-null pointer coercion is always safe.
        let list_address = unsafe { NonNull::new_unchecked(self) };

        Some(Inserted::<'a, T> {
            node_address,
            list_address,
            marker: marker::PhantomData,
        })
    }

    /// Attempt to insert the target value `T` at the front of the list.
    ///
    /// This will unequivocally fail if the value is already present in another
    /// list.
    ///
    ///  # Safety
    ///
    /// The [`Inserted`] handle must be consumed again into this
    /// [`IntrusiveList`] with a proper [`IntrusiveList::try_remove`]
    /// invocation.
    #[must_use]
    pub unsafe fn try_insert_front<'a>(
        &mut self,
        target_value: Pin<&'a mut T>,
    ) -> Option<Inserted<'a, T>> {
        let &mut Self {
            head_node: ref mut list_head,
            tail_node: ref mut list_tail,
            ref mut node_count,
        } = self;

        match (target_value.as_ref().prior(), target_value.as_ref().next()) {
            (Some(..), ..) | (.., Some(..)) => return None,
            (None, None) => (),
        }

        // SAFETY: The contained `&mut T` is never unpinned, it is only coerced
        // to a pointer.
        let target_value = unsafe { Pin::into_inner_unchecked(target_value) };

        // SAFETY: Reference to non-null pointer coercion is always safe.
        let node_address = unsafe { NonNull::new_unchecked(target_value) };

        if let &mut Some(mut head_address) = list_head {
            // SAFETY: Was originally pinned.
            let head_node = unsafe { Pin::new_unchecked(head_address.as_mut()) };

            // SAFETY: Not part of any other list. Respective backlink
            // underway.
            let _ = unsafe { head_node.replace_prior(node_address) };

            // SAFETY: `target_value` was originally pinned. Link invariants
            // are satisfied at this point too.
            let _ = unsafe { Pin::new_unchecked(target_value).replace_next(head_address) };

            let _ = list_head.replace(node_address);
        } else {
            let _ = list_head.replace(node_address);

            let _ = list_tail.replace(node_address);
        }

        *node_count += 1;

        // SAFETY: Reference to non-null pointer coercion is always safe.
        let list_address = unsafe { NonNull::new_unchecked(self) };

        Some(Inserted::<'a, T> {
            node_address,
            list_address,
            marker: marker::PhantomData,
        })
    }

    /// Try to unlock an [`Inserted`] value from being accessed immutably.
    ///
    /// This will not unlock the value if it happens to not be from this same
    /// list.
    #[inline]
    #[must_use]
    pub fn try_unlock<'a>(&'a self, target_handle: &Inserted<'a, T>) -> Option<Pin<&'a T>> {
        let &Inserted {
            list_address,
            node_address,
            ..
        } = target_handle;

        if ptr::eq(list_address.as_ptr().cast_const(), self) {
            Some(
                // SAFETY:
                //
                // The pointer is perfectly convertible to a reference - as it
                // as been sourced from a reference itself.
                //
                // Furthermore, ownership is local to this list due to the
                // safety contract.
                unsafe { Pin::new_unchecked(node_address.as_ref()) },
            )
        } else {
            None
        }
    }

    /// Try to unlock an [`Inserted`] value from being accessed in a mutable
    /// manner.
    ///
    /// This will not unlock the value if it happens to not be from this same
    /// list.
    #[inline]
    pub fn try_unlock_mut<'a>(
        &'a mut self,
        target_handle: &Inserted<'a, T>,
    ) -> Option<Pin<&'a mut T>> {
        let &Inserted {
            list_address,
            mut node_address,
            ..
        } = target_handle;

        if ptr::eq(list_address.as_ptr().cast_const(), self) {
            Some(
                // SAFETY:
                //
                // The pointer is perfectly convertible to a reference - as it
                // as been sourced from a reference itself.
                //
                // Furthermore, ownership is local to this list due to the
                // safety contract.
                unsafe { Pin::new_unchecked(node_address.as_mut()) },
            )
        } else {
            None
        }
    }

    /// Unlock an [`Inserted`] value from being accessed immutably.
    ///
    /// # Panics
    ///
    /// Panics if the target handle is not native to this list.
    #[must_use]
    #[inline]
    pub fn unlock<'a>(&'a self, target_handle: &Inserted<'a, T>) -> Pin<&'a T> {
        self.try_unlock(target_handle)
            .expect("handle is not native to this intrusive list")
    }

    /// Unlock an [`Inserted`] value from being accessed in a mutable manner.
    ///
    /// # Panics
    ///
    /// Panics if the target handle is not native to this list.
    #[must_use]
    #[inline]
    pub fn unlock_mut<'a>(&'a mut self, target_handle: &Inserted<'a, T>) -> Pin<&'a mut T> {
        self.try_unlock_mut(target_handle)
            .expect("handle is not native to this intrusive list")
    }

    /// Attempt to remove a node from this list.
    ///
    /// Fails if the specified [`Inserted`] handle is not native to this list.
    ///
    /// # Errors
    ///
    /// Returns the original handle when it belongs to another list.
    #[inline]
    #[allow(
        clippy::needless_pass_by_value,
        reason = "ownership proves the handle cannot remain usable after removal"
    )]
    pub fn try_remove<'a>(
        &mut self,
        target_handle: Inserted<'a, T>,
    ) -> Result<Pin<&'a mut T>, Inserted<'a, T>> {
        // SAFETY: Reference to non-null pointer coercion is always safe.
        let self_address = unsafe { NonNull::new_unchecked(self) };

        let &mut Self {
            ref mut head_node,
            ref mut tail_node,
            ref mut node_count,
        } = self;

        let Inserted {
            list_address,
            mut node_address,
            marker,
            ..
        } = target_handle;

        if ptr::eq(
            self_address.as_ptr().cast_const(),
            list_address.as_ptr().cast_const(),
        ) {
            // SAFETY: This was originally acquired through a `Pin<&mut _>`, and
            // we have borrowed the whole list mutably.
            let mut target_node = unsafe { Pin::new_unchecked(node_address.as_mut()) };

            let (prior_node, next_node) =
                (target_node.as_ref().prior(), target_node.as_ref().next());

            match (prior_node, next_node) {
                (Some(mut prior_node), Some(mut next_node)) => {
                    // SAFETY: [see previous safety comment]
                    let target_prior = unsafe { Pin::new_unchecked(prior_node.as_mut()) };

                    // SAFETY: [see previous safety comment]
                    let target_next = unsafe { Pin::new_unchecked(next_node.as_mut()) };

                    // SAFETY: Node is untouched during removal.
                    unsafe {
                        let _ = target_prior.replace_next(next_node);

                        let _ = target_next.replace_prior(prior_node);
                    };
                }
                (None, Some(mut next_node)) => {
                    // SAFETY: [see previous safety comment]
                    let target_next = unsafe { Pin::new_unchecked(next_node.as_mut()) };

                    // NOTE: The new head must not back-reference the removed
                    // node, as that link dangles once the node's lifetime is
                    // due.
                    *target_next.prior_slot_mut() = None;

                    *head_node = Some(next_node);
                }
                (Some(mut prior_node), None) => {
                    // SAFETY: [see previous safety comment]
                    let target_prior = unsafe { Pin::new_unchecked(prior_node.as_mut()) };

                    // NOTE: The new tail must not forward-reference the removed
                    // node, as that link dangles once the node's lifetime is
                    // due.
                    *target_prior.next_slot_mut() = None;

                    *tail_node = Some(prior_node);
                }
                (None, None) => (*head_node, *tail_node) = (None, None),
            }

            *target_node.as_mut().next_slot_mut() = None;
            *target_node.as_mut().prior_slot_mut() = None;

            *node_count -= 1;

            Ok(target_node)
        } else {
            Err(Inserted {
                list_address,
                node_address,
                marker,
            })
        }
    }
}

#[derive(Debug)]
pub struct External<T>(Link<Self>, T);

impl<T> External<T> {
    /// Wrap the target `T` into an externally-managed [`Link`] wrapper
    /// new-type.
    #[inline]
    pub const fn node(target_value: T) -> Self {
        Self(Link::nothing(), target_value)
    }

    /// Unwrap the target `T` from the wrapper.
    #[inline]
    pub fn unwrap(self) -> T {
        let Self(.., target_value) = self;

        target_value
    }
}

impl<T> Deref for External<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        let Self(.., target_value) = self;

        target_value
    }
}

impl<T> DerefMut for External<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        let Self(.., target_value) = self;

        target_value
    }
}

// SAFETY: The embedded `Link` is never accessed outside of an `unsafe fn`.
unsafe impl<T> Linked for External<T> {
    #[inline]
    unsafe fn link(self: Pin<&Self>) -> Pin<&Link<Self>> {
        let Self(target_link, ..) = self.get_ref();

        // SAFETY: The `Link` is pinned.
        unsafe { Pin::new_unchecked(target_link) }
    }

    #[inline]
    unsafe fn link_mut(self: Pin<&mut Self>) -> Pin<&mut Link<Self>> {
        // SAFETY: The `Link` is never moved.
        let &mut Self(ref mut target_link, ..) = unsafe { self.get_unchecked_mut() };

        // SAFETY: The `Link` is pinned.
        unsafe { Pin::new_unchecked(target_link) }
    }
}

// SAFETY: Abides by list logical invariants.
unsafe impl<T> Backward for External<T> {
    fn prior_slot(self: Pin<&Self>) -> &Option<NonNull<Self>> {
        let Self(target_link, ..) = self.get_ref();

        target_link.prior_slot()
    }

    fn prior_slot_mut(self: Pin<&mut Self>) -> &mut Option<NonNull<Self>> {
        // SAFETY: The `Link` is never moved.
        let &mut Self(ref mut target_link, ..) = unsafe { self.get_unchecked_mut() };

        target_link.prior_slot_mut()
    }
}

// SAFETY: Abides by list logical invariants.
unsafe impl<T> Forward for External<T> {
    fn next_slot(self: Pin<&Self>) -> &Option<NonNull<Self>> {
        let Self(target_link, ..) = self.get_ref();

        target_link.next_slot()
    }

    fn next_slot_mut(self: Pin<&mut Self>) -> &mut Option<NonNull<Self>> {
        // SAFETY: The `Link` is never moved.
        let &mut Self(ref mut target_link, ..) = unsafe { self.get_unchecked_mut() };

        target_link.next_slot_mut()
    }
}

#[test]
fn insertion_and_removal() {
    let mut v = External::node(0);
    let mut v2 = External::node(1);

    let value = unsafe { Pin::new_unchecked(&mut v) };
    let value2 = unsafe { Pin::new_unchecked(&mut v2) };

    let mut list = IntrusiveList::new();

    let first_handle = unsafe {
        list.try_insert_back(value)
            .expect("node is not in another list")
    };

    let second_handle = unsafe {
        list.try_insert_back(value2)
            .expect("node is not in another list")
    };

    let first = list
        .try_remove(first_handle)
        .expect("handle belongs to this list");
    let second = list
        .try_remove(second_handle)
        .expect("handle belongs to this list");

    assert_eq!(**first, 0);
    assert_eq!(**second, 1);
}
