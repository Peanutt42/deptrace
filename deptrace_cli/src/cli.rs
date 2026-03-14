use clap::Parser;
use colored::Colorize;
use deptrace::{PluginProvider, Plugins, PluginsGenerateConfigError};
use deptrace_cargo_plugin::CargoPluginProvider;
use deptrace_config::{LoadProjectConfigFileError, ProjectConfig, ProjectConfigFile};
use indicatif::{ProgressBar, ProgressStyle};
use miette::{Context, IntoDiagnostic, Result, miette};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Debug, Clone, Parser)]
#[command(version)]
pub struct Cli {
    pub executable: PathBuf,
    /// (optional) override the project_dir used, current working dir is used by default
    #[arg(long)]
    pub override_project_dir: Option<PathBuf>,
    /// (optional) override what project config file is loaded, by default we look for
    /// './deptrace.toml' and for './.deptrace.toml'
    #[arg(long)]
    pub override_project_config_file: Option<PathBuf>,
    /// (optional) list of plugins that should not be loaded (additional to the `disabled_plugins`
    /// in the project configuration file)
    #[arg(long)]
    pub disabled_plugins: Vec<String>,
    /// (optional) treats every warning as a error
    #[arg(long, default_value_t = false)]
    pub warnings_as_errors: bool,
}
impl Cli {
    pub fn load_project_config(&mut self) -> Result<ProjectConfigFile> {
        let project_dir = self.override_project_dir.clone().unwrap_or(
            std::env::current_dir().expect("failed to get the current working directory"),
        );

        let (mut project_config_file, project_config_filepath) =
            match &self.override_project_config_file {
                Some(project_config_filepath) => (
                    ProjectConfigFile::read_from_file(project_config_filepath).map_err(|e| *e)?,
                    Some(project_config_filepath.clone()),
                ),
                None => Self::try_load_project_config_file(&project_dir)?
                    .map(|(project_config_file, project_config_filepath)| {
                        (project_config_file, Some(project_config_filepath))
                    })
                    .unwrap_or((ProjectConfigFile::default(), None)),
            };

        if let Some(project_config_filepath) = project_config_filepath {
            let num_config_targets = project_config_file.config.targets.len();
            println!(
                "{:>14} project config file {} ({num_config_targets} targets)",
                "Loaded".green().bold(),
                project_config_filepath.display()
            );
        }

        let plugin_providers: Vec<Box<dyn PluginProvider>> = vec![Box::new(CargoPluginProvider)];

        let mut disabled_plugin_names = project_config_file.disabled_plugins.clone();
        disabled_plugin_names.append(&mut self.disabled_plugins.clone());

        let plugins =
            Plugins::load_suitable(&project_dir, plugin_providers, &disabled_plugin_names);

        project_config_file.config =
            Self::generate_project_config(plugins, project_config_file.config.clone())
                .into_diagnostic()?;
        Ok(project_config_file)
    }

    fn try_load_project_config_file(
        project_dir: &Path,
    ) -> Result<Option<(ProjectConfigFile, PathBuf)>> {
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
                return Err(miette!(
                    "Multiple configuration files! Will not load {} as {} is already used.",
                    possible_project_config_filepath.display(),
                    used_project_config_filepath.display()
                ));
            }

            match ProjectConfigFile::read_from_file(&possible_project_config_filepath) {
                Ok(loaded_project_config_file) => {
                    project_config_file_and_filepath =
                        Some((loaded_project_config_file, possible_project_config_filepath));
                }
                Err(e) => match *e {
                    LoadProjectConfigFileError::IO(ref e)
                        if matches!(e.kind(), std::io::ErrorKind::NotFound) => {}
                    LoadProjectConfigFileError::Toml { .. } | LoadProjectConfigFileError::IO(_) => {
                        return Err((*e).into());
                    }
                },
            }
        }

        Ok(project_config_file_and_filepath)
    }

    fn generate_project_config(
        plugins: Plugins,
        mut project_config: ProjectConfig,
    ) -> Result<ProjectConfig, PluginsGenerateConfigError> {
        let progressbar = ProgressBar::new(plugins.len() as u64).with_style(
            ProgressStyle::with_template(
                "{spinner:.green} {prefix:>12.cyan.bold} {msg} {pos:>5}/{len}",
            )
            .unwrap()
            .progress_chars("=> "),
        );
        progressbar.set_prefix("Generating");
        progressbar.enable_steady_tick(Duration::from_millis(100));

        for (plugin_name, plugin) in plugins.into_iter() {
            progressbar.set_message(plugin_name);

            let generate_plugin_error =
                |source: Box<dyn std::error::Error + Send + Sync>| PluginsGenerateConfigError {
                    plugin_name: plugin_name.to_string(),
                    source,
                };

            let plugin_project_config = plugin
                .generate_project_config()
                .map_err(generate_plugin_error)?;
            let num_plugin_targets = plugin_project_config.targets.len();

            project_config = project_config
                .merge(plugin_project_config)
                .map_err(|e| generate_plugin_error(Box::new(e)))?;

            progressbar.println(format!(
                "{:>14} {plugin_name} plugin ({num_plugin_targets} targets)",
                "Generated".green().bold()
            ));

            progressbar.inc(1);
        }

        let total_num_targets = project_config.targets.len();

        progressbar.finish_and_clear();
        println!(
            "{:>14} ({total_num_targets} targets)",
            "Finished".green().bold()
        );

        Ok(project_config)
    }
}
