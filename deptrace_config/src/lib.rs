mod dependency;
pub use dependency::{
	DependencyConfig, DependencyKind, DependencyNameOrDependencyConfig, NamedDependencyConfig,
};

mod project;
pub use project::{LoadProjectConfigFileError, ProjectConfig, ProjectConfigFile};

mod target;
pub use target::TargetConfig;

mod plugin;
pub use plugin::PluginConfig;

use colored::Colorize;

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum NormalOrPluginName {
	Normal(String),
	Plugin { plugin_name: String, name: String },
}
impl NormalOrPluginName {
	pub fn pretty_fmt(&self) -> String {
		match self {
			Self::Normal(name) => name.clone(),
			Self::Plugin { plugin_name, name } => format!("{}:{name}", plugin_name.bright_black()),
		}
	}
}
impl From<String> for NormalOrPluginName {
	fn from(value: String) -> Self {
		match value.split_once(':') {
			Some((plugin_name, name)) => Self::Plugin {
				plugin_name: plugin_name.to_string(),
				name: name.to_string(),
			},
			None => Self::Normal(value),
		}
	}
}
impl serde::Serialize for NormalOrPluginName {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let str = self.to_string();
		str.serialize(serializer)
	}
}
impl<'de> serde::Deserialize<'de> for NormalOrPluginName {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		Ok(String::deserialize(deserializer)?.into())
	}
}
impl std::fmt::Debug for NormalOrPluginName {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.to_string().fmt(f)
	}
}
impl std::fmt::Display for NormalOrPluginName {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Normal(name) => name.fmt(f),
			Self::Plugin { plugin_name, name } => {
				write!(f, "{plugin_name}:{name}")
			}
		}
	}
}
