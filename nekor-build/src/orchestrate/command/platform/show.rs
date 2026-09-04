use core::fmt;

use clap::Subcommand;
use fack::prelude::Error;
use serde::Serialize;

use crate::{
    invoke::InvokeContext,
    manifest::PlatformDesc,
    orchestrate::command::{Execute, Output, OutputError, OutputOptions, OutputStructured},
    platform::Platform,
};

/// An error relevant to the platform show command.
#[derive(Debug, Error)]
#[error("platform show command failed")]
pub struct PlatformShowError;

/// The structured output of the platform show command.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlatformShowOutput<'a> {
    /// Platform metadata.
    Metadata {
        /// The registered platform metadata.
        #[serde(flatten)]
        metadata: &'a PlatformDesc,
    },
    /// The resolved platform data and metadata.
    All {
        /// The resolved platform data.
        platform: &'a Platform,
        /// The registered platform metadata.
        #[serde(flatten)]
        metadata: &'a PlatformDesc,
    },
    /// The resolved platform data.
    Resolved {
        /// The resolved platform data.
        platform: &'a Platform,
    },
}

impl Output for PlatformShowOutput<'_> {
    fn output<W>(self, writer: &mut W, &OutputOptions { structured, .. }: &OutputOptions) -> Result<(), OutputError>
    where
        W: fmt::Write,
    {
        match structured {
            Some(OutputStructured::Json) => {
                let value = serde_json::to_string(&self)?;
                writer.write_str(value.as_str())?;

                Ok(())
            },
            None => self.output_human(writer),
        }
    }
}

impl PlatformShowOutput<'_> {
    /// Write the human-readable representation of the platform output.
    fn output_human<W>(self, writer: &mut W) -> Result<(), OutputError>
    where
        W: fmt::Write,
    {
        match self {
            Self::Metadata { metadata } => Self::write_metadata(writer, metadata),
            Self::All { platform, metadata } => {
                Self::write_metadata(writer, metadata)?;
                Self::write_platform(writer, platform)
            },
            Self::Resolved { platform } => Self::write_platform(writer, platform),
        }
    }

    /// Write platform registration metadata.
    fn write_metadata<W>(writer: &mut W, metadata: &PlatformDesc) -> Result<(), OutputError>
    where
        W: fmt::Write,
    {
        writeln!(writer, "Name {}", metadata.name())?;
        writeln!(
            writer,
            "Description {}",
            metadata.description().unwrap_or("(no description)")
        )?;
        writeln!(writer, "Path {}", metadata.path())?;
        writeln!(
            writer,
            "Base {}",
            metadata.base().map_or("(no platform)", |base| base.as_str())
        )?;

        Ok(())
    }

    /// Write the resolved platform data.
    fn write_platform<W>(writer: &mut W, platform: &Platform) -> Result<(), OutputError>
    where
        W: fmt::Write,
    {
        let value = serde_json::to_string_pretty(platform)?;
        writeln!(writer, "{value}")?;

        Ok(())
    }
}

/// A platform show command.
#[derive(Subcommand, Debug, Clone, Default)]
pub enum PlatformShowCommand {
    /// Show all platform information.
    #[default]
    All,
    /// Show platform metadata.
    Metadata,
    /// Show resolved platform data.
    Resolved,
}

impl Execute for PlatformShowCommand {
    type Data<'a> = (&'a Platform, &'a PlatformDesc);
    type Error = PlatformShowError;
    type Output<'a> = PlatformShowOutput<'a>;

    fn command<'a>(
        self,
        _: &'a InvokeContext,
        (platform, metadata): Self::Data<'a>,
    ) -> Result<Self::Output<'a>, Self::Error> {
        Ok(match self {
            Self::All => PlatformShowOutput::All { platform, metadata },
            Self::Metadata => PlatformShowOutput::Metadata { metadata },
            Self::Resolved => PlatformShowOutput::Resolved { platform },
        })
    }
}
