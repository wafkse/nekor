use alloc::collections::BTreeMap;
use core::{
    borrow::Borrow,
    convert::Infallible,
    fmt,
    ops::{Deref, DerefMut},
    str::FromStr,
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
    #[must_use]
    pub const fn build(&self) -> &ManifestBuildInfo {
        let &Self { ref build, .. } = self;

        build
    }

    /// Determine the platform database specified in the manifest.
    #[inline]
    #[must_use]
    pub const fn platform(&self) -> &PlatformDatabase {
        let &Self { ref platform, .. } = self;

        platform
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManifestBuildInfo {
    /// The default platform to build for, if specified.
    default: Option<PlatformName>,
}

impl ManifestBuildInfo {
    /// Determine the default platform name to build.
    #[inline]
    #[must_use]
    pub const fn platform(&self) -> Option<&PlatformName> {
        let &Self { ref default, .. } = self;

        default.as_ref()
    }
}

/// The stable manifest identity of a platform.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PlatformName(String);

impl PlatformName {
    /// Construct a platform name from its manifest representation.
    #[inline]
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Determine the manifest representation of this platform name.
    #[inline]
    #[must_use]
    pub const fn as_str(&self) -> &str {
        let &Self(ref value) = self;

        value.as_str()
    }
}

impl fmt::Display for PlatformName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl Borrow<str> for PlatformName {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for PlatformName {
    type Err = Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(value))
    }
}

/// The database of all available platforms.
///
/// This is a new-type over a [`BTreeMap<PlatformName, PlatformDesc>`].
#[derive(Debug, Clone, Deserialize)]
#[serde(from = "Vec<PlatformDesc>")]
pub struct PlatformDatabase(BTreeMap<PlatformName, PlatformDesc>);

impl From<Vec<PlatformDesc>> for PlatformDatabase {
    fn from(platforms: Vec<PlatformDesc>) -> Self {
        let map = platforms
            .into_iter()
            .map(|platform| (platform.name.clone(), platform))
            .collect();

        Self(map)
    }
}

impl Deref for PlatformDatabase {
    type Target = BTreeMap<PlatformName, PlatformDesc>;

    fn deref(&self) -> &Self::Target {
        let &Self(ref target_value) = self;

        target_value
    }
}

impl DerefMut for PlatformDatabase {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let &mut Self(ref mut target_value) = self;

        target_value
    }
}

/// A basic description about a platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformDesc {
    /// The name of the platform in question.
    name: PlatformName,

    /// The path to the platform root directory.
    path: Utf8PathBuf,

    /// The optional base platform inherited by this platform.
    base: Option<PlatformName>,

    /// The description of the platform.
    description: Option<String>,
}

impl PlatformDesc {
    /// Determine the name of the platform.
    #[inline]
    #[must_use]
    pub const fn name(&self) -> &PlatformName {
        let &Self { ref name, .. } = self;

        name
    }

    /// Determine the path to the platform root directory.
    #[inline]
    #[must_use]
    pub fn path(&self) -> &Utf8Path {
        let &Self { ref path, .. } = self;

        path.as_path()
    }

    /// Determine the base platform name.
    #[inline]
    #[must_use]
    pub const fn base(&self) -> Option<&PlatformName> {
        let &Self { ref base, .. } = self;

        base.as_ref()
    }

    /// Determine the description of the platform.
    #[inline]
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        let &Self { ref description, .. } = self;

        description.as_deref()
    }
}
