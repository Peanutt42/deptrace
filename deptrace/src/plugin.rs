use deptrace_config::ProjectConfig;
use std::{collections::HashMap, path::PathBuf};

pub trait Plugin {
    fn generate_project_config(&self) -> Result<ProjectConfig, Box<dyn std::error::Error>>;
}

pub trait PluginProvider {
    fn get_plugin_name(&self) -> &'static str;

    /// plugin providers should return None as soon as possible if they know the project does not
    /// fit this plugin
    fn try_load_plugin(&self, project_dir: PathBuf) -> Option<Box<dyn Plugin>>;
}

pub struct Plugins {
    /// maps a plugin's name to the plugin
    plugins: HashMap<&'static str, Box<dyn Plugin>>,
}
impl Plugins {
    /// loads suitable plugins given the project, excluding the explicitly disabled ones ofc
    pub fn load_suitable(
        project_dir: PathBuf,
        plugin_providers: Vec<Box<dyn PluginProvider>>,
        disabled_plugin_names: &[String],
    ) -> Self {
        let mut suitable_plugins = HashMap::new();

        for plugin_provider in plugin_providers {
            let name = plugin_provider.get_plugin_name();
            if disabled_plugin_names.contains(&name.to_string()) {
                continue;
            }

            if let Some(plugin) = plugin_provider.try_load_plugin(project_dir.clone()) {
                suitable_plugins.insert(name, plugin);
            }
        }

        Self {
            plugins: suitable_plugins,
        }
    }

    pub fn generate_project_config(self) -> Result<ProjectConfig, Box<dyn std::error::Error>> {
        let mut project_config = ProjectConfig::default();
        for plugin in self.plugins.into_values() {
            project_config = project_config.merge(plugin.generate_project_config()?)?;
        }
        Ok(project_config)
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, &'static str, Box<dyn Plugin>> {
        self.plugins.iter()
    }
}
