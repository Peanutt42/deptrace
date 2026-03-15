use std::{collections::HashMap, sync::Arc};

use deptrace_config::DependencyKind;

/// name of dependency is key for HashMap<String, Arc<Dependency>>
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
	pub kinds: Vec<DependencyKind>,
	/// list of filenames of the dynamic libraries this dependency provides
	pub provides_libraries: Vec<String>,
	/// list of dependency that this dependency relies on
	pub subdependencies: HashMap<String, Arc<Dependency>>,
}
impl Dependency {
	pub fn provides_library(&self, library: &str) -> bool {
		self.provides_libraries.contains(&library.to_string())
			|| self
				.subdependencies
				.values()
				.any(|subdep| subdep.provides_library(library))
	}
}
