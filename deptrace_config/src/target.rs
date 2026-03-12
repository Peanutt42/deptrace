use crate::DependencyNameOrDependencyConfig;
use serde::{Deserialize, Serialize};

/// target <=> executable
/// name of the target is the key for the HashMap<String, TargetConfig>
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TargetConfig {
    /// list of dependency names
    pub dependencies: Vec<DependencyNameOrDependencyConfig>,
}
