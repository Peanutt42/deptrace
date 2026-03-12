use crate::{DependencyConfig, TargetConfig};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LoadProjectConfigFileError {
    #[error("faield to read file: {0}")]
    IO(#[from] std::io::Error),
    #[error("failed to parse toml: {0}")]
    Toml(#[from] toml::de::Error),
}

/// Configuration of a project
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectConfigFile {
    #[serde(flatten, default)]
    pub config: ProjectConfig,
    /// list of plugin names to explicitly not automatically load for this project
    #[serde(default)]
    pub disabled_plugins: Vec<String>,
}
impl ProjectConfigFile {
    pub fn read_from_file(filepath: impl AsRef<Path>) -> Result<Self, LoadProjectConfigFileError> {
        let content = std::fs::read_to_string(filepath)?;
        Ok(toml::from_str(&content)?)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// name of the entire project, not used for anything important right now (optional)
    #[serde(default)]
    pub name: Option<String>,
    /// dependency configurations that are reused in multiple targets in this project
    #[serde(default)]
    pub dependencies: HashMap<String, DependencyConfig>,

    #[serde(default)]
    pub targets: HashMap<String, TargetConfig>,
}
#[derive(Debug, Clone, Error)]
pub enum MergeProjectConfigError {
    #[error("there are multiple different project names in multiple project configuration files")]
    MultipleDifferentProjectNames,
    #[error("there are dublicate targets between multiple project configuration files")]
    DublicateTarget,
    #[error("there are dublicate dependencies between multiple project configuration files")]
    DublicateDependency,
}
impl ProjectConfig {
    pub fn merge(self, other: Self) -> Result<Self, MergeProjectConfigError> {
        let name = match (&self.name, &other.name) {
            (Some(self_name), Some(other_name)) => {
                if self_name == other_name {
                    self.name
                } else {
                    return Err(MergeProjectConfigError::MultipleDifferentProjectNames);
                }
            }
            _ => self.name.or(other.name),
        };

        let mut dependencies = HashMap::new();
        for (dependency_name, dependency) in self
            .dependencies
            .into_iter()
            .chain(other.dependencies.into_iter())
        {
            if dependencies.insert(dependency_name, dependency).is_some() {
                return Err(MergeProjectConfigError::DublicateDependency);
            }
        }

        let mut targets = HashMap::new();
        for (target_name, target) in self.targets.into_iter().chain(other.targets.into_iter()) {
            if targets.insert(target_name, target).is_some() {
                return Err(MergeProjectConfigError::DublicateTarget);
            }
        }

        Ok(ProjectConfig {
            name,
            dependencies,
            targets,
        })
    }
}
