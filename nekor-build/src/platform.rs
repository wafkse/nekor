//! Platform loading and resolution support.

use std::{collections::BTreeSet, fs, io, path::PathBuf};

use cargo_metadata::camino::{Utf8Path, Utf8PathBuf};
use fack::prelude::Error;
use nekor_build_kdl_config::{
    document::Document,
    error::LoadError,
    merge::{Merge, MergeError},
    overlay::{Overlay, OverlayError},
    source::Source,
};
use serde::Serialize;
use walkdir::WalkDir;

use crate::manifest::{PlatformDatabase, PlatformDesc, PlatformName};

/// A platform data layer or fully resolved platform.
#[derive(Debug, Clone)]
pub struct Platform {
    /// The root directory of the most-derived platform layer.
    root: Utf8PathBuf,
    /// The typed data document resolved for the platform.
    document: Document,
}

/// An error produced while discovering files below a platform root.
#[derive(Debug, Error)]
pub enum PlatformFileError {
    /// Walking a platform directory failed.
    #[error("failed to walk platform directory `{root}` with {error}")]
    Walk {
        /// The root being traversed.
        root: Utf8PathBuf,
        /// The directory traversal failure.
        error: walkdir::Error,
    },
    /// A discovered platform path is not valid UTF-8.
    #[error("platform path is not valid UTF-8 `{path:?}`")]
    NonUtf8Path {
        /// The path rejected by the UTF-8 path boundary.
        path: PathBuf,
    },
}

/// An error produced while loading one platform layer.
#[derive(Debug, Error)]
pub enum PlatformLoadError {
    /// Platform source discovery failed.
    #[error(transparent(0))]
    File(PlatformFileError),
    /// A platform source could not be read.
    #[error("failed to read platform source `{path}` with {error}")]
    Read {
        /// The source path that failed to read.
        path: Utf8PathBuf,
        /// The filesystem failure.
        error: io::Error,
    },
    /// A platform source is not valid typed KDL configuration.
    #[error("failed to load platform source `{path}` with {error}")]
    Load {
        /// The KDL source path.
        path: Utf8PathBuf,
        /// The typed KDL loading failure.
        error: Box<LoadError>,
    },
    /// Same-precedence platform fragments conflict.
    #[error(transparent(0))]
    Merge(Box<MergeError>),
}

/// An error produced while resolving a platform inheritance chain.
#[derive(Debug, Error)]
pub enum PlatformResolveError {
    /// An inherited platform was not present in the manifest database.
    #[error("base platform `{name}` was not found")]
    PlatformNotFound {
        /// The missing platform name.
        name: PlatformName,
    },
    /// Platform inheritance contains a cycle.
    #[error("platform inheritance contains a cycle at `{name}`")]
    InheritanceCycle {
        /// The platform that closes the cycle.
        name: PlatformName,
    },
    /// A platform layer failed to load.
    #[error("failed to load platform layer `{name}` with {error}")]
    Load {
        /// The platform layer name.
        name: PlatformName,
        /// The platform loading failure.
        error: Box<PlatformLoadError>,
    },
    /// A derived platform layer violates inherited configuration invariants.
    #[error("failed to overlay platform layer `{name}` with {error}")]
    Overlay {
        /// The derived platform layer name.
        name: PlatformName,
        /// The typed overlay failure.
        error: Box<OverlayError>,
    },
}

