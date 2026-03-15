use deptrace_config::ProjectConfig;
use std::{collections::HashMap, path::Path};
use thiserror::Error;

pub type PluginPrintlnCallback = Box<dyn Fn(String)>;

pub trait Plugin {
	fn generate_project_config(
		&self,
		println_callback: PluginPrintlnCallback,
	) -> Result<ProjectConfig, Box<dyn std::error::Error + Send + Sync>>;
}

pub trait PluginProvider {
	fn get_plugin_name(&self) -> &'static str;

	/// plugin providers should return None as soon as possible if they know the project does not
	/// fit this plugin
	fn try_load_plugin(&self, project_dir: &Path) -> Option<Box<dyn Plugin>>;
}

#[derive(Debug, Error)]
#[error("plugin '{plugin_name}' failed to generate their configuration")]
pub struct PluginsGenerateConfigError {
	pub plugin_name: String,
	#[source]
	pub source: Box<dyn std::error::Error + Send + Sync>,
}

pub struct Plugins {
	/// maps a plugin's name to the plugin
	plugins: HashMap<&'static str, Box<dyn Plugin>>,
}
impl Plugins {
	/// loads suitable plugins given the project, excluding the explicitly disabled ones ofc
	pub fn load_suitable(
		project_dir: impl AsRef<Path>,
		plugin_providers: Vec<Box<dyn PluginProvider>>,
		disabled_plugin_names: &[String],
	) -> Self {
		let mut suitable_plugins = HashMap::new();

		for plugin_provider in plugin_providers {
			let name = plugin_provider.get_plugin_name();
			if disabled_plugin_names.contains(&name.to_string()) {
				continue;
			}

			if let Some(plugin) = plugin_provider.try_load_plugin(project_dir.as_ref()) {
				suitable_plugins.insert(name, plugin);
			}
		}

		Self {
			plugins: suitable_plugins,
		}
	}

	pub fn iter(&self) -> std::collections::hash_map::Iter<'_, &'static str, Box<dyn Plugin>> {
		self.plugins.iter()
	}

	pub fn len(&self) -> usize {
		self.plugins.len()
	}

	pub fn is_empty(&self) -> bool {
		self.plugins.is_empty()
	}
}
impl std::iter::IntoIterator for Plugins {
	type Item = (&'static str, Box<dyn Plugin>);
	type IntoIter = std::collections::hash_map::IntoIter<&'static str, Box<dyn Plugin>>;
	fn into_iter(self) -> Self::IntoIter {
		self.plugins.into_iter()
	}
}
