use crate::{Dependency, Target};
use deptrace_config::{DependencyNameOrDependencyConfig, ProjectConfig};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum ResolveProjectError {
    #[error("unresolved dependency '{dep_name}'")]
    UnresolvedDependency {
        dep_name: String,
        // usage_source: Source,
    },
    #[error("dublicate dependency definition of '{dep_name}'")]
    DublicateDependencyDefinition {
        dep_name: String,
        // first_source: Source,
        // dublicate_source: Source,
    },
}

pub fn resolve_project_config(
    project_config: ProjectConfig,
) -> Result<Vec<Target>, ResolveProjectError> {
    let mut targets = Vec::new();

    // TODO: make iterativ resolve algo that resolves dependency names step by step, aborting when
    // no deps can be resolved in a step
    let mut project_dependencies: Vec<Arc<Dependency>> = Vec::new();

    for (name, target_config) in project_config.targets {
        let mut dependencies = Vec::new();

        for dependency in target_config.dependencies {
            match dependency {
                DependencyNameOrDependencyConfig::Name(dep_name) => {}
                DependencyNameOrDependencyConfig::Config(dep_config) => {}
            }
        }

        targets.push(Target::new(name, dependencies));
    }

    Ok(targets)
}
