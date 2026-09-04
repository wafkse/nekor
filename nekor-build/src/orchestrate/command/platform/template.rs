use alloc::collections::BTreeMap;
use core::fmt;
use std::{fs, io, path::StripPrefixError};

use cargo_metadata::camino::{Utf8Path, Utf8PathBuf};
use clap::Subcommand;
use fack::prelude::Error;
use minijinja::{Environment, UndefinedBehavior};
use serde::Serialize;

use crate::{
    invoke::InvokeContext,
    manifest::PlatformDesc,
    orchestrate::command::{Execute, Output, OutputError, OutputOptions, OutputStructured},
    platform::{Platform, PlatformFileError},
};

/// A platform template command.
#[derive(Subcommand, Debug, Clone)]
pub enum PlatformTemplateCommand {
    /// Build the templates associated with the platform.
    Build,

    /// Clean generated template output files.
    Clean,
}

impl Execute for PlatformTemplateCommand {
    type Data<'a> = (&'a Platform, &'a PlatformDesc);
    type Error = PlatformTemplateError;
    type Output<'a> = PlatformTemplateOutput;

    fn command<'a>(
        self,
        context: &'a InvokeContext,
        (platform, _): Self::Data<'a>,
    ) -> Result<Self::Output<'a>, Self::Error> {
        match self {
            Self::Build => Self::build(context, platform),
            Self::Clean => Self::clean(platform),
        }
    }
}

impl PlatformTemplateCommand {
    /// Render every `MiniJinja` template belonging to the selected platform.
    fn build(context: &InvokeContext, platform: &Platform) -> Result<PlatformTemplateOutput, PlatformTemplateError> {
        let mut environment = Environment::new();
        let workspace = context.root().to_string();
        let platform_root = platform.root().to_string();

        environment.set_undefined_behavior(UndefinedBehavior::Strict);
        environment.set_keep_trailing_newline(true);
        environment.add_function("workspace", move || workspace.clone());
        environment.add_function("platform", move || platform_root.clone());

        let sources = platform.template_files()?;

        for path in &sources {
            let name = Self::template_name(platform.root(), path.as_path())?;
            let source = fs::read_to_string(path.as_std_path()).map_err(|error| PlatformTemplateError::Io {
                path: path.clone(),
                error,
            })?;

            environment.add_template_owned(name, source)?;
        }

        let mut processed = BTreeMap::new();

        for path in sources {
            let name = Self::template_name(platform.root(), path.as_path())?;
            let template = environment.get_template(name.as_str())?;
            let rendered = template.render(platform.document())?;
            let output = Self::output_path(path.as_path());

            fs::write(output.as_std_path(), rendered).map_err(|error| PlatformTemplateError::Io {
                path: output.clone(),
                error,
            })?;

            _ = processed.insert(name, output.to_string());
        }

        Ok(PlatformTemplateOutput::Build { template: processed })
    }

    /// Remove generated outputs corresponding to platform templates.
    ///
    /// # Errors
    ///
    /// Returns an error when template discovery fails. Individual missing
    /// generated files are ignored so cleaning remains idempotent.
    fn clean(platform: &Platform) -> Result<PlatformTemplateOutput, PlatformTemplateError> {
        let sources = platform.template_files()?;
        let mut files = Vec::new();

        for source in sources {
            let output = Self::output_path(source.as_path());

            if fs::remove_file(output.as_std_path()).is_ok() {
                files.push(output.to_string());
            }
        }

        Ok(PlatformTemplateOutput::Clean { files })
    }

    /// Determine the `MiniJinja` template name relative to a platform root.
    fn template_name(root: &Utf8Path, path: &Utf8Path) -> Result<String, PlatformTemplateError> {
        path.strip_prefix(root)
            .map(Utf8Path::to_string)
            .map_err(|error| PlatformTemplateError::TemplatePath {
                root: root.to_path_buf(),
                path: path.to_path_buf(),
                error,
            })
    }

    /// Determine the generated output path for a template source.
    fn output_path(source: &Utf8Path) -> Utf8PathBuf {
        let mut output = source.to_path_buf();
        _ = output.set_extension("");
        output
    }
}

/// An error relevant to the platform template command.
#[derive(Debug, Error)]
pub enum PlatformTemplateError {
    /// Platform template discovery failed.
    #[error(transparent(0))]
    File(PlatformFileError),

    /// `MiniJinja` failed to parse or render a template.
    #[error(transparent(0))]
    MiniJinja(minijinja::Error),

    /// A template path cannot be made relative to the platform root.
    #[error("template `{path}` is outside platform root `{root}` with {error}")]
    TemplatePath {
        /// The platform root.
        root: Utf8PathBuf,

        /// The template path.
        path: Utf8PathBuf,

        /// The path prefix failure.
        error: StripPrefixError,
    },
    /// A platform template file operation failed.
    #[error("template file operation failed for `{path}` with {error}")]
    Io {
        /// The template or generated output path.
        path: Utf8PathBuf,

        /// The underlying filesystem failure.
        error: io::Error,
    },
}

/// The structured output of the platform template command.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlatformTemplateOutput {
    /// Output from building platform templates.
    Build {
        /// Template names mapped to generated output paths.
        template: BTreeMap<String, String>,
    },

    /// Output from cleaning generated platform files.
    Clean {
        /// Generated files that were removed.
        files: Vec<String>,
    },
}

impl Output for PlatformTemplateOutput {
    fn output<W>(
        self,
        writer: &mut W,
        &OutputOptions { structured, verbose }: &OutputOptions,
    ) -> Result<(), OutputError>
    where
        W: fmt::Write,
    {
        match structured {
            Some(OutputStructured::Json) => {
                let value = serde_json::to_string(&self)?;
                writer.write_str(value.as_str())?;

                Ok(())
            },
            None => match self {
                Self::Build { template } => {
                    if verbose {
                        for (source, output) in template {
                            writeln!(writer, "built {source} -> {output}")?;
                        }
                    }

                    Ok(())
                },
                Self::Clean { files } => {
                    if verbose {
                        for file in files {
                            writeln!(writer, "deleted `{file}`")?;
                        }
                    }

                    Ok(())
                },
            },
        }
    }
}

impl From<PlatformFileError> for PlatformTemplateError {
    fn from(error: PlatformFileError) -> Self {
        Self::File(error)
    }
}

impl From<minijinja::Error> for PlatformTemplateError {
    fn from(error: minijinja::Error) -> Self {
        Self::MiniJinja(error)
    }
}
