use core::{cmp, fmt, hash, marker};

use nekor_bitwise::prelude::Extract;
use zerocopy::{Immutable, IntoBytes};

/// A new-type wrapper over a partioned integer primitive.
///
/// This is used in structures where an integer is divided into two or more
/// fields.
///
/// Note that this is transparent over `P`'s [`Extract::Output`] type.
#[derive(Clone, Copy, IntoBytes, Immutable)]
#[repr(transparent)]
// NOTE(invariant): The stored scalar is exactly the extracted partition output; the phantom
// parameter preserves invariance over the source primitive without affecting layout.
pub struct Partitioned<const N: usize, const M: usize, P>(
    // NOTE: `P::Output` is the smallest integer that can fit `M - N`
    // consecutive bits.
    P::Output,
    // NOTE(variance): Keep `Partitioned` explicitly invariant over `P`.
    marker::PhantomData<fn() -> P>,
)
where
    P: Extract<N, M>;

impl<const N: usize, const M: usize, P> Partitioned<N, M, P>
where
    P: Extract<N, M>,
{
    /// Construct new [`Partitioned`] new-type from the bare sawed-off bitset.
    #[inline]
    pub const fn raw(target_value: P::Output) -> Self {
        Self(target_value, marker::PhantomData)
    }

    /// Borrow the stored partition value.
    #[inline]
    #[must_use]
    pub const fn value(&self) -> &P::Output {
        let &Self(ref target_value, ..) = self;

        target_value
    }

    /// Mutably borrow the stored partition value.
    #[inline]
    pub const fn value_mut(&mut self) -> &mut P::Output {
        let &mut Self(ref mut target_value, ..) = self;

        target_value
    }

    /// Partition away the `N..=M` bits from the target value of type `P`.
    #[inline]
    pub fn away(target_value: P) -> Self {
        Self(Extract::<N, M>::extract(&target_value), marker::PhantomData)
    }

    /// Fuse this [`Partitioned`] value with the whole value of type `P`.
    #[inline]
    pub fn merge(self, target_whole: &mut P) -> P {
        let Self(target_value, ..) = self;

        let _: P::Output = Extract::<N, M>::merge(target_whole, target_value);

        *target_whole
    }
}

impl<const N: usize, const M: usize, P> PartialEq for Partitioned<N, M, P>
where
    P: Extract<N, M>,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        let &Self(ref left_value, ..) = self;

        let &Self(ref right_value, ..) = other;

        left_value == right_value
    }
}

impl<const N: usize, const M: usize, P> Eq for Partitioned<N, M, P> where P: Extract<N, M> {}

impl<const N: usize, const M: usize, P> PartialOrd for Partitioned<N, M, P>
where
    P: Extract<N, M>,
{
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        let &Self(ref left_value, ..) = self;

        let &Self(ref right_value, ..) = other;

        left_value.partial_cmp(right_value)
    }
}

impl<const N: usize, const M: usize, P> Ord for Partitioned<N, M, P>
where
    P: Extract<N, M>,
    P::Output: Ord,
{
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        let &Self(ref left_value, ..) = self;

        let &Self(ref right_value, ..) = other;

        left_value.cmp(right_value)
    }
}

impl<const N: usize, const M: usize, P> hash::Hash for Partitioned<N, M, P>
where
    P: Extract<N, M>,
    P::Output: hash::Hash,
{
    #[inline]
    fn hash<H>(&self, target_state: &mut H)
    where
        H: hash::Hasher,
    {
        let &Self(ref target_value, ..) = self;

        target_value.hash(target_state);
    }
}

impl<const N: usize, const M: usize, P> fmt::Debug for Partitioned<N, M, P>
where
    P: Extract<N, M>,
    P::Output: fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let &Self(ref target_value, ..) = self;

        f.write_fmt(format_args!("Partitioned::<{N}..{M}>({target_value:?})"))
    }
}
