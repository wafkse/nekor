pub mod platform;

use std::fmt;

use clap::{Parser, Subcommand, ValueEnum};

use fack::prelude::Error;
use serde::Serialize;

use crate::{
    invoke::InvokeContext,
    orchestrate::{
        Orchestrate,
        command::platform::{OrchestratePlatform, OrchestratePlatformError},
    },
};

/// The top-level build orchestration command.
#[derive(Subcommand, Debug, Clone)]
pub enum OrchestrateCommand {
    /// Platform-related orchestration and build commands.
    Platform(OrchestratePlatform),
}

impl Orchestrate for OrchestrateCommand {
    type Error = OrchestrateError;

    fn schedule(self, context: &mut InvokeContext) -> Result<(), Self::Error> {
        match self {
            Self::Platform(orchestrate_platform) => orchestrate_platform
                .schedule(context)
                .map_err(OrchestrateError::PlatformError),
        }
    }

    fn execute(self, context: &InvokeContext) -> Result<(), Self::Error> {
        match self {
            Self::Platform(orchestrate_platform) => orchestrate_platform
                .execute(context)
                .map_err(OrchestrateError::PlatformError),
        }
    }
}

/// An enumeration of all possible orchestration errors.
#[derive(Debug, Error)]
pub enum OrchestrateError {
    /// An error specific to the platform command.
    #[error(transparent(0))]
    PlatformError(OrchestratePlatformError),

    /// An internal error, of an unspecified kind.
    #[error("internal error")]
    Internal,
}

/// A trait for individual executable commands.
pub trait Execute {
    /// The associated data required for the execution.
    type Data<'a>;

    /// The output of the command.
    type Output<'a>: Output;

    /// The associated error type that can arise for command execution.
    type Error;

    /// Execute the command.
    ///
    /// # Errors
    ///
    /// Returns an error when the command cannot produce its output.
    fn command<'a>(
        self,
        context: &'a InvokeContext,
        data: Self::Data<'a>,
    ) -> Result<Self::Output<'a>, Self::Error>;
}

/// A hint to tell the program in which output format should
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum OutputStructured {
    /// Output in JSON format.
    #[default]
    Json,
}

/// The output options of the overall build system.
#[derive(Debug, Clone, Copy, Parser)]
pub struct OutputOptions {
    /// Whether the output should be structured.
    #[arg(
        long,
        num_args = 0..=1,
        require_equals = false,
        default_missing_value = "json",
        value_enum
    )]
    pub structured: Option<OutputStructured>,

    /// Whether the human-readable input should include all possible
    /// information.
    #[arg(long, default_value_t = true)]
    pub verbose: bool,
}

/// A trait for generalised command outputs.
pub trait Output: Serialize {
    /// Output the downstream formatted output as per the provided options.
    ///
    /// # Errors
    ///
    /// Returns an error when writing formatted output fails.
    fn output<W>(self, writer: &mut W, options: &OutputOptions) -> fmt::Result
    where
        W: fmt::Write;
}
