mod dependency;
pub use dependency::{
	DependencyConfig, DependencyKind, DependencyNameOrDependencyConfig, NamedDependencyConfig,
};

mod project;
pub use project::{LoadProjectConfigFileError, ProjectConfig, ProjectConfigFile};

mod target;
pub use target::TargetConfig;
