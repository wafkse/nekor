//! Typed synchronous work for Isolate invocations.

/// A synchronous computation invoked by an Isolate.
pub trait Job {
    /// The input type borrowed from the caller while the job is ran.
    type Input;

    /// The terminal value produced by [`run`].
    ///
    /// [`run`]: Self::run
    type Output;

    /// Execute the job's synchronous body.
    fn run(input: &Self::Input) -> Self::Output;
}
