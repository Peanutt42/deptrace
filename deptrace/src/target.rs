use std::{collections::HashMap, path::PathBuf, sync::Arc};

use deptrace_config::NormalOrPluginName;

use crate::Dependency;

/// target <=> executable
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
	pub filepath: PathBuf,
	pub dependencies: HashMap<NormalOrPluginName, Arc<Dependency>>,
}
impl Target {
	pub fn new(
		filepath: PathBuf,
		dependencies: HashMap<NormalOrPluginName, Arc<Dependency>>,
	) -> Self {
		Self {
			filepath,
			dependencies,
		}
	}
}
