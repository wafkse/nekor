use core::{
    marker::PhantomPinned,
    ops::{Deref, DerefMut},
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
};

use nekor_project::{PinnedDrop, Project};

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Project)]
    struct Named<T> {
        #[project(pin)]
        pinned: T,
        value: usize,
    }

    #[derive(Project)]
    struct Tuple<T>(#[project(pin)] T, usize);

    #[derive(Project)]
    struct Empty;

    #[derive(Project)]
    struct EmptyTuple;

    #[derive(Project)]
    struct Unit<const N: usize>;

    #[derive(Project)]
    struct Generic<'project, T, const N: usize>
    where
        T: 'project,
    {
        #[project(pin)]
        pinned: T,
        borrowed: &'project T,
        bytes: [u8; N],
    }

    #[derive(Project)]
    enum State<T> {
        Ready {
            #[project(pin)]
            value: T,
            generation: usize,
        },
        Waiting(#[project(pin)] T, usize),
        Complete,
    }

    #[derive(Project)]
    #[project(unsafe = Unpin)]
    #[project(unsafe = Drop)]
    #[project(unsafe = Deref)]
    #[project(unsafe = DerefMut)]
    struct UnsafeOptOut<'a> {
        #[project(pin)]
        pinned: PhantomPinned,

        dropped: &'a AtomicBool,
    }

    impl Unpin for UnsafeOptOut<'_> {}

    impl Drop for UnsafeOptOut<'_> {
        fn drop(&mut self) {
            let &mut Self { ref mut dropped, .. } = self;

            dropped.store(true, Ordering::Relaxed);
        }
    }

    impl Deref for UnsafeOptOut<'_> {
        type Target = PhantomPinned;

        fn deref(&self) -> &Self::Target {
            let &Self { ref pinned, .. } = self;

            pinned
        }
    }

    impl DerefMut for UnsafeOptOut<'_> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            let &mut Self { ref mut pinned, .. } = self;

            pinned
        }
    }

    #[derive(Project)]
    #[project(unsafe = Drop)]
    struct DropTarget<'a> {
        #[project(pin)]
        pinned: PhantomPinned,
        dropped: &'a AtomicBool,
    }

    // SAFETY: The implementation only observes the pinned field and updates the
    // independent drop flag. It does not move the source or change its variant.
    unsafe impl PinnedDrop for DropTarget<'_> {
        unsafe fn drop(self: Pin<&mut Self>) {
            let DropTargetProjectionMut { pinned, dropped } = self.project_mut();
            let _: Pin<&mut PhantomPinned> = pinned;
            dropped.store(true, Ordering::Relaxed);
        }
    }

    impl Drop for DropTarget<'_> {
        fn drop(&mut self) {
            // SAFETY: `Drop::drop` keeps this value in place while the pinned drop
            // hook runs and this implementation forwards exactly once.
            let pinned = unsafe { Pin::new_unchecked(self) };

            // SAFETY: This forwards from `Drop::drop` exactly once for the same
            // pinned value.
            unsafe { <Self as PinnedDrop>::drop(pinned) };
        }
    }

    #[test]
    fn projects_named_struct_fields() {
        let mut target_value = Box::pin(Named {
            pinned: PhantomPinned,
            value: 7,
        });

        let projection = target_value.as_ref().project();
        let _: Pin<&PhantomPinned> = projection.pinned;
        assert_eq!(*projection.value, 7);

        let projection_mut = target_value.as_mut().project_mut();
        *projection_mut.value = 11;

        assert_eq!(target_value.as_ref().project().value, &11);
    }

    #[test]
    fn projects_tuple_empty_and_unit_structures() {
        let tuple = Box::pin(Tuple(PhantomPinned, 13));
        let TupleProjection(pinned, value) = tuple.as_ref().project();
        let _: Pin<&PhantomPinned> = pinned;
        assert_eq!(*value, 13);

        let empty = Box::pin(Empty);
        let EmptyProjection = empty.as_ref().project();

        let empty_tuple = Box::pin(EmptyTuple);
        let EmptyTupleProjection = empty_tuple.as_ref().project();

        let unit = Box::pin(Unit::<4>);
        let UnitProjection = unit.as_ref().project();
    }

    #[test]
    fn preserves_lifetime_type_const_and_where_generics() {
        let borrowed = 23_usize;
        let mut target_value = Box::pin(Generic {
            pinned: 17_usize,
            borrowed: &borrowed,
            bytes: [1_u8, 2, 3],
        });

        let GenericProjection {
            pinned,
            borrowed,
            bytes,
        } = target_value.as_ref().project();
        assert_eq!(*pinned, 17);
        assert_eq!(**borrowed, 23);
        assert_eq!(*bytes, [1, 2, 3]);

        target_value.as_mut().project_mut().bytes[1] = 9;
        assert_eq!(target_value.as_ref().project().bytes[1], 9);
    }

    #[test]
    fn projects_every_enum_field_shape() {
        let ready = Box::pin(State::Ready {
            value: PhantomPinned,
            generation: 3,
        });

        match ready.as_ref().project() {
            StateProjection::Ready { value, generation } => {
                let _: Pin<&PhantomPinned> = value;
                assert_eq!(*generation, 3);
            },
            StateProjection::Waiting(..) | StateProjection::Complete => unreachable!(),
        }

        let waiting = Box::pin(State::Waiting(PhantomPinned, 5));

        match waiting.as_ref().project() {
            StateProjection::Waiting(value, generation) => {
                let _: Pin<&PhantomPinned> = value;
                assert_eq!(*generation, 5);
            },
            StateProjection::Ready { .. } | StateProjection::Complete => unreachable!(),
        }

        let complete = Box::pin(State::<PhantomPinned>::Complete);
        assert!(matches!(complete.as_ref().project(), StateProjection::Complete));
    }

    #[test]
    fn structural_unpin_uses_only_pinned_fields() {
        fn require_unpin<T>()
        where
            T: Unpin,
        {
        }

        require_unpin::<Named<usize>>();
        require_unpin::<Generic<'static, usize, 1>>();
        require_unpin::<UnsafeOptOut<'static>>();

        let dropped = AtomicBool::new(false);

        {
            let _target_value = Box::pin(UnsafeOptOut {
                pinned: PhantomPinned,
                dropped: &dropped,
            });
        }

        assert!(dropped.load(Ordering::Relaxed));
    }

    #[test]
    fn forwards_pinned_drop() {
        let dropped = AtomicBool::new(false);

        {
            let _target_value = Box::pin(DropTarget {
                pinned: PhantomPinned,
                dropped: &dropped,
            });
        }

        assert!(dropped.load(Ordering::Relaxed));
    }
}
