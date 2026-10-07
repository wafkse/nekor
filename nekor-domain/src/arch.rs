#![cfg_attr(
    miri,
    allow(
        unused_imports,
        dead_code,
        reason = "to avoid extra `cfg(...)` gates on module items"
    )
)]
//! Per-architecture storage container definitions.

use core::{
    any, arch,
    cell::UnsafeCell,
    fmt, marker,
    mem::{self, MaybeUninit},
    ptr::NonNull,
    sync::atomic::{AtomicU8, Ordering},
};

use crate::{domain::Domain, prelude::Preset, store::Store};

/// A header inside a static storage slot.
#[derive(Debug)]
#[repr(transparent)]
// NOTE(invariant): The atomic byte always contains a valid `InitializationStage` discriminant.
pub struct Header(AtomicU8);

impl Header {
    /// Determine the stored [`InitializationStage`] in the header.
    #[inline]
    pub fn load(&self) -> InitializationStage {
        let &Self(ref target_value) = self;

        let Some(target_stage) = InitializationStage::raw(target_value.load(Ordering::Acquire)) else {
            unreachable!()
        };

        target_stage
    }

    /// Store the target stage into the header.
    ///
    /// # Safety
    ///
    /// The associated storage to this header must be initialized if a
    /// [`InitializationStage::Initialized`] is provided.
    #[inline]
    pub unsafe fn store(&self, target_stage: InitializationStage) {
        let &Self(ref target_value) = self;

        target_value.store(target_stage as u8, Ordering::Release);
    }

    /// Migrate the stored [`InitializationStage`] in the header to a distinct
    /// stage.
    ///
    /// # Failure
    ///
    /// On failure, this yields the unexpected stage read from the same header.
    ///
    /// # Safety
    ///
    /// The associated storage to this header must be initialized if a
    /// [`InitializationStage::Initialized`] is migrated to.
    #[inline]
    pub unsafe fn migrate(
        &self,
        expected_stage: InitializationStage,
        next_stage: InitializationStage,
    ) -> Option<InitializationStage> {
        let &Self(ref target_value) = self;

        match target_value.compare_exchange(
            expected_stage as u8,
            next_stage as u8,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(..) => None,
            Err(target_value) => InitializationStage::raw(target_value),
        }
    }
}

/// A singular initialization stage for a static.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
#[repr(u8)]
pub enum InitializationStage {
    /// The static storage remains uninitialized.
    #[default]
    // NOTE: The zero discriminant matches the hardcoded storage assembly.
    Uninitialized = 0,

    /// Another thread is initializing the static storage.
    ///
    /// The current thread spins until it is initialized.
    Pending,

    /// The static storage is initialized and ready to be read.
    Initialized,
}

impl InitializationStage {
    /// Interpret a raw [`u8`] as an initialization stage.
    #[inline]
    #[must_use]
    pub const fn raw(target_value: u8) -> Option<Self> {
        const UNINITIALIZED: u8 = InitializationStage::uninitialized();
        const PENDING: u8 = InitializationStage::pending();
        const INITIALIZED: u8 = InitializationStage::initialized();

        match target_value {
            UNINITIALIZED | PENDING | INITIALIZED => {
                let target_stage = match target_value {
                    UNINITIALIZED => Self::Uninitialized,
                    PENDING => Self::Pending,
                    INITIALIZED => Self::Initialized,
                    _ => unreachable!(),
                };

                Some(target_stage)
            },
            _ => None,
        }
    }
}

impl InitializationStage {
    /// The [`u8`] value tag that represents the uninitialized stage.
    #[inline]
    #[must_use]
    pub const fn uninitialized() -> u8 {
        Self::Uninitialized as u8
    }

    /// The [`u8`] value tag that represents the pending initialization stage.
    #[inline]
    #[must_use]
    pub const fn pending() -> u8 {
        Self::Pending as u8
    }

    /// The [`u8`] value tag that represents the initialized stage.
    #[inline]
    #[must_use]
    pub const fn initialized() -> u8 {
        Self::Initialized as u8
    }
}

/// A static storage allocation for a type `T`.
///
/// Unlike [`Container`], this does not require pointer dereference, which in
/// turn makes it ideal for dealing with statics in const-eval.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The stored offset reaches the aligned `Container<T>` emitted for this storage
// allocation.
pub struct Allocation<T>(usize, marker::PhantomData<fn() -> T>)
where
    T: Store;

