//! Build command orchestration,

pub mod command;

use core::iter;

use fack::prelude::Error;

use crate::{invoke::InvokeContext, orchestrate::command::OrchestrateCommand};

/// A trait to represent a command that can further schedule downstream
/// orchestration commands.
pub trait Orchestrate {
    /// The associated error type with this orchestration.
    type Error: Error;

    /// Schedule the required downstream orchestration commands for this
    /// command.
    ///
    /// # Errors
    ///
    /// Returns an error when downstream scheduling fails.
    fn schedule(self, context: &mut InvokeContext) -> Result<(), Self::Error>;

    /// Execute the orchestration command.
    ///
    /// # Errors
    ///
    /// Returns an error when command execution fails.
    fn execute(self, context: &InvokeContext) -> Result<(), Self::Error>;
}

/// A orchestration context.
///
/// The `nekor-build` system self-invokes itself to satisfy requirements of a
/// top-level invocation.
///
/// A set of delegate orchestration commands are combined in their own vector so
/// that orchestration commands that can progress in parallel are indeed
/// performed in parallel. Orchestration commands that reside in distinct
/// vectors are effectively barriered and a posterior vector will not
/// commence unless anterior vectors all have been exhausted.
///
/// Optionally, both command vectors and individual orchestration commands can
/// have their own names, for clarity.
#[derive(Debug, Clone)]
pub struct OrchestrationContext(Vec<OrchestrateVector>);

impl OrchestrationContext {
    /// Creates a new [`OrchestrationContext`].
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self(Vec::<OrchestrateVector>::new())
    }
}

impl OrchestrationContext {
    /// Append a set concurrent commands to the current burst.
    ///
    /// This represents a set of commands that do not have any dependencies, and
    /// can be executed with other commands apart from themselves.
    #[inline]
    pub fn concurrent(&mut self, command: impl Iterator<Item = OrchestrateCommand>) {
        let &mut Self(ref mut vector_list) = self;

        let target_vector = if vector_list.is_empty() {
            vector_list.push_mut(OrchestrateVector::new())
        } else {
            let final_index = vector_list.len() - 1;

            &mut vector_list[final_index]
        };

        target_vector.content_mut().extend(command);
    }

    /// Identical to the [`OrchestrationContext::concurrent`] associated
    /// function, but operating on a singular command.
    #[inline]
    pub fn concurrent_one(&mut self, command: OrchestrateCommand) {
        self.concurrent(iter::once(command));
    }

    /// Append a set of isolated commands for future orchestration.
    ///
    /// This is for commands that have a prior dependency, and must not be
    /// executed before such dependencies are satisfied.
    #[inline]
    pub fn isolated(&mut self, command: impl Iterator<Item = OrchestrateCommand>) {
        let &mut Self(ref mut vector_list) = self;

        vector_list
            .push_mut(OrchestrateVector::new())
            .content_mut()
            .extend(command);
    }

    /// Identical to the [`OrchestrationContext::isolated`] associated function,
    /// but operating on a singular command.
    #[inline]
    pub fn isolated_one(&mut self, command: OrchestrateCommand) {
        self.isolated(iter::once(command));
    }
}

impl Default for OrchestrationContext {
    fn default() -> Self {
        Self::new()
    }
}

impl OrchestrationContext {
    /// Determine the orchestration vectors contained within this context.
    #[inline]
    #[must_use]
    pub const fn content(&self) -> &[OrchestrateVector] {
        let &Self(ref orchestrate_vec) = self;

        orchestrate_vec.as_slice()
    }

    /// Determine the orchestration vectors contained within this context, but
    /// in a mutable manner.
    #[inline]
    pub const fn content_mut(&mut self) -> &mut Vec<OrchestrateVector> {
        let &mut Self(ref mut orchestrate_vec) = self;

        orchestrate_vec
    }
}

/// An orchestration vector.
#[derive(Debug, Clone)]
pub struct OrchestrateVector {
    /// The list of [`Orchestrate`] commands contained within this vector.
    orchestrate_list: Vec<OrchestrateCommand>,
}

impl OrchestrateVector {
    /// Creates a new unnamed [`OrchestrateVector`].
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        let orchestrate_list = Vec::<OrchestrateCommand>::new();

        Self { orchestrate_list }
    }
}

impl Default for OrchestrateVector {
    fn default() -> Self {
        Self::new()
    }
}

impl OrchestrateVector {
    /// Determine the slice of [`Orchestrate`] commands that this vector
    /// manages.
    #[inline]
    #[must_use]
    pub const fn content(&self) -> &[OrchestrateCommand] {
        let &Self {
            ref orchestrate_list, ..
        } = self;

        orchestrate_list.as_slice()
    }

    /// Determine the slice of [`Orchestrate`] commands that this vector
    /// manages, in a mutable manner.
    #[inline]
    pub const fn content_mut(&mut self) -> &mut Vec<OrchestrateCommand> {
        let &mut Self {
            ref mut orchestrate_list,
            ..
        } = self;

        orchestrate_list
    }
}
