//! Platform build support for Nekor

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A platform specification.
///
/// This is the root-level structure contained within a `platform.toml` file.
#[derive(Debug, Serialize, Deserialize)]
pub struct PlatformSpec {
    /// The top-level `platform` key.
    ///
    /// This serves to contain the most basic information for the platform.
    platform: PlatformInfo,

    /// The build information relevant to this platform specification file.
    build: PlatformBuildInfo,

    /// The platform-specific scopes defined by the specification file.
    #[serde(default)]
    scope: BTreeMap<String, serde_json::Value>,
}

impl PlatformSpec {
    /// Determine the platform information.
    #[inline]
    pub fn platform(&self) -> &PlatformInfo {
        let Self { platform, .. } = self;

        platform
    }

    /// Determine the build information relevant to the platform.
    #[inline]
    pub fn build(&self) -> &PlatformBuildInfo {
        let Self { build, .. } = self;

        build
    }

    /// Determine the available scopes for this platform.
    #[inline]
    pub fn scope(&self) -> &BTreeMap<String, serde_json::Value> {
        let Self { scope, .. } = self;

        scope
    }
}

/// Basic platform information.
#[derive(Debug, Serialize, Deserialize)]
pub struct PlatformInfo {
    /// The base platform specification to use, if any.
    ///
    /// This is a bare platform name.
    base: Option<String>,
}

impl PlatformInfo {
    /// Determine the base platform to inherit from.
    #[inline]
    pub fn base(&self) -> Option<&str> {
        let Self { base, .. } = self;

        base.as_deref()
    }
}

/// Build information for a platform.
#[derive(Debug, Serialize, Deserialize)]
pub struct PlatformBuildInfo {
    /// The scopes that are applied by-default to the platform.
    scopes: Option<Vec<String>>,
}

impl PlatformBuildInfo {
    /// Determine the default scopes to build with for this platform.
    #[inline]
    pub fn scopes(&self) -> impl Iterator<Item = &str> + use<'_> {
        let Self { scopes, .. } = self;

        scopes
            .iter()
            .map(Vec::<String>::as_slice)
            .map(IntoIterator::into_iter)
            .flatten()
            .map(String::as_str)
    }
}
