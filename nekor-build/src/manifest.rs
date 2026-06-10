use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
};

use cargo_metadata::camino::{Utf8Path, Utf8PathBuf};

use serde::{Deserialize, Serialize};

/// A build manifest.
#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    /// The manifest build information.
    build: ManifestBuildInfo,

    /// The database of available platforms.
    platform: PlatformDatabase,
}

impl Manifest {
    /// Determine the manifest build information.
    #[inline]
    pub fn build(&self) -> &ManifestBuildInfo {
        let Self { build, .. } = self;

        build
    }

    /// Determine the platform database specified in the manifest.
    #[inline]
    pub fn platform(&self) -> &PlatformDatabase {
        let Self { platform, .. } = self;

        platform
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManifestBuildInfo {
    /// The default platform to build for, if specified.
    default: Option<String>,
}

impl ManifestBuildInfo {
    /// Determine the default platform name to build.
    #[inline]
    pub fn platform(&self) -> Option<&str> {
        let Self { default, .. } = self;

        default.as_deref()
    }
}

/// The database of all available platforms.
///
/// This is a new-type over a [`BTreeMap<String, PlatformDesc>`].
#[derive(Debug, Clone, Deserialize)]
#[serde(from = "Vec<PlatformDesc>")]
pub struct PlatformDatabase(BTreeMap<String, PlatformDesc>);

impl From<Vec<PlatformDesc>> for PlatformDatabase {
    fn from(platforms: Vec<PlatformDesc>) -> Self {
        let map = platforms
            .into_iter()
            .map(|platform| (platform.name.clone(), platform))
            .collect();

        PlatformDatabase(map)
    }
}

impl Deref for PlatformDatabase {
    type Target = BTreeMap<String, PlatformDesc>;

    fn deref(&self) -> &Self::Target {
        let Self(target_value) = self;

        target_value
    }
}

impl DerefMut for PlatformDatabase {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let Self(target_value) = self;

        target_value
    }
}

/// A basic description about a platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformDesc {
    /// The name of the platform in question.
    name: String,

    /// The path to the platform configuration file.
    path: Utf8PathBuf,

    /// The description of the platform.
    description: Option<String>,
}

impl PlatformDesc {
    /// Determine the name of the platform.
    #[inline]
    pub fn name(&self) -> &str {
        let Self { name, .. } = self;

        name.as_str()
    }

    /// Determine the path to the platform configuration file.
    #[inline]
    pub fn path(&self) -> &Utf8Path {
        let Self { path, .. } = self;

        path.as_path()
    }

    /// Determine the description of the platform.
    #[inline]
    pub fn description(&self) -> Option<&str> {
        let Self { description, .. } = self;

        description.as_deref()
    }
}
