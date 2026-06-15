pub mod show;

pub mod template;

use std::{fmt, fs, io};

use cargo_metadata::camino::Utf8PathBuf;

use clap::{Parser, Subcommand};
use fack::prelude::Error;
use serde::Serialize;

use crate::{
    invoke::InvokeContext,
    manifest::PlatformDesc,
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
    platform::PlatformSpec,
};

// TODO: Make platform template family ofn subcommands: build

/// An orchestrate platform command.
#[derive(Parser, Debug, Clone)]
pub struct OrchestratePlatform {
    /// The name of the platform to orchestrate build for.
    ///
    /// This name must be recognized by the manifest.
    #[arg(long, short)]
    name: String,

    /// The subcommand of the platform command.
    #[clap(subcommand)]
    platform_command: PlatformCommand,
}

/// A platform-level command in the orchestrator.
#[derive(Subcommand, Debug, Clone)]
pub enum PlatformCommand {
    /// Show basic information and metadata about the platform
    #[clap(subcommand)]
    Show(PlatformShowCommand),

    /// Platform template management
    #[clap(subcommand)]
    Template(PlatformTemplateCommand),
}

/// An error relevant to a platform command.
#[derive(Debug, Error)]
pub enum OrchestratePlatformError {
    /// The platform was not found in the build manifest.
    #[error("the target platform was not found: {name}")]
    PlatformNotFound { name: String },

    /// The specified platform configuration file was unable to be read.
    #[error("failed to read platform configuration file: {_0}")]
    PlatformConfigFile(io::Error),

    /// Failed to deserialize the platform configuration file.
    #[error("failed to parse file `{path}`: {error}")]
    DeserializeConfigFile {
        path: Utf8PathBuf,
        error: toml::de::Error,
    },

    /// An error specific to the `platform show` command.
    #[error(transparent(0))]
    PlatformShowError(PlatformShowError),

    /// An error specific to the `platform template` command.
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

        let Some(desc) = context.manifest().platform().get(name.as_str()) else {
            return Err(OrchestratePlatformError::PlatformNotFound { name });
        };

        let spec_path = context.root().join(desc.path());

        let spec: PlatformSpec = toml::from_slice(
            fs::read(spec_path.clone())
                .map_err(OrchestratePlatformError::PlatformConfigFile)?
                .as_slice(),
        )
        .map_err(|error| OrchestratePlatformError::DeserializeConfigFile {
            path: spec_path,
            error,
        })?;

        let base = if let Some(base) = spec.platform().base() {
            let Some(base_desc) = context.manifest().platform().get(base) else {
                return Err(OrchestratePlatformError::PlatformNotFound {
                    name: base.to_string(),
                });
            };

            let base_path = context.root().join(base_desc.path());

            let base_spec: PlatformSpec = toml::from_slice(
                fs::read(base_path.clone())
                    .map_err(OrchestratePlatformError::PlatformConfigFile)?
                    .as_slice(),
            )
            .map_err(|error| OrchestratePlatformError::DeserializeConfigFile {
                path: base_path,
                error,
            })?;

            Some((base_desc, base_spec))
        } else {
            None
        };

        let target_output = platform_command.command(context, (&spec, desc, base))?;

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
    /// Output for the "platform show" subcommand.
    Show(PlatformShowOutput<'a>),

    /// Output for the "platform template" subcommand.
    Template(PlatformTemplateOutput),
}

impl<'a> Output for PlatformOutput<'a> {
    fn output<W>(self, writer: &mut W, options: &OutputOptions) -> fmt::Result
    where
        W: fmt::Write,
    {
        match self {
            PlatformOutput::Show(target_output) => target_output.output(writer, options),
            PlatformOutput::Template(target_output) => target_output.output(writer, options),
        }
    }
}

impl Execute for PlatformCommand {
    type Data<'a> = (
        &'a PlatformSpec,
        &'a PlatformDesc,
        Option<(&'a PlatformDesc, PlatformSpec)>,
    );

    type Output<'a> = PlatformOutput<'a>;

    type Error = OrchestratePlatformError;

    fn command<'a>(
        self,
        context: &'a InvokeContext,
        data: Self::Data<'a>,
    ) -> Result<Self::Output<'a>, Self::Error> {
        match self {
            PlatformCommand::Show(target_command) => target_command
                .command(context, data)
                .map(PlatformOutput::Show)
                .map_err(OrchestratePlatformError::PlatformShowError),
            PlatformCommand::Template(target_command) => target_command
                .command(context, data)
                .map(PlatformOutput::Template)
                .map_err(OrchestratePlatformError::PlatformTemplateError),
        }
    }
}
