use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    fs::{self, remove_file},
    io,
};

use clap::Subcommand;

use fack::prelude::Error;

use serde::Serialize;
use tera::Tera;

use crate::{
    invoke::InvokeContext,
    manifest::PlatformDesc,
    orchestrate::command::{Execute, Output, OutputOptions, OutputStructured},
    platform::PlatformSpec,
};

#[derive(Subcommand, Debug, Clone)]
pub enum PlatformTemplateCommand {
    /// Build the associated templates for the platform.
    Build {
        /// Whether to include the default build scopes in the expansion.
        #[arg(long, default_value_t = false)]
        no_default_scope: bool,

        /// The scopes to include for the build.
        #[arg(long, short)]
        scope: Vec<String>,
    },

    /// Clean all associated template output files.
    Clean,
}

impl Execute for PlatformTemplateCommand {
    type Data<'a> = (
        &'a PlatformSpec,
        &'a PlatformDesc,
        Option<(&'a PlatformDesc, PlatformSpec)>,
    );

    type Output<'a> = PlatformTemplateOutput;

    type Error = PlatformTemplateError;

    fn command<'a>(
        self,
        context: &'a InvokeContext,
        (spec, desc, base): Self::Data<'a>,
    ) -> Result<Self::Output<'a>, Self::Error> {
        match self {
            PlatformTemplateCommand::Build {
                no_default_scope,
                scope,
            } => {
                let parent_dir = context.root().join(
                    desc.path()
                        .parent()
                        .expect("cannot find parent for existing file"),
                );

                let mut tera_state = Tera::new(parent_dir.join("**/*.tera").as_str())
                    .map_err(PlatformTemplateError::TeraError)?;

                let mut tera_context = tera::Context::new();

                let mut include_scope = Vec::<String>::new();

                include_scope.extend(scope);

                if !no_default_scope {
                    include_scope.extend(spec.build().scopes().map(str::to_string));
                }

                let scope_tree = {
                    fn merge_value(a: &mut tera::Value, b: tera::Value) {
                        match (a, b) {
                            (tera::Value::Object(a_map), tera::Value::Object(b_map)) => {
                                for (k, v) in b_map {
                                    match a_map.get_mut(&k) {
                                        Some(a_val) => merge_value(a_val, v),
                                        None => _ = a_map.insert(k, v),
                                    }
                                }
                            }
                            (a_slot, b) => *a_slot = b,
                        }
                    }

                    let mut base_tree = if let Some((.., base_spec)) = &base {
                        base_spec.scope().to_owned()
                    } else {
                        BTreeMap::new()
                    };

                    let mut scope_map = spec.scope().to_owned();

                    for (k, v) in base_tree.iter_mut() {
                        if let Some(existing) = scope_map.remove(k) {
                            merge_value(v, existing);
                        }
                    }

                    for (k, v) in scope_map.into_iter() {
                        base_tree.insert(k, v);
                    }

                    base_tree
                };

                for (scope_name, scope_value) in include_scope
                    .iter()
                    .map(|scope_name| (scope_name, scope_tree.get(scope_name)))
                {
                    if let Some(scope_value) = scope_value {
                        tera_context.insert(scope_name, scope_value);
                    } else {
                        eprintln!("scope not found: {scope_name}");
                    }
                }

                // NOTE: We aren't rendering HTML, so disable auto-escape functionality.
                tera_state.autoescape_on(Vec::<&'static str>::new());

                let mut processed_template = BTreeMap::new();

                // NOTE: Standard functions provided to each template.
                {
                    /// A *Tera* function that returns a stored string,
                    /// irrespective of provided arguments.
                    struct StaticStringFn(String);

                    impl tera::Function for StaticStringFn {
                        fn call(
                            &self,
                            args: &HashMap<String, tera::Value>,
                        ) -> tera::Result<tera::Value> {
                            let Self(target_value) = self;

                            if args.len() > 0 {
                                return Err(tera::Error::msg("this takes no arguments"));
                            }

                            Ok(tera::Value::String(target_value.clone()))
                        }
                    }

                    // NOTE: The workspace base root path.
                    tera_state.register_function(
                        "workspace",
                        StaticStringFn(
                            context
                                .root()
                                .as_std_path()
                                .to_path_buf()
                                .to_string_lossy()
                                .to_string(),
                        ),
                    );

                    // NOTE: The platform base path.
                    tera_state.register_function(
                        "platform",
                        StaticStringFn(
                            parent_dir
                                .as_std_path()
                                .to_path_buf()
                                .to_string_lossy()
                                .to_string(),
                        ),
                    );
                }

                for template_name in tera_state.get_template_names() {
                    let mut filename = parent_dir.join(template_name);

                    // NOTE: This always has a `tera` extension, so remove it.
                    assert!(filename.set_extension(""));

                    fs::write(
                        filename.as_std_path(),
                        tera_state
                            .render(template_name, &mut tera_context)
                            .map_err(PlatformTemplateError::TeraError)?,
                    )
                    .map_err(PlatformTemplateError::Io)?;

                    processed_template.insert(template_name.to_string(), filename.into_string());
                }

                Ok(PlatformTemplateOutput::Build {
                    template: processed_template,
                })
            }
            PlatformTemplateCommand::Clean => {
                let parent_dir = context.root().join(
                    desc.path()
                        .parent()
                        .expect("cannot find parent for existing file"),
                );

                let tera_state = Tera::new(parent_dir.join("**/*.tera").as_str())
                    .map_err(PlatformTemplateError::TeraError)?;

                let mut cleaned_files = Vec::new();

                for template_name in tera_state.get_template_names() {
                    let mut path = parent_dir.join(template_name);

                    // NOTE: This always has a `tera` extension, so remove it.
                    assert!(path.set_extension(""));

                    let path_str = path.to_string();

                    if let Ok(..) = remove_file(path) {
                        cleaned_files.push(path_str);
                    }
                }

                Ok(PlatformTemplateOutput::Clean {
                    files: cleaned_files,
                })
            }
        }
    }
}

