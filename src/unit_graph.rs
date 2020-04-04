use anyhow::anyhow;
use std::{convert::TryFrom, fmt};

#[derive(Debug, serde::Deserialize)]
pub struct Dependency {
    pub index: usize,
    pub extern_crate_name: String,
    pub public: bool,
    pub noprelude: bool,
}

#[derive(Debug, Eq, PartialEq, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    Build,
    RunCustomBuild,
}

#[derive(Debug, serde::Deserialize)]
pub struct Target {
    pub kind: Vec<String>,
    pub crate_types: Vec<String>,
    pub name: String,
    pub src_path: String,
    pub edition: String,
    pub doctest: bool,
}

#[derive(Debug, serde::Deserialize)]
pub struct Profile {
    pub name: String,
    pub opt_level: String,
    pub lto: String,
    pub codegen_units: Option<u32>,
    pub debuginfo: Option<u32>,
    pub debug_assertions: bool,
    pub overflow_checks: bool,
    pub rpath: bool,
    pub incremental: bool,
    pub panic: String,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct PackageId {
    pub name: String,
    pub version: String,
    pub source: String,
}

impl TryFrom<String> for PackageId {
    type Error = anyhow::Error;

    fn try_from(s: String) -> anyhow::Result<Self> {
        let mut parts = s.split(' ');
        let name = parts.next().ok_or_else(|| anyhow!("No name"))?.to_owned();
        let version = parts
            .next()
            .ok_or_else(|| anyhow!("No version"))?
            .to_owned();
        let source = parts.next().ok_or_else(|| anyhow!("No source"))?.to_owned();
        Ok(Self {
            name,
            version,
            source,
        })
    }
}

impl Into<String> for PackageId {
    fn into(self) -> String {
        format!("{} {} {}", self.name, self.version, self.source)
    }
}

impl fmt::Display for PackageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.name, self.version)?;
        Ok(())
    }
}

impl PartialEq<escargot::format::WorkspaceMember<'_>> for PackageId {
    fn eq(&self, rhs: &escargot::format::WorkspaceMember) -> bool {
        serde_json::to_string(self).unwrap() == serde_json::to_string(rhs).unwrap()
    }
}

impl PartialEq<PackageId> for escargot::format::WorkspaceMember<'_> {
    fn eq(&self, rhs: &PackageId) -> bool {
        serde_json::to_string(self).unwrap() == serde_json::to_string(rhs).unwrap()
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct Unit {
    pub pkg_id: PackageId,
    pub target: Target,
    pub profile: Profile,
    pub platform: Option<String>,
    pub mode: Mode,
    pub features: Vec<String>,
    pub dependencies: Vec<Dependency>,
}

#[derive(Debug, serde::Deserialize)]
pub struct UnitGraph {
    pub version: u8,
    pub roots: Vec<usize>,
    pub units: Vec<Unit>,
}
