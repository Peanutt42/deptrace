use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Dependency {
    /// name of this dependency, should match the package name so its easier to find
    pub name: String,
    /// list of filenames of the dynamic libraries this dependency provides
    pub provides_libraries: Vec<String>,
    // TODO: is this needed?
    /// list of dependency that this dependency relies on
    pub subdependencies: Vec<Arc<Dependency>>,
}