impl<T> Allocation<T>
where
    T: Store,
{
    /// Determine the address of the [`Allocation`] in the [`Preset`]
    /// [`Domain`].
    #[inline]
    #[must_use]
    pub fn address() -> NonNull<Self> {
        Self::address_in::<Preset>()
    }

    /// Determine the address of the [`Allocation`] slot for `T` in the
    /// [`Domain`] `D`.
    ///
    /// # Remarks
    ///
    /// Due to `const`-eval limitations, the following remarks must be
    /// well-understood to be able to consume the [`Allocation`] in a safe
    /// manner.
    ///
    /// The main issue is returned pointer *does (at least) partially point* to
    /// an [`Allocation`], but is not guaranteed to be aligned.
    ///
    /// Therefore, any accesses to the target [`Allocation`] must make sure to
    /// use an unaligned load.
    ///
    /// Nevertheless, since the respective storage address is known to the
    /// compiler, the performance penalty of an explicit unaligned load may
    /// be superceded altogheter.
    ///
    /// However, the returned pointer is always guaranteed to be aligned to
    /// whatever the native function alignment is.
    ///
    /// This is a byte-level invariant, and therefore *can be relied upon in
    /// unsafe code*, as long as the pointer is aligned manually.
    #[inline]
    #[cfg(not(miri))]
    #[must_use]
    pub fn address_in<D>() -> NonNull<Self>
    where
        D: Domain,
    {
        // NOTE:
        //  The cast is valid as we have handwritten the allocation bytes.
        let alloc_addr = self::storage::<D::Adapter<T>> as *mut Self;

        // SAFETY: Always non-null, as it originates from a `fn item`.
        unsafe { NonNull::new_unchecked(alloc_addr) }
    }

    /// This is a specialization function for Miri-only use.
    ///
    /// For the actual documentation, see this same path but with the `miri`
    /// `cfg`-attr disabled.
    #[inline]
    #[cfg(miri)]
    pub fn address_in<D>() -> NonNull<Self>
    where
        D: Domain,
    {
        let slab = self::miri::Slab::find_or_create::<D::Adapter<T>>();

        let storage = slab.storage();

        storage.allocation().cast()
    }
}

impl<T> Allocation<T>
where
    T: Store,
{
    /// Determine the forward offset to the [`inline storage`] for `T`.
    ///
    /// [`inline storage`]: Container
    #[inline]
    #[must_use]
    pub const fn offset(&self) -> usize {
        let &Self(target_value, ..) = self;

        target_value
    }
}

/// A container for generic static storage.
#[repr(C, packed)]
// NOTE(invariant): The header tracks initialization of the adjacent `value_storage` for the same
// static allocation.
pub struct Container<T>
where
    T: Store,
{
    /// The heads-first inline storage for `T`.
    // NOTE(rationale): Static and zeroed storage paths require crate-wide access to this field.
    pub(crate) value_storage: UnsafeCell<MaybeUninit<T>>,

    /// The initialization header of the storage.
    // NOTE(rationale): Static and zeroed storage paths require crate-wide access to this field.
    pub(crate) value_header: Header,
}

