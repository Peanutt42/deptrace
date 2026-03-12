use clap::Parser;
use deptrace::{PluginProvider, Plugins};
use deptrace_cargo_plugin::CargoPluginProvider;
use deptrace_config::{LoadProjectConfigFileError, ProjectConfigFile};
use std::path::PathBuf;

#[derive(Debug, Clone, Parser)]
#[command(version)]
pub struct Cli {
    pub executable: PathBuf,
    /// (optional) override the project_dir used, current working dir is used by default
    pub override_project_dir: Option<PathBuf>,
    /// (optional) list of plugins that should not be loaded (additional to the `disabled_plugins`
    /// in the project configuration file)
    pub disabled_plugins: Vec<String>,
}
impl Cli {
    pub fn load_project_config(&mut self) -> Result<ProjectConfigFile, ()> {
        let project_dir = self.override_project_dir.clone().unwrap_or(
            std::env::current_dir().expect("failed to get the current working directory"),
        );

        // relative to the project_dir
        let possible_project_config_filenames: &[&str] = &["deptrace.toml", ".deptrace.toml"];
        let mut project_config_file_and_filepath: Option<(ProjectConfigFile, PathBuf)> = None;

        for possible_project_config_filename in possible_project_config_filenames {
            let possible_project_config_filepath =
                project_dir.join(possible_project_config_filename);

            if let Some((_, used_project_config_filepath)) =
                project_config_file_and_filepath.as_ref()
                && possible_project_config_filepath.exists()
            {
                println!(
                    "WARNING: Multiple configuration files! Will not load {} as {} is already used.",
                    possible_project_config_filepath.display(),
                    used_project_config_filepath.display()
                );
            }

            match ProjectConfigFile::read_from_file(&possible_project_config_filepath) {
                Ok(loaded_project_config_file) => {
                    project_config_file_and_filepath =
                        Some((loaded_project_config_file, possible_project_config_filepath));
                }
                Err(LoadProjectConfigFileError::Toml(e)) => {
                    // TODO: add warning function/log
                    eprintln!(
                        "Could not load project configuration file {}, invalid format: {e}",
                        possible_project_config_filepath.display()
                    );
                }
                Err(LoadProjectConfigFileError::IO(_)) => {}
            }
        }

        let mut project_config_file = match project_config_file_and_filepath {
            Some((project_config_file, _)) => project_config_file,
            None => ProjectConfigFile::default(),
        };

        let plugin_providers: Vec<Box<dyn PluginProvider>> = vec![Box::new(CargoPluginProvider)];

        let mut disabled_plugin_names = project_config_file.disabled_plugins.clone();
        disabled_plugin_names.append(&mut self.disabled_plugins.clone());

        let plugins = Plugins::load_suitable(project_dir, plugin_providers, &disabled_plugin_names);

        println!(
            "Plugins used: {}",
            plugins
                .iter()
                .map(|(plugin_name, _plugin)| *plugin_name)
                .collect::<Vec<&'static str>>()
                .join(", ")
        );

        let plugins_project_config = plugins
            .generate_project_config()
            .expect("a plugin failed to generate their project config");

        project_config_file.config = project_config_file
            .config
            .merge(plugins_project_config)
            .expect("failed to merge project config file with plugin loaded configuration");

        Ok(project_config_file)
    }
}
