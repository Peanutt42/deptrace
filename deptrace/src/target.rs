use std::sync::Arc;

use crate::Dependency;

/// target <=> executable
pub struct Target {
    pub name: String,
    pub dependencies: Vec<Arc<Dependency>>,
}
impl Target {
    pub fn new(name: String, dependencies: Vec<Arc<Dependency>>) -> Self {
        Self { name, dependencies }
    }
}
