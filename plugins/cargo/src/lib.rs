use cargo_metadata::{Metadata, MetadataCommand};
use deptrace::{Plugin, PluginProvider};
use deptrace_config::{ProjectConfig, TargetConfig};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum CargoPluginGenerateError {}

pub struct CargoPlugin {
    cargo_metadata: Metadata,
}
impl Plugin for CargoPlugin {
    fn generate_project_config(
        &self,
    ) -> Result<ProjectConfig, Box<dyn std::error::Error + Send + Sync>> {
        let mut project_config = ProjectConfig::default();

        for package in self.cargo_metadata.workspace_packages() {
            for target in &package.targets {
                if target.is_dylib() || target.is_cdylib() {
                    // TODO: add to (dynamic) libraries
                }

                if target.is_bin() {
                    project_config.targets.insert(
                        target.name.clone(),
                        TargetConfig {
                            // TODO
                            dependencies: vec![],
                        },
                    );
                }
            }
        }

        Ok(project_config)
    }
}

// TODO: figure out whether we should run the CargoMetadata command here or later in
// `generate_project_config`
pub struct CargoPluginProvider;
impl PluginProvider for CargoPluginProvider {
    fn get_plugin_name(&self) -> &'static str {
        "cargo"
    }

    fn try_load_plugin(&self, project_dir: &Path) -> Option<Box<dyn Plugin>> {
        // if there is no Cargo.toml, we dont enable the CargoPlugin, eventhough there could be a
        // cargo workspace in a parent directory
        if !project_dir.join("Cargo.toml").exists() {
            return None;
        }

        let mut metadata_command = MetadataCommand::new();
        metadata_command.current_dir(project_dir);

        let metadata = metadata_command.exec().ok()?;

        Some(Box::new(CargoPlugin {
            cargo_metadata: metadata,
        }))
    }
}
