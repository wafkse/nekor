//! Transactions that defer architectural publication until their effects are prepared.
//!
//! A [`Batch`] supplies provisional state for peripheral operations and [`Stage`](super::Stage)
//! handlers. Once effects have been staged, [`Prepare::prepare`] consumes the transaction and
//! establishes the requirements for publication, returning a value that implements [`Commit`].
//!
//! Because a transaction may fail or be abandoned before publication, its implementation must
//! preserve authoritative state throughout operations, staging, and preparation. This also
//! applies when a prepared value is dropped. Only [`Commit::commit`] publishes the changes,
//! with isolation and cleanup enforced by the concrete implementation.

use core::error::Error;

use nekor_virt_model::prelude::Arch;

/// A source of architectural transactions for a logical processor.
///
/// The implementation supplies the processor association and provisional state needed by
/// peripheral operations. Its transaction type implements [`Stage`](super::Stage) for each
/// supported effect, so operations on several peripherals can contribute to the same batch.
pub trait Batch<A: Arch> {
    /// The transaction context for this [`Batch`].
    ///
    /// This may borrow state and resources from the batch source for `'a`.
    type Transaction<'a>: Prepare
    where
        Self: 'a;

    /// The associated error that can occur during [`Batch::begin`].
    type Error: Error;

    /// Attempt to begin the associated architectural transaction.
    fn begin(&mut self) -> Result<Self::Transaction<'_>, Self::Error>;
}

/// Validates a transaction and acquires the resources needed to publish it.
///
/// Implementations must resolve all fallible publication requirements before returning a
/// [`Prepared`](Self::Prepared) value. In particular, a failed staging operation must block
/// preparation until explicit recovery restores the transaction's consistency.
///
/// Successful preparation transfers responsibility for provisional changes to the returned
/// value. If preparation fails, authoritative state must remain unchanged, while the
/// implementation determines how provisional resources are recovered or released.
pub trait Prepare: Sized {
    /// An error indicating that the transaction could not be prepared for publication.
    ///
    /// Preparation failure leaves authoritative state unchanged, although the concrete
    /// implementation defines how provisional resources are recovered or released.
    type Error: Error;

    /// A transaction that owns the provisional changes and resources required for infallible
    /// publication.
    ///
    /// Producing this value establishes that all fallible publication prerequisites have been
    /// satisfied.
    type Prepared: Commit;

    /// Consumes the transaction and acquires the resources required to publish its provisional
    /// changes.
    ///
    /// Returns an error if any publication prerequisite cannot be satisfied.
    /// A failed staging operation must prevent successful preparation until explicit recovery
    /// restores transaction consistency.
    ///
    /// On failure, authoritative state remains unchanged.
    ///
    /// On success, responsibility for provisional changes transfers to the returned prepared
    /// transaction.
    fn prepare(self) -> Result<Self::Prepared, Self::Error>;
}

/// Publishes a transaction whose fallible prerequisites have been satisfied.
///
/// Because [`Prepare::prepare`] must resolve those prerequisites, implementations must complete
/// publication using prepared resources without further fallible operations. The concrete
/// implementation defines the participating architectural state and its publication visibility,
/// returning a receipt only after publication completes.
pub trait Commit: Sized {
    /// Evidence that publication has completed for the participating architectural state.
    ///
    /// The concrete implementation defines the information carried by the receipt and the
    /// visibility guarantees established before it is returned.
    type Receipt;

    /// Consumes the prepared transaction and publishes its provisional changes using the resources
    /// acquired during preparation.
    ///
    /// Publication must not require further fallible operations. Returns a receipt only after
    /// publication completes according to the implementation’s visibility guarantees.
    fn commit(self) -> Self::Receipt;
}
