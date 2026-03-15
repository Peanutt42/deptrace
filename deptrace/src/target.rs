use std::{collections::HashMap, path::PathBuf, sync::Arc};

use crate::Dependency;

/// target <=> executable
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
	pub filepath: PathBuf,
	pub dependencies: HashMap<String, Arc<Dependency>>,
}
impl Target {
	pub fn new(filepath: PathBuf, dependencies: HashMap<String, Arc<Dependency>>) -> Self {
		Self {
			filepath,
			dependencies,
		}
	}
}
