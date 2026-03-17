#![allow(unused_assignments)] // miette actually reads those values, see
// `LoadProjectConfigFileError`

use crate::{DependencyConfig, DependencyNameOrDependencyConfig, PluginConfig, TargetConfig};
use miette::{Diagnostic, NamedSource, SourceSpan};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
#[error("failed to parse toml: {0}")]
pub struct TomlDeErrorMsg(String);

#[derive(Debug, Error, Diagnostic)]
pub enum LoadProjectConfigFileError {
	#[error("faield to read file: {0}")]
	IO(#[from] std::io::Error),
	#[error("failed to load project config file")]
	Toml {
		#[diagnostic_source]
		toml_error_msg: TomlDeErrorMsg,
		#[source_code]
		source_code: NamedSource<String>,
		#[label("here")]
		span: SourceSpan,
	},
}

/// Configuration of a project
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectConfigFile {
	#[serde(flatten, default)]
	pub config: ProjectConfig,
	/// treat warnings as errors, exiting with exit code on warnings
	#[serde(default)]
	pub warnings_as_errors: bool,
	/// map of plugin names to their config
	#[serde(default)]
	pub plugins: HashMap<String, PluginConfig>,
}
impl ProjectConfigFile {
	pub fn read_from_file(
		filepath: impl AsRef<Path>,
	) -> Result<Self, Box<LoadProjectConfigFileError>> {
		let content = std::fs::read_to_string(filepath.as_ref())
			.map_err(|e| Box::new(LoadProjectConfigFileError::IO(e)))?;
		let filename = filepath
			.as_ref()
			.file_name()
			.map(|str| str.to_string_lossy().to_string())
			.unwrap_or("<unknown filename>".to_string());

		toml::from_str(&content).map_err(|cause| {
			let span = cause
				.span()
				.map(|range| range.into())
				.unwrap_or((0..0).into());
			Box::new(LoadProjectConfigFileError::Toml {
				toml_error_msg: TomlDeErrorMsg(cause.message().to_string()),
				source_code: NamedSource::new(filename, content),
				span,
			})
		})
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
	/// referencing a depedency by name does not count as a declaration
	pub fn count_dependency_declarations(&self) -> usize {
		fn count_dependency_declarations_in_subdependencies(
			subdependencies: &[DependencyNameOrDependencyConfig],
		) -> usize {
			subdependencies
				.iter()
				.map(|d| match d {
					DependencyNameOrDependencyConfig::Name(_) => 0,
					DependencyNameOrDependencyConfig::Config(subdep) => {
						1 + count_dependency_declarations_in_subdependencies(
							&subdep.config.subdependencies,
						)
					}
				})
				.sum()
		}

		self.dependencies.len()
			+ self
				.targets
				.values()
				.map(|t| count_dependency_declarations_in_subdependencies(&t.dependencies))
				.sum::<usize>()
	}

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