/// An error relevant to the `platform show` command.
#[derive(Debug, Error)]
pub enum PlatformTemplateError {
    #[error(transparent(0))]
    TeraError(tera::Error),

    #[error("input-output error: {_0}")]
    Io(io::Error),
}

/// The structured output of the `platform show` command.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlatformTemplateOutput {
    /// The output of the build subcommand.
    Build {
        /// The map of the processed templates to the generated ones.
        template: BTreeMap<String, String>,
    },

    /// The output of the clean subcommand.
    Clean {
        /// The list of files that were removed.
        files: Vec<String>,
    },
}

impl<'a> Output for PlatformTemplateOutput {
    fn output<W>(
        self,
        writer: &mut W,
        &OutputOptions {
            structured,
            verbose,
        }: &OutputOptions,
    ) -> fmt::Result
    where
        W: fmt::Write,
    {
        match structured {
            Some(structured_format) => {
                let ref value_tree =
                    serde_json::to_value(self).expect("cannot convert to value tree");

                let serialize_sink = match structured_format {
                    OutputStructured::Json => {
                        serde_json::to_string(value_tree).expect("could not serialize")
                    }
                };

                writer.write_str(serialize_sink.as_str())
            }
            None => match self {
                PlatformTemplateOutput::Build {
                    template: processed,
                } => {
                    if verbose {
                        for (template, output) in processed {
                            writeln!(writer, "built {template} -> {output}")?;
                        }
                    }

                    Ok(())
                }
                PlatformTemplateOutput::Clean {
                    files: cleaned_files,
                } => {
                    if verbose {
                        for file in cleaned_files {
                            writeln!(writer, "deleted `{file}`")?;
                        }
                    }

                    Ok(())
                }
            },
        }
    }
}