impl<T> Container<T>
where
    T: Store,
{
    /// Determine the address of the [`Container`] in the [`Preset`] [`Domain`].
    #[inline]
    #[must_use]
    pub fn address() -> NonNull<Self> {
        Self::address_in::<Preset>()
    }

    /// Determine the address of the [`Container`] slot for `T` in the
    /// [`Domain`] `D`.
    ///
    /// This method is safe due to intrinsic guarantees on storage layout.
    ///
    /// # Remarks
    ///
    /// The returned pointer is aligned to `T`'s alignment boundary.
    /// And since the [`Header`] is 1-aligned, all fields can be read without
    /// alignment issues whatsoever.
    ///
    /// This *can be relied upon in unsafe code*.
    ///
    /// For a const-compatible reference, see the [`Allocation`] type.
    #[inline]
    #[must_use]
    pub fn address_in<D>() -> NonNull<Self>
    where
        D: Domain,
    {
        // NOTE:
        //  This is only aligned to the architecture-defined function alignment,
        // not to  the alignment `T` requires.
        //  So we just store a relative offset to the aligned allocation
        // manually.
        let unaligned_address = Allocation::<T>::address_in::<D>();

        // FIXME: Revisit whether this can except in alignment-checking architectures.

        // SAFETY: Value is hardcoded in. Always valid.
        let target_alloc: &Allocation<T> = &unsafe { unaligned_address.read_unaligned() };

        // SAFETY: The relative offset to the `Container` is hardcoded
        // automatically by the compiler, therefore, this new pointer does not
        // violate provenance requirements.
        unsafe {
            unaligned_address
                .byte_add(Allocation::offset(target_alloc))
                .cast::<Self>()
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for Container<T>
where
    T: Store,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Container").finish_non_exhaustive()
    }
}

/// A `fn item` that reserves inline space for a single value of `T` at
/// compile-time.
///
/// # Implementation
///
/// This symbol does the following, in mention order:
///
/// - Reserve enough space to accomodate a `T` inline.
/// - Emit a single *initialization byte* to lazily initialize `T` in a concurrent and lock-free
///   manner.
/// - Emit a pc-relative relocation to a specific per-`T` symbol: [`TypeId::of<T>`] to ensure
///   uniqueness of monomorphized [`storage`] symbols (particularly when different `T`s
///   coincidentally have the same size) against agressive optimization, both compiler-wise and
///   linker-wise.
///
/// # Safety
///
/// - This function must never be called.
///
/// [`TypeId::of<T>`]: any::TypeId::of
#[unsafe(link_section = concat!(env!("KERNEL_DOMAIN_STORAGE_SECTION"), ".offset"))]
#[unsafe(naked)]
// NOTE(rationale): The architecture storage symbol is shared with its parent module.
pub(super) unsafe extern "C" fn storage<T>() -> !
where
    T: Store,
{
    // NOTE:
    //  The use of relative addressing here is intentional.
    //
    //  As of writing, this works for every architecture the `asm!` macro can be
    // used in.
    arch::naked_asm!(
        // SAFETY(1): Relative offset to actual storage bytes.
        ".{pointer_width}byte 2f - .",
        concat!(
            ".pushsection ", env!("KERNEL_DOMAIN_STORAGE_SECTION"), ".typename.{id_monomorphic}", ", \"aw\", @progbits"
        ),
        ".balign {type_alignment}",
        "2:",
        ".fill {type_size}, 1, 0x00",
        ".byte {stage_uninitialized}",
        ".balign {pointer_alignment}",
        ".{pointer_width}byte {id_monomorphic} - .", /* pointer_width = {2,4,8} -> {2byte,4byte,8byte}; */
        ".popsection",
        pointer_width = const mem::size_of::<usize>(),
        pointer_alignment = const mem::align_of::<usize>(),
        id_monomorphic = sym any::TypeId::of::<T>,
        stage_uninitialized = const InitializationStage::uninitialized(),
        type_alignment = const mem::align_of::<T>(),
        type_size = const mem::size_of::<T>(),
        options(att_syntax),
    )
}

// FIXME: This was written when the possibility of Miri being multi-threaded in
// the Future could be possible.
//
// Revisit whether more lock-free machinery is actually benefitial in this case.

#[cfg(miri)]
pub(crate) mod miri {
    //! Specialization for Miri support.
    //!
    //! Miri is unable to interpret code uses inline assembly.

    use core::{
        any::TypeId,
        cell::UnsafeCell,
        mem::{self, MaybeUninit},
        ptr::{self, NonNull},
        sync::atomic::{AtomicPtr, Ordering},
    };

    use nekor_miri::memory::Allocation;

    use crate::{arch::Container, store::Store};

    /// A slab allocator for specializing Miri support into the `nekor-domain`
    /// subsystem.
    ///
    /// This stores types by their respective [`TypeId`] only once.
    ///
    /// In this case, it performs an `O(n)` search rather than a constant-time
    /// dereference, but it remains unimportant as long as no divergent
    /// logic in non-`arch` code between regular execution and Miri is present.
    ///
    /// # Lock-free
    ///
    /// Due to the fact that this slab allocator does not require freeing any
    /// used up slabs, one can force it to be lock-free.
    ///
    /// Particulary, this is done through the following algorithm:
    ///
    /// ## Traversal
    ///
    /// The first allocated [`Slab`] will be for a globally-inaccessible
    /// uninhabited type (a.k.a `PhantomRestricted`), to retrieve the next
    /// [`Slab`], one must perform an `atomic load` with at least
    /// [`Ordering::Acquire`] to synchronize with threads that are adding
    /// additional slabs.
    ///
    /// This can be performed in a loop until the next slab pointer *is null*.
    ///
    /// ## Insertion
    ///
    /// Insertion is done through a fallible CAS loop, where a thread traverses
    /// to the last [`Slab`] in the allocator and attempts to perform an `atomic
    /// compare-and-exchange` operation with the:
    ///
    /// - [`Ordering::AcqRel`]: For a *successful operation*.
    /// - [`Ordering::Acquire`]: For a *failed operation*.
    ///
    /// In the case of a *failed operation* the retrying thread must again find
    /// the latest [`Slab`] that was allocated and attempt the operation again.
    ///
    /// This presents a very tiny contention footprint (as an [`AtomicPtr`] does
    /// not present any further changes in time after it is initialized to a
    /// [`Slab`]), which can allow finding internal concurrency problems.
    pub struct Slab {
        /// The pointer to next slab in the list.
        next_slab: AtomicPtr<Self>,

        /// The type identifier the [`Storage`] instance is for.
        type_id: TypeId,

        /// The Miri-backed storage for the target type.
        storage: Storage,
    }

    impl Slab {
        /// Return the permanent list head.
        #[inline]
        const fn global() -> &'static Self {
            static GLOBAL_SLAB: Slab = Slab::restricted();

            &GLOBAL_SLAB
        }

        /// A const-compatible builder for a [`Slab`].
        const fn restricted() -> Self {
            /// A restricted type to avoid usage of allocator-dependent
            /// features.
            ///
            /// This effectively prevents access to the underlying [`Storage`],
            /// as no accesses are performed without a matching type identifier
            enum PhantomRestricted {}

            let next_slab = AtomicPtr::new(ptr::null_mut());

            let type_id = TypeId::of::<PhantomRestricted>();

            // NOTE: `Storage` is never accessed without a matching type id.
            let storage = Storage(NonNull::dangling());

            Slab {
                next_slab,
                type_id,
                storage,
            }
        }

        /// Allocate an unpublished candidate for one storage key.
        fn instance<T>() -> Candidate
        where
            T: Store,
        {
            let type_id = TypeId::of::<T>();

            let slab = Allocation::new::<Slab>().expect("allocate slab");

            let next_slab = AtomicPtr::new(ptr::null_mut());

            let (storage, storage_allocation) = Storage::allocate::<T>();

            // SAFETY: The allocation has the Slab layout and remains owned by the candidate.
            unsafe {
                slab.pointer()
                    .cast::<MaybeUninit<Slab>>()
                    .as_ptr()
                    .write(MaybeUninit::new(Self {
                        next_slab,
                        type_id,
                        storage,
                    }))
            };

            Candidate {
                slab,
                storage: storage_allocation,
            }
        }

        /// Attempt to chain a new [`Slab`] to the list.
        ///
        /// - [`None`] if the chaining operation was successful.
        /// - [`Some`] if the chaining operation failed, with the new [`Slab`].
        fn chain(&self, slab: *mut Slab) -> Option<&'static Self> {
            let &Self { ref next_slab, .. } = self;

            match next_slab.compare_exchange(ptr::null_mut(), slab, Ordering::AcqRel, Ordering::Acquire) {
                Ok(..) => None,
                Err(actual_next) => {
                    // SAFETY: Pointer has been checked to be non-null.
                    let nonnull = unsafe { NonNull::new_unchecked(actual_next) };

                    Some(
                        // SAFETY: Has been sourced from a `&'static Slab`
                        unsafe { nonnull.as_ref() },
                    )
                },
            }
        }

        /// Determine if a [`Slab`] is destined for a type `T`.
        #[inline]
        fn is<T>(&self) -> bool
        where
            T: Store,
        {
            let &Self { type_id, .. } = self;

            type_id == TypeId::of::<T>()
        }

        /// Traverse the next published slab using acquire ordering.
        fn next_in_line(&self) -> Option<&'static Self> {
            let &Self { ref next_slab, .. } = self;

            let next_ptr = next_slab.load(Ordering::Acquire);

            if ptr::eq(next_ptr, ptr::null_mut()) {
                None
            } else {
                // SAFETY: Pointer has been checked to be non-null.
                let nonnull = unsafe { NonNull::new_unchecked(next_ptr) };

                Some(
                    // SAFETY: Has been sourced from a `&'static Slab`
                    unsafe { nonnull.as_ref() },
                )
            }
        }
    }

    impl Slab {
        /// Return the type-erased storage for this slab.
        #[inline]
        pub const fn storage(&self) -> &Storage {
            let &Self { ref storage, .. } = self;

            storage
        }

        /// Attempt to find the [`Slab`] holding the target type `T`.
        ///
        /// - [`Ok`] if found.
        /// - [`Err`] if not found, with the last [`Slab`] visited.
        #[inline]
        pub fn find<T>() -> Result<&'static Slab, &'static Slab>
        where
            T: Store,
        {
            let mut slab_ref = Self::global();

            loop {
                if slab_ref.is::<T>() {
                    break Ok(slab_ref);
                }

                let next = slab_ref.next_in_line();

                match next {
                    Some(next) => slab_ref = next,
                    None => break Err(slab_ref),
                }
            }
        }

        /// Find or create a new [`Slab`] for a `T`.
        pub fn find_or_create<T>() -> &'static Self
        where
            T: Store,
        {
            match Self::find::<T>() {
                Ok(found_slab) => found_slab,
                Err(mut last_slab) => {
                    let new_slab = Slab::instance::<T>();

                    loop {
                        match last_slab.chain(new_slab.pointer()) {
                            Some(new_last) => {
                                if new_last.is::<T>() {
                                    break new_last;
                                }

                                last_slab = new_last
                            },
                            None => break new_slab.publish(),
                        }
                    }
                },
            }
        }
    }

    // SAFETY: Synchronization is externally managed.
    unsafe impl Sync for Slab {}

    /// Unpublished slab and storage, both owned until insertion succeeds.
    struct Candidate {
        /// Slab allocation containing initialized metadata.
        slab: Allocation,

        /// Backing allocation for the slab's storage.
        storage: Allocation,
    }

    impl Candidate {
        /// Return the unpublished slab's address without creating a static reference.
        fn pointer(&self) -> *mut Slab {
            let Self { slab, .. } = self;

            slab.pointer().cast::<Slab>().as_ptr()
        }

        /// Root both allocations after the slab has been published by the CAS.
        fn publish(self) -> &'static Slab {
            let Self { slab, storage } = self;
            storage.leak();
            let pointer = slab.leak().cast::<Slab>();

            // SAFETY: Both allocations are rooted and the slab has been initialized.
            unsafe { pointer.as_ref() }
        }
    }

    /// A type-erased analog to [`Allocation`] that encodes a position-dependent
    /// offset to an [`Storage`] instance.
    ///
    /// [`Allocation`]: crate::store::Allocation
    #[repr(transparent)]
    pub struct TypelessAllocation(usize);

    /// Address of the type-erased storage allocation.
    pub struct Storage(NonNull<TypelessAllocation>);

    impl Storage {
        /// Allocate a storage image and keep its owner unpublished.
        fn allocate<T>() -> (Self, Allocation)
        where
            T: Store,
        {
            #[repr(C, align(1))]
            struct Whole<T>
            where
                T: Store,
            {
                /// The type-erased allocation indicator.
                allocation: TypelessAllocation,

                /// The underlying storage container.
                container: MaybeUninit<Container<T>>,
            }

            let allocation = Allocation::new::<Whole<T>>().expect("allocate storage");
            let alloc_ptr = allocation.pointer();

            // SAFETY: The allocation has enough size and alignment for Whole<T>.
            let uninit_whole = unsafe { alloc_ptr.cast::<MaybeUninit<UnsafeCell<Whole<T>>>>().as_mut() };

            let scratch = Whole {
                allocation: TypelessAllocation(mem::offset_of!(Whole<T>, container)),
                // NOTE: This initializes the header in the container.
                container: MaybeUninit::zeroed(),
            };

            let target_whole = uninit_whole.write(UnsafeCell::new(scratch));

            let allocation_handle = NonNull::from(target_whole).cast::<TypelessAllocation>();

            (Self(allocation_handle), allocation)
        }
    }

    impl Storage {
        /// Return the start of the type-erased storage image.
        #[inline]
        pub const fn allocation(&self) -> NonNull<TypelessAllocation> {
            let &Self(allocation_handle) = self;

            allocation_handle
        }
    }

    #[cfg(test)]
    mod tests {
        use core::ptr;
        use std::{sync::Barrier, thread};

        use super::Slab;
        use crate::{arch::Container, domain::arbitrary::Arbitrary};

        #[test]
        fn domains_have_distinct_storage() {
            struct First;
            struct Second;

            let first = Container::<u64>::address_in::<Arbitrary<First>>();
            let second = Container::<u64>::address_in::<Arbitrary<Second>>();
            assert!(!ptr::eq(first.as_ptr(), second.as_ptr()));
            assert_eq!(first, Container::<u64>::address_in::<Arbitrary<First>>());
        }

        #[test]
        fn competing_candidates_release_the_loser() {
            struct Race;
            let barrier = Barrier::new(2);

            thread::scope(|scope| {
                for _ in 0..2 {
                    let barrier = &barrier;
                    scope.spawn(move || {
                        let last = match Slab::find::<Race>() {
                            Err(last) => last,
                            Ok(_) => unreachable!(),
                        };
                        let candidate = Slab::instance::<Race>();
                        barrier.wait();
                        match last.chain(candidate.pointer()) {
                            None => {
                                let _published = candidate.publish();
                            },
                            Some(winner) => {
                                assert!(winner.is::<Race>());
                                drop(candidate);
                            },
                        }
                    });
                }
            });

            assert!(Slab::find::<Race>().is_ok());
        }
    }
}
