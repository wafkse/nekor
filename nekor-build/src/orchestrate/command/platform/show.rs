use std::{collections::BTreeMap, fmt};

use clap::Subcommand;
use fack::prelude::Error;

use serde::Serialize;

use crate::{
    invoke::InvokeContext,
    manifest::PlatformDesc,
    orchestrate::command::{Execute, Output, OutputOptions, OutputStructured},
    platform::PlatformSpec,
};

/// An error relevant to the `platform show` command.
#[derive(Debug, Error)]
pub enum PlatformShowError {
    /// The provided scope was not found.
    #[error("the scope `{scope}` does not exist")]
    ScopeNotFound { scope: String },

    /// Failed to serialize TOML structured output.
    #[error("failed to serialize toml: {_0}")]
    Toml(toml::ser::Error),

    /// Failed to serialize TOML structured output.
    #[error("failed to serialize json: {_0}")]
    Json(serde_json::Error),
}

/// The structured output of the `platform show` command.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlatformShowOutput<'a> {
    /// A metadata subcommand output.
    Metadata {
        /// The metadata of the platform.
        #[serde(flatten)]
        metadata: &'a PlatformDesc,
    },

    /// A show-all subcommand output.
    All {
        /// The base platform specification.
        base: Option<PlatformSpec>,

        /// The complete specification for the platform at hand.
        #[serde(flatten)]
        spec: &'a PlatformSpec,

        #[serde(flatten)]
        /// The metadata for the platform.
        metadata: &'a PlatformDesc,
    },

    /// A scope subcommand output for a target scope.
    Scope {
        /// The name of the scope.
        name: String,

        /// The tree of data defined for the scope.
        scope: &'a serde_json::Value,
    },

    /// A scope subcommand output for all available scopes.
    Scopes {
        /// The tree of data defined for all scopes.
        #[serde(flatten)]
        scopes: &'a BTreeMap<String, serde_json::Value>,
    },
}

impl<'a> Output for PlatformShowOutput<'a> {
    fn output<W>(
        self,
        writer: &mut W,
        OutputOptions {
            structured,
            verbose,
        }: &OutputOptions,
    ) -> fmt::Result
    where
        W: fmt::Write,
    {
        let _ = verbose;

        match structured {
            Some(target_format @ OutputStructured::Json) => {
                let ref value_tree =
                    serde_json::to_value(self).expect("cannot convert to value tree");

                let serialize_sink = match target_format {
                    OutputStructured::Json => {
                        serde_json::to_string(value_tree).expect("could not serialize")
                    }
                };

                writer.write_str(serialize_sink.as_str())
            }
            // TODO(deduplicate): Deal with this code duplication later.
            None => match self {
                PlatformShowOutput::Metadata { metadata } => {
                    writeln!(writer, "Name: {}", metadata.name())?;

                    writeln!(
                        writer,
                        "Description: {}",
                        metadata.description().unwrap_or("(no description)")
                    )?;

                    writeln!(writer, "Path: {}", metadata.path())
                }
                PlatformShowOutput::All {
                    base,
                    spec,
                    metadata,
                } => {
                    writeln!(writer, "Name: {}", metadata.name())?;

                    writeln!(
                        writer,
                        "Description: {}",
                        metadata.description().unwrap_or("(no description)")
                    )?;

                    writeln!(writer, "Path: {}", metadata.path())?;

                    writeln!(
                        writer,
                        "Inherits From: {}",
                        spec.platform().base().unwrap_or("(no platform)")
                    )?;

                    write!(writer, "Default Build Scopes: ",)?;
                    for scope in spec.build().scopes() {
                        write!(writer, "{scope} ")?;
                    }
                    writer.write_char('\n')?;

                    if let Some(base) = base {
                        for (scope, values) in base.scope() {
                            writeln!(writer, "Inherited Scope `{scope}`:")?;

                            let value_str = serde_json::to_string_pretty(values)
                                .expect("failed to serialize")
                                .lines()
                                .map(|line| {
                                    let mut s = String::from('\t');

                                    s.push_str(line);

                                    s
                                })
                                .collect::<String>();

                            writeln!(writer, "{value_str}")?;
                        }
                    }

                    for (scope, values) in spec.scope() {
                        writeln!(writer, "Scope `{scope}`:")?;

                        let value_str = serde_json::to_string_pretty(values)
                            .expect("failed to serialize")
                            .lines()
                            .map(|line| {
                                let mut s = String::from('\t');

                                s.push_str(line);

                                s
                            })
                            .collect::<String>();

                        writeln!(writer, "{value_str}")?;
                    }

                    Ok(())
                }
                PlatformShowOutput::Scope { name, scope } => {
                    writeln!(writer, "Scope `{name}`:")?;

                    let value_str = serde_json::to_string_pretty(scope)
                        .expect("failed to serialize")
                        .lines()
                        .map(|line| {
                            let mut s = String::from('\t');

                            s.push_str(line);

                            s
                        })
                        .collect::<String>();

                    writeln!(writer, "{value_str}")
                }
                PlatformShowOutput::Scopes { scopes } => {
                    for (scope, values) in scopes {
                        writeln!(writer, "Scope `{scope}`:")?;

                        let value_str = serde_json::to_string_pretty(values)
                            .expect("failed to serialize")
                            .lines()
                            .map(|line| {
                                let mut s = String::from('\t');

                                s.push_str(line);

                                s
                            })
                            .collect::<String>();

                        writeln!(writer, "{value_str}")?;
                    }

                    Ok(())
                }
            },
        }
    }
}

/// A "platform show" command.
#[derive(Subcommand, Debug, Clone, Default)]
pub enum PlatformShowCommand {
    /// Show all relevant information for the platform.
    #[default]
    All,

    /// Show platform metadata.
    Metadata,

    /// Show available platform scopes.
    Scope {
        /// The name of the platform to optionally target.
        #[arg(short, long)]
        name: Option<String>,
    },
}

impl Execute for PlatformShowCommand {
    type Data<'a> = (
        &'a PlatformSpec,
        &'a PlatformDesc,
        Option<(&'a PlatformDesc, PlatformSpec)>,
    );

    type Output<'a> = PlatformShowOutput<'a>;

    type Error = PlatformShowError;

    fn command<'a>(
        self,
        _: &'a InvokeContext,
        (spec, metadata, base_tuple): Self::Data<'a>,
    ) -> Result<Self::Output<'a>, Self::Error> {
        match self {
            PlatformShowCommand::All => Ok(PlatformShowOutput::All {
                base: base_tuple.map(|(.., base_spec)| base_spec),
                spec,
                metadata,
            }),
            PlatformShowCommand::Metadata => Ok(PlatformShowOutput::Metadata { metadata }),

            PlatformShowCommand::Scope { name: Some(name) } => {
                if let Some(scope) = spec.scope().get(name.as_str()) {
                    Ok(PlatformShowOutput::Scope { name, scope })
                } else {
                    Err(PlatformShowError::ScopeNotFound { scope: name })
                }
            }
            PlatformShowCommand::Scope { name: None } => {
                let scopes = spec.scope();

                Ok(PlatformShowOutput::Scopes { scopes })
            }
        }
    }
}
