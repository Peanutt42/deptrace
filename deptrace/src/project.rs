use crate::{Dependency, Target};
use deptrace_config::{
	DependencyConfig, DependencyKind, DependencyNameOrDependencyConfig, NamedDependencyConfig,
	ProjectConfig,
};
use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Arc};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
	pub name: Option<String>,
	pub targets: HashMap<String, Target>,
	pub dependencies: HashMap<String, Arc<Dependency>>,
}
impl Project {
	pub fn new(
		name: Option<String>,
		targets: HashMap<String, Target>,
		dependencies: HashMap<String, Arc<Dependency>>,
	) -> Self {
		Self {
			name,
			targets,
			dependencies,
		}
	}
}

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
) -> Result<Project, ResolveProjectError> {
	// phase 1: collect all dependency declarations
	let unresolved_project_dependency_declarations =
		collect_unresolved_project_dependency_declarations(&project_config)?;

	// TODO: merge phase 2 and 3 into one
	// phase 2: remove all dependency declarations in subdependencies
	// (this means we dont have to worry about the infinetly recursive subdependency declarations
	// anymore)
	let unresolved_project_dependencies_ref_subdeps_only =
		unresolved_project_dependency_declarations
			.into_iter()
			.map(|(name, dep)| (name.clone(), NamedDependencyConfig::new(name, dep).into()))
			.collect::<HashMap<String, UnresolvedDependencyRefSubdependencyOnly>>();

	// phase 3: iteratively resolve all subdeps of the declared dependencies, until everything is
	// resolved
	let partially_unresolved_dependencies = unresolved_project_dependencies_ref_subdeps_only
		.into_iter()
		.map(|(name, dep)| {
			(
				name,
				Rc::new(RefCell::new(PartiallyUnresolvedDependency {
					name: dep.name,
					kinds: dep.kinds,
					provides: dep.provides,
					subdependencies: dep
						.subdependencies
						.into_iter()
						.map(UnresolvedOrPartiallyUnresolvedDependency::Unresolved)
						.collect(),
				})),
			)
		})
		.collect::<HashMap<String, Rc<RefCell<PartiallyUnresolvedDependency>>>>();

	loop {
		let mut found_unresolved_dependency = false;
		for dep in partially_unresolved_dependencies.values() {
			for subdep in dep.borrow_mut().subdependencies.iter_mut() {
				if let UnresolvedOrPartiallyUnresolvedDependency::Unresolved(subdep_name) = subdep {
					found_unresolved_dependency = true;
					let Some(subdep_ref) =
						partially_unresolved_dependencies.get(subdep_name.as_str())
					else {
						return Err(ResolveProjectError::UnresolvedDependency {
							dep_name: subdep_name.clone(),
						});
					};
					*subdep =
						UnresolvedOrPartiallyUnresolvedDependency::Partially(subdep_ref.clone());
				}
			}
		}

		if !found_unresolved_dependency {
			break;
		}
	}

	// phase 4
	let resolved_project_dependencies: HashMap<String, Arc<Dependency>> =
		partially_unresolved_dependencies
			.into_values()
			.map(|dep| {
				dep.borrow()
					.clone()
					.try_to_resolve()
					.expect("there should be no more unresolved dependencies after phase 3")
			})
			.collect();

	let mut targets: HashMap<String, Target> = HashMap::new();

	for (target_name, target_config) in project_config.targets {
		let mut resolved_target_dependencies = HashMap::new();

		for dependency in target_config.dependencies {
			let dep_name = match dependency {
				DependencyNameOrDependencyConfig::Name(dep_name) => dep_name,
				DependencyNameOrDependencyConfig::Config(dep_config) => dep_config.name,
			};

			if let Some(resolved_dependency) = resolved_project_dependencies.get(&dep_name) {
				resolved_target_dependencies.insert(dep_name, resolved_dependency.clone());
			} else {
				return Err(ResolveProjectError::UnresolvedDependency { dep_name });
			}
		}

		targets.insert(
			target_name.to_string(),
			Target::new(target_config.filepath, resolved_target_dependencies),
		);
	}

	Ok(Project::new(
		project_config.name,
		targets,
		resolved_project_dependencies,
	))
}