impl Platform {
    /// Resolve one platform and its complete base chain.
    ///
    /// # Errors
    ///
    /// Returns an error for missing bases, inheritance cycles, invalid layers,
    /// or typed overlay failures.
    pub fn resolve(
        workspace: &Utf8Path,
        database: &PlatformDatabase,
        target: &PlatformDesc,
    ) -> Result<Self, PlatformResolveError> {
        let mut chain = Vec::new();
        let mut visited = BTreeSet::new();
        let mut current = target;

        loop {
            if !visited.insert(current.name().clone()) {
                return Err(PlatformResolveError::InheritanceCycle {
                    name: current.name().clone(),
                });
            }
            chain.push(current);

            let Some(base) = current.base() else {
                break;
            };
            let Some(base_desc) = database.get(base) else {
                return Err(PlatformResolveError::PlatformNotFound { name: base.clone() });
            };
            current = base_desc;
        }

        let root = current;
        let mut resolved = Self::load(&workspace.join(root.path())).map_err(|error| {
            PlatformResolveError::Load {
                name: root.name().clone(),
                error: Box::new(error),
            }
        })?;

        for desc in chain.into_iter().rev().skip(1) {
            let layer = Self::load(&workspace.join(desc.path())).map_err(|error| {
                PlatformResolveError::Load {
                    name: desc.name().clone(),
                    error: Box::new(error),
                }
            })?;
            resolved = resolved
                .overlay(layer)
                .map_err(|error| PlatformResolveError::Overlay {
                    name: desc.name().clone(),
                    error: Box::new(error),
                })?;
        }

        Ok(resolved)
    }

    /// Load one platform layer from all KDL files below its data directory.
    ///
    /// # Errors
    ///
    /// Returns an error when file discovery, parsing, or strict merging fails.
    pub fn load(root: &Utf8Path) -> Result<Self, PlatformLoadError> {
        let mut document = Document::default();

        for path in Self::files(&root.join("data"), "kdl").map_err(PlatformLoadError::File)? {
            let input = fs::read_to_string(path.as_std_path()).map_err(|error| {
                PlatformLoadError::Read {
                    path: path.clone(),
                    error,
                }
            })?;
            let source = Source::file(path.clone());
            let fragment = Document::parse(input.as_str(), &source).map_err(|error| {
                PlatformLoadError::Load {
                    path,
                    error: Box::new(error),
                }
            })?;
            document = document
                .merge(fragment)
                .map_err(|error| PlatformLoadError::Merge(Box::new(error)))?;
        }

        Ok(Self {
            root: root.to_path_buf(),
            document,
        })
    }

    /// Determine the root directory of the most-derived platform layer.
    #[inline]
    #[must_use]
    pub fn root(&self) -> &Utf8Path {
        let Self { root, .. } = self;

        root.as_path()
    }

    /// Determine the resolved typed platform data document.
    #[inline]
    #[must_use]
    pub const fn document(&self) -> &Document {
        let Self { document, .. } = self;

        document
    }

    /// Discover `MiniJinja` templates belonging to the selected platform layer.
    ///
    /// # Errors
    ///
    /// Returns an error when recursive file discovery fails.
    pub fn template_files(&self) -> Result<Vec<Utf8PathBuf>, PlatformFileError> {
        let Self { root, .. } = self;

        Self::files(root.as_path(), "jinja")
    }

    /// Discover files with one extension below a platform directory.
    fn files(root: &Utf8Path, extension: &str) -> Result<Vec<Utf8PathBuf>, PlatformFileError> {
        let mut target = Vec::new();

        for entry in WalkDir::new(root.as_std_path()) {
            let entry = entry.map_err(|error| PlatformFileError::Walk {
                root: root.to_path_buf(),
                error,
            })?;
            if !entry.file_type().is_file() {
                continue;
            }
            let path = Utf8PathBuf::from_path_buf(entry.into_path())
                .map_err(|path| PlatformFileError::NonUtf8Path { path })?;
            if path.extension().is_some_and(|value| value == extension) {
                target.push(path);
            }
        }
        target.sort();
        Ok(target)
    }
}

impl Overlay for Platform {
    type Error = OverlayError;

    fn overlay(self, derived: Self) -> Result<Self, Self::Error> {
        let Self { document, .. } = self;
        let Self {
            root,
            document: derived,
        } = derived;

        document
            .overlay(derived)
            .map(|document| Self { root, document })
    }
}

impl Serialize for Platform {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let Self { document, .. } = self;

        document.serialize(serializer)
    }
}
