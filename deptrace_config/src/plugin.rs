use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const fn default_enabled() -> bool {
	true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginConfig {
	#[serde(default = "default_enabled")]
	pub enabled: bool,
	/// this will be parsed by the plugin for configuration
	#[serde(flatten)]
	pub extra_fields: HashMap<String, toml::Value>,
}
