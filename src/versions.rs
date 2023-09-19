use std::{collections::HashMap, str::FromStr};

use semver::{BuildMetadata, Comparator, Op, Prerelease, Version, VersionReq};

use crate::{
    errors::{CommandError, ParseError},
    types::VersionData,
};

const EMPTY_VERSION: Version = Version {
    major: 0,
    minor: 0,
    patch: 0,
    pre: Prerelease::EMPTY,
    build: BuildMetadata::EMPTY,
};

type PackageDetails = (String, Option<Comparator>);

pub struct Versions;
impl Versions {
    pub fn parse_package_details(details: String) -> Result<PackageDetails, ParseError> {
        let mut split = details.split("@");

        let name = split
            .next()
            .expect("Provided package name is empty")
            .to_string();

        let version_raw = match split.next() {
            Some(version_raw) if version_raw == "latest" => return Ok((name, None)),
            Some(version_raw) => version_raw,
            None => return Ok((name, None)),
        };

        let version = VersionReq::parse(version_raw)
            .or_else(|err| Err(ParseError::InvalidVersionNotation(err)))?;

        let comparator = version
            .comparators
            .get(0)
            .expect("Missing version comparator")
            .clone(); // Annoyingly we have to clone because we can't move out of the array

        Ok((name, Some(comparator)))
    }

    /// If a version comparator has the major, patch and minor available a string version will be returned with the resolved version.
    /// This version string can be used to retrieve a package version from the NPM registry.
    /// If the version is not resolvable without requesting the full package data, None will be returned.
    /// None will also be returned if the version operator is Op::Less (<?.?.?) because we need all versions to get the latest version less than this
    pub fn resolve_full_version(semantic_version: Option<&Comparator>) -> Option<String> {
        let latest = String::from("latest");

        let semantic_version = match semantic_version {
            Some(semantic_version) => semantic_version,
            None => return Some(latest),
        };

        let (minor, patch) = match (semantic_version.minor, semantic_version.patch) {
            (Some(minor), Some(patch)) => (minor, patch),
            _ => return None,
        };

        match semantic_version.op {
            Op::Greater | Op::GreaterEq | Op::Wildcard => Some(latest),
            Op::Exact | Op::LessEq | Op::Tilde | Op::Caret => {
                Some(Self::to_string(semantic_version.major, minor, patch))
            }
            _ => None,
        }
    }

    /// Should only be executed if the version comparator is missing a minor or patch.
    /// This can be checked with resolve_full_version() which will return None if this is the case.
    pub fn resolve_partial_version(
        semantic_version: Option<&Comparator>,
        available_versions: &HashMap<String, VersionData>,
    ) -> Result<String, CommandError> {
        let semantic_version = semantic_version
            .expect("Function should not be called as the version can be resolved to 'latest'");

        let mut versions = available_versions.iter().collect::<Vec<_>>();

        // Serde scambles the order of the hashmap so we need to reorder it to find the latest versions
        Self::sort(&mut versions);

        if semantic_version.op == Op::Less {
            // Annoyingly we can't put `if let` and other comparisons on the same line as it's unstable as of writing
            if let (Some(minor), Some(patch)) = (semantic_version.minor, semantic_version.patch) {
                let version_position = versions
                    .iter()
                    .position(|(ver, _)| {
                        ver == &&Self::to_string(semantic_version.major, minor, patch)
                    })
                    .ok_or(CommandError::InvalidVersion)?;

                return Ok(versions
                    .get(version_position - 1)
                    .expect("Invalid version provided (no smaller versions available)")
                    .0
                    .to_string());
            }
        }

        let mut versions = available_versions.iter().collect::<Vec<_>>();

        // Do in reverse order so we find the latest compatible version.
        for (version_str, _) in versions.iter().rev() {
            let version = Version::from_str(version_str.as_str()).unwrap_or(EMPTY_VERSION);

            if semantic_version.matches(&version) {
                return Ok(version_str.to_string());
            }
        }

        Err(CommandError::InvalidVersion)
    }

    // NOTE(conaticus): This might not be effective for versions that include a prerelease in the version (experimental, canary etc)
    fn sort(versions_vec: &mut Vec<(&String, &VersionData)>) {
        versions_vec.sort_by(|a, b| a.0.cmp(b.0))
    }

    fn to_string(major: u64, minor: u64, patch: u64) -> String {
        format!("{}.{}.{}", major, minor, patch)
    }
}
