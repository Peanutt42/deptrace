use crate::{Dependency, Target};
use deptrace_config::{
	DependencyConfig, DependencyKind, DependencyNameOrDependencyConfig, NamedDependencyConfig,
	ProjectConfig,
};
use indexmap::IndexSet;
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

#[derive(Debug, Clone, Error, PartialEq, Eq)]
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
	#[error("cyclic dependency detected: {dependency_cicle}")]
	CyclicDependency { dependency_cicle: DependencyCycle },
}

pub fn resolve_project_config(
	project_config: ProjectConfig,
) -> Result<Project, ResolveProjectError> {
	// phase 1: collect all dependency declarations
	let unresolved_project_dependency_declarations =
		collect_unresolved_project_dependency_declarations(&project_config)?;

	// phase 2: remove all dependency declarations in subdependencies
	// (this means we dont have to worry about the infinetly recursive subdependency declarations
	// anymore)
	// then, iteratively resolve all subdeps of the declared dependencies, until everything is
	// resolved
	let partially_unresolved_dependencies: HashMap<
		String,
		Rc<RefCell<PartiallyUnresolvedDependency>>,
	> = unresolved_project_dependency_declarations
		.into_iter()
		.map(|(name, dep)| {
			// removes the dependency declarations in subdependencies, replaces them with name references
			let dep: UnresolvedDependencyRefSubdependencyOnly =
				NamedDependencyConfig::new(name.clone(), dep).into();

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
		.collect();

	for dep in partially_unresolved_dependencies.values() {
		for subdep in dep.borrow_mut().subdependencies.iter_mut() {
			if let UnresolvedOrPartiallyUnresolvedDependency::Unresolved(subdep_name) = subdep {
				let Some(subdep_ref) = partially_unresolved_dependencies.get(subdep_name.as_str())
				else {
					return Err(ResolveProjectError::UnresolvedDependency {
						dep_name: subdep_name.clone(),
					});
				};
				*subdep = UnresolvedOrPartiallyUnresolvedDependency::Partially(subdep_ref.clone());
			}
		}
	}

	// phase 3
	let mut resolved_project_dependencies: HashMap<String, Arc<Dependency>> = HashMap::new();
	for dep in partially_unresolved_dependencies.into_values() {
		match dep
			.borrow()
			.clone()
			.try_to_resolve(&mut resolved_project_dependencies)
		{
			Ok(()) => {}
			Err(e) => match e {
				ResolveDependencyError::UnresolvedDependency(_) => unreachable!("{e}"),
				ResolveDependencyError::CyclicDependency { dependency_cicle } => {
					return Err(ResolveProjectError::CyclicDependency { dependency_cicle });
				}
			},
		};
	}

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
	fn try_to_resolve(
		self,
		resolved_project_dependencies: &mut HashMap<String, Arc<Dependency>>,
	) -> Result<(), ResolveDependencyError> {
		// ignore the Ok value
		self.try_to_resolve_impl(resolved_project_dependencies, &mut IndexSet::new())
			.map(|_| ())
	}

	/// IndexSet preserves the insertion order
	fn try_to_resolve_impl(
		self,
		resolved_project_dependencies: &mut HashMap<String, Arc<Dependency>>,
		visited_dep_names: &mut IndexSet<String>,
	) -> Result<(String, Arc<Dependency>), ResolveDependencyError> {
		if let Some(resolved_dep) = resolved_project_dependencies.get(&self.name) {
			return Ok((self.name, resolved_dep.clone()));
		}

		if visited_dep_names.contains(&self.name) {
			let dependency_cycle = DependencyCycle::from_visited_dependencies(
				visited_dep_names.clone(),
				self.name.clone(),
			);
			return Err(ResolveDependencyError::CyclicDependency {
				dependency_cicle: dependency_cycle,
			});
		}
		visited_dep_names.insert(self.name.clone());

		let mut subdependencies = HashMap::with_capacity(self.subdependencies.len());
		for subdep in self.subdependencies {
			let mut visited_dep_names = visited_dep_names.clone();

			match subdep {
				UnresolvedOrPartiallyUnresolvedDependency::Unresolved(unresolved_dep_name) => {
					return Err(ResolveDependencyError::UnresolvedDependency(
						unresolved_dep_name,
					));
				}
				UnresolvedOrPartiallyUnresolvedDependency::Partially(subdep) => {
					let (subdep_name, subdep) = subdep.borrow().clone().try_to_resolve_impl(
						resolved_project_dependencies,
						&mut visited_dep_names,
					)?;
					subdependencies.insert(subdep_name, subdep);
				}
			}
		}

		let resolved_dependency = Arc::new(Dependency {
			kinds: self.kinds,
			provides_libraries: self.provides,
			subdependencies,
		});

		resolved_project_dependencies.insert(self.name.clone(), resolved_dependency.clone());

		Ok((self.name, resolved_dependency))
	}
}
#[derive(Debug, Clone)]
enum UnresolvedOrPartiallyUnresolvedDependency {
	Unresolved(String),
	Partially(Rc<RefCell<PartiallyUnresolvedDependency>>),
}
#[derive(Debug, Clone, Error)]
enum ResolveDependencyError {
	#[error(
		"unresolved dependency '{0}' even though at this point, there should not be any more unresolved dependencies"
	)]
	UnresolvedDependency(String),
	#[error("cyclic dependency detected: {dependency_cicle}")]
	CyclicDependency { dependency_cicle: DependencyCycle },
}
/// IndexSet preserves the insertion order and also offers O(1) lookup
/// list of the dependency chain of a dependency cicle:
/// given a n entries, the cicle based on indices would look like this: 0 -> 1 -> 2 -> ... -> n -> 0
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyCycle(pub IndexSet<String>);
impl DependencyCycle {
	pub fn from_visited_dependencies(
		visited_dependencies: IndexSet<String>,
		cycle_start_dep_name: String,
	) -> Self {
		let mut found_cycle_start = false;
		let mut dependency_cycle = Self(IndexSet::with_capacity(visited_dependencies.len()));
		for dep_name in visited_dependencies.into_iter() {
			if dep_name == cycle_start_dep_name {
				found_cycle_start = true;
			}
			if found_cycle_start {
				dependency_cycle.0.insert(dep_name);
			}
		}
		dependency_cycle
	}
}
impl std::fmt::Display for DependencyCycle {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self.0.first() {
			Some(recurred_dep_name) => {
				write!(f, "{recurred_dep_name} --> ")?;
				for dep_name in self.0.iter().skip(1) {
					write!(f, "{dep_name} --> ")?;
				}
				write!(f, "{recurred_dep_name}")
			}
			None => write!(f, "invalid dependency cicle: empty vec!"),
		}
	}
}