fn collect_unresolved_project_dependency_declarations(
	project_config: &ProjectConfig,
) -> Result<HashMap<String, DependencyConfig>, ResolveProjectError> {
	let mut unresolved_project_dependency_declarations = project_config.dependencies.clone();
	for dep in project_config.dependencies.values() {
		collect_unresolved_dependency_declarations(
			&dep.subdependencies,
			&mut unresolved_project_dependency_declarations,
		)?;
	}
	for target_config in project_config.targets.values() {
		collect_unresolved_dependency_declarations(
			&target_config.dependencies,
			&mut unresolved_project_dependency_declarations,
		)?;
	}

	Ok(unresolved_project_dependency_declarations)
}

/// recursively collects all dependency declarations, especially inside the subdependencies
fn collect_unresolved_dependency_declarations(
	subdeps: &[DependencyNameOrDependencyConfig],
	out_unresolved_project_dependency_declarations: &mut HashMap<String, DependencyConfig>,
) -> Result<(), ResolveProjectError> {
	for subdep in subdeps {
		if let DependencyNameOrDependencyConfig::Config(named_subdep_config) = subdep {
			if out_unresolved_project_dependency_declarations
				.contains_key(&named_subdep_config.name)
			{
				return Err(ResolveProjectError::DublicateDependencyDefinition {
					dep_name: named_subdep_config.name.clone(),
				});
			}
			out_unresolved_project_dependency_declarations.insert(
				named_subdep_config.name.clone(),
				named_subdep_config.config.clone(),
			);

			collect_unresolved_dependency_declarations(
				&named_subdep_config.config.subdependencies,
				out_unresolved_project_dependency_declarations,
			)?;
		}
	}
	Ok(())
}

/// unresolved dependency where all subdependencies are just their names as reference
/// important to reduce the complexity of infinitly recursive structures the inline dependency
/// configuration brings
#[derive(Debug, Clone)]
struct UnresolvedDependencyRefSubdependencyOnly {
	name: String,
	kinds: Vec<DependencyKind>,
	provides: Vec<String>,
	subdependencies: Vec<String>,
}
impl From<NamedDependencyConfig> for UnresolvedDependencyRefSubdependencyOnly {
	fn from(value: NamedDependencyConfig) -> Self {
		Self {
			name: value.name,
			kinds: value.config.kinds,
			provides: value.config.provides,
			subdependencies: value
				.config
				.subdependencies
				.into_iter()
				.map(|subdep| match subdep {
					DependencyNameOrDependencyConfig::Name(subdep_name) => subdep_name,
					DependencyNameOrDependencyConfig::Config(subdep_config) => subdep_config.name,
				})
				.collect(),
		}
	}
}

#[derive(Debug, Clone)]
struct PartiallyUnresolvedDependency {
	name: String,
	kinds: Vec<DependencyKind>,
	provides: Vec<String>,
	subdependencies: Vec<UnresolvedOrPartiallyUnresolvedDependency>,
}
impl PartiallyUnresolvedDependency {
	fn try_to_resolve(self) -> Option<(String, Arc<Dependency>)> {
		let mut subdependencies = HashMap::with_capacity(self.subdependencies.len());
		for subdep in self.subdependencies {
			match subdep {
				UnresolvedOrPartiallyUnresolvedDependency::Unresolved(_) => return None,
				UnresolvedOrPartiallyUnresolvedDependency::Partially(subdep) => {
					let (subdep_name, subdep) = subdep.borrow().clone().try_to_resolve()?;
					subdependencies.insert(subdep_name, subdep);
				}
			}
		}

		Some((
			self.name,
			Arc::new(Dependency {
				kinds: self.kinds,
				provides_libraries: self.provides,
				subdependencies,
			}),
		))
	}
}
#[derive(Debug, Clone)]
enum UnresolvedOrPartiallyUnresolvedDependency {
	Unresolved(String),
	Partially(Rc<RefCell<PartiallyUnresolvedDependency>>),
}
