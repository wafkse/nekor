use std::{fs, io, thread};

use cargo_metadata::{
    Metadata, MetadataCommand,
    camino::{Utf8Path, Utf8PathBuf},
};

use clap::Parser;

use fack::prelude::Error;
use serde::Deserialize;

use crate::{
    manifest::Manifest,
    orchestrate::{
        Orchestrate, OrchestrateVector, OrchestrationContext,
        command::{OrchestrateCommand, OrchestrateError, OutputOptions},
    },
};

/// The root-level metadata destined for the Nekor Build System.
///
/// This is metadata found in the `Cargo.toml` manifest of either the workspace
/// or the crate.
#[derive(Debug, Clone, Deserialize)]
pub struct RootMetadata {
    /// The path to the manifest for the Nekor Build System.
    manifest: Utf8PathBuf,
}

/// The invoke context used for individual orchestration.
#[derive(Debug, Clone)]
pub struct InvokeContext {
    /// The root directory used for path resolution.
    ///
    /// This is the directory where the root-level Cargo manifest is located.
    root: Utf8PathBuf,

    /// The root-level metadata for the Nekor Build System.
    metadata: RootMetadata,

    /// The manifest that was found.
    manifest: Manifest,

    /// The orchestration context containing all further orchestration
    /// subcommands.
    orchestrate_context: OrchestrationContext,

    /// The output options for this context.
    output: OutputOptions,
}

impl InvokeContext {
    /// Determine the root directory to use for path resolution.
    #[inline]
    pub fn root(&self) -> &Utf8Path {
        let Self { root, .. } = self;

        root.as_path()
    }

    /// Determine the root-level metadata provided.
    #[inline]
    pub fn metadata(&self) -> &RootMetadata {
        let Self { metadata, .. } = self;

        metadata
    }

    /// Determine the root-level metadata provided, in a mutable manner.
    #[inline]
    pub fn metadata_mut(&mut self) -> &mut RootMetadata {
        let Self { metadata, .. } = self;

        metadata
    }

    /// Determine the build manifest.
    #[inline]
    pub fn manifest(&self) -> &Manifest {
        let Self { manifest, .. } = self;

        manifest
    }

    /// Determine the build manifest, in a mutable manner.
    #[inline]
    pub fn manifest_mut(&mut self) -> &mut Manifest {
        let Self { manifest, .. } = self;

        manifest
    }

    /// Determine the orchestration context.
    #[inline]
    pub fn orchestrate(&self) -> &OrchestrationContext {
        let Self {
            orchestrate_context,
            ..
        } = self;

        orchestrate_context
    }

    /// Determine the orchestration context, in a mutable manner.
    #[inline]
    pub fn orchestrate_mut(&mut self) -> &mut OrchestrationContext {
        let Self {
            orchestrate_context,
            ..
        } = self;

        orchestrate_context
    }

    /// Determine the output options of this invocation context.
    #[inline]
    pub fn output(&self) -> &OutputOptions {
        let Self { output, .. } = self;

        output
    }
}

/// An individual build system for the Nekor Unikernel.
#[derive(Parser, Debug, Clone)]
pub struct Invoke {
    /// The global output options of the invocation.
    #[command(flatten)]
    output_options: OutputOptions,

    /// The orchestration command used for this invocation.
    #[clap(subcommand)]
    orchestrate_command: Option<OrchestrateCommand>,
}

impl Invoke {
    /// The entry point to an invocation.
    ///
    /// This takes a parsed [`Invocation`] top-level structure and commences the
    /// orchestration process.
    #[inline]
    pub fn run(self) -> Result<(), InvokeError> {
        let Self {
            output_options,
            orchestrate_command,
        } = self;

        let Metadata {
            workspace_root,
            workspace_metadata,
            ..
        } = MetadataCommand::new()
            .exec()
            .map_err(InvokeError::Metadata)?;

        let root_metadata: RootMetadata = serde_json::from_value(
            workspace_metadata
                .get("nekor")
                .ok_or_else(|| InvokeError::Manifest(InvokeManifestError::MetadataNotFound))?
                .to_owned(),
        )
        .map_err(InvokeError::JsonError)?;

        let manifest: Manifest = toml::from_slice(
            fs::read(workspace_root.join(root_metadata.manifest.clone()))
                .map_err(InvokeError::Io)?
                .as_slice(),
        )
        .map_err(InvokeError::TomlError)?;

        let mut context = InvokeContext {
            root: workspace_root,
            metadata: root_metadata,
            manifest,
            output: output_options,
            orchestrate_context: OrchestrationContext::new(),
        };

        if let Some(target_command) = orchestrate_command {
            Orchestrate::schedule(target_command, &mut context)
                .map_err(InvokeError::Orchestrate)?;
        }

        let vector_list = context
            .orchestrate_context
            .content_mut()
            .drain(..)
            .collect::<Vec<OrchestrateVector>>();

        thread::scope(|scope| -> Result<(), InvokeError> {
            for mut vector in vector_list {
                let mut handle_list = Vec::new();

                for command in vector.content_mut().drain(..) {
                    let thread_closure = || command.execute(&context);

                    handle_list.push(scope.spawn(thread_closure));
                }

                for handle in handle_list {
                    let _ = handle
                        .join()
                        .map_err(|_| OrchestrateError::Internal)
                        .map_err(InvokeError::Orchestrate)??;
                }
            }

            Ok(())
        })
    }
}

/// An invocation error.
#[derive(Debug, Error)]
pub enum InvokeError {
    /// An orchestration error occured.
    #[error(transparent(0))]
    Orchestrate(OrchestrateError),

    /// Cargo metadata could not be acquired.
    #[error(transparent(0))]
    Metadata(cargo_metadata::Error),

    /// Manifest-related errors, see each individual variant for further
    /// information.
    #[error(transparent(0))]
    Manifest(InvokeManifestError),

    /// Input-output errors. Forwarded verbatim.
    #[error(transparent(0))]
    Io(io::Error),

    /// JSON serialization or deserialization failed.
    #[error("json serialization error")]
    #[error(from)]
    #[error(source(0))]
    JsonError(serde_json::Error),

    /// TOML deserialization failed.
    #[error(transparent(0))]
    TomlError(toml::de::Error),
}

impl From<OrchestrateError> for InvokeError {
    fn from(value: OrchestrateError) -> Self {
        Self::Orchestrate(value)
    }
}

/// A sub-error intended for manifest-related errors.
#[derive(Debug, Error)]
pub enum InvokeManifestError {
    /// The manifest file referenced in the workspace metadata was not found.
    #[error("the manifest file was not found")]
    NotFound,

    /// The workspace-level (or otherwise crate-level) metadata entry for the
    /// build system was not present.
    #[error("the workspace metadata manifest entry is not present")]
    MetadataNotFound,
}
