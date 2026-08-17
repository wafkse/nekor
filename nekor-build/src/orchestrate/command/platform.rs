pub mod show;
pub mod template;

use std::fmt;

use clap::{Parser, Subcommand};
use fack::prelude::Error;
use serde::Serialize;

use crate::{
    invoke::InvokeContext,
    manifest::{PlatformDesc, PlatformName},
    orchestrate::{
        Orchestrate,
        command::{
            Execute, OrchestrateCommand, Output, OutputOptions,
            platform::{
                show::{PlatformShowCommand, PlatformShowError, PlatformShowOutput},
                template::{
                    PlatformTemplateCommand, PlatformTemplateError, PlatformTemplateOutput,
                },
            },
        },
    },
    platform::{Platform, PlatformResolveError},
};

/// An orchestrate platform command.
#[derive(Parser, Debug, Clone)]
pub struct OrchestratePlatform {
    /// The name of the platform to orchestrate build for.
    #[arg(long, short)]
    name: PlatformName,

    /// The platform-level operation to perform.
    #[clap(subcommand)]
    platform_command: PlatformCommand,
}

/// A platform-level command in the orchestrator.
#[derive(Subcommand, Debug, Clone)]
pub enum PlatformCommand {
    /// Show platform information.
    #[clap(subcommand)]
    Show(PlatformShowCommand),
    /// Manage platform templates.
    #[clap(subcommand)]
    Template(PlatformTemplateCommand),
}

/// An error relevant to a platform command.
#[derive(Debug, Error)]
pub enum OrchestratePlatformError {
    /// The selected platform was not found in the build manifest.
    #[error("the target platform was not found `{name}`")]
    PlatformNotFound {
        /// The missing selected platform name.
        name: PlatformName,
    },
    /// Platform resolution failed.
    #[error(transparent(0))]
    PlatformResolve(PlatformResolveError),
    /// An error specific to the platform show command.
    #[error(transparent(0))]
    PlatformShowError(PlatformShowError),
    /// An error specific to the platform template command.
    #[error(transparent(0))]
    PlatformTemplateError(PlatformTemplateError),
}

impl Orchestrate for OrchestratePlatform {
    type Error = OrchestratePlatformError;

    fn schedule(self, context: &mut InvokeContext) -> Result<(), Self::Error> {
        context
            .orchestrate_mut()
            .concurrent_one(OrchestrateCommand::Platform(self));

        Ok(())
    }

    fn execute(self, context: &InvokeContext) -> Result<(), Self::Error> {
        let Self {
            name,
            platform_command,
        } = self;
        let database = context.manifest().platform();
        let Some(desc) = database.get(name.as_str()) else {
            return Err(OrchestratePlatformError::PlatformNotFound { name });
        };
        let platform = Platform::resolve(context.root(), database, desc)
            .map_err(OrchestratePlatformError::PlatformResolve)?;
        let target_output = platform_command.command(context, (&platform, desc))?;
        let mut target_buffer = String::new();

        PlatformOutput::output(target_output, &mut target_buffer, context.output())
            .expect("failed to format output");
        println!("{}", target_buffer);

        Ok(())
    }
}

/// The output of a platform command.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlatformOutput<'a> {
    /// Output for the platform show subcommand.
    Show(PlatformShowOutput<'a>),
    /// Output for the platform template subcommand.
    Template(PlatformTemplateOutput),
}

impl Output for PlatformOutput<'_> {
    fn output<W>(self, writer: &mut W, options: &OutputOptions) -> fmt::Result
    where
        W: fmt::Write,
    {
        match self {
            Self::Show(target_output) => target_output.output(writer, options),
            Self::Template(target_output) => target_output.output(writer, options),
        }
    }
}

impl Execute for PlatformCommand {
    type Data<'a> = (&'a Platform, &'a PlatformDesc);
    type Output<'a> = PlatformOutput<'a>;
    type Error = OrchestratePlatformError;

    fn command<'a>(
        self,
        context: &'a InvokeContext,
        data: Self::Data<'a>,
    ) -> Result<Self::Output<'a>, Self::Error> {
        match self {
            Self::Show(target_command) => target_command
                .command(context, data)
                .map(PlatformOutput::Show)
                .map_err(OrchestratePlatformError::PlatformShowError),
            Self::Template(target_command) => target_command
                .command(context, data)
                .map(PlatformOutput::Template)
                .map_err(OrchestratePlatformError::PlatformTemplateError),
        }
    }
}
