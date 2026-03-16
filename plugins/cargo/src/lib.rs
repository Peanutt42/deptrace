use cargo_metadata::{Metadata, MetadataCommand};
use colored::Colorize;
use deptrace::{Plugin, PluginPrintlnCallback, PluginProvider};
use deptrace_config::{ProjectConfig, TargetConfig};
use std::{
	collections::HashMap,
	io::BufReader,
	path::{Path, PathBuf},
	process::{Command, Stdio},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CargoPluginGenerateError {
	#[error("failed to run cargo build: {0}")]
	RunCargoBuild(#[from] std::io::Error),
	#[error("cargo build did not finish successfully")]
	UnsuccessfullCargoBuild,
	#[error(
		"did not find the executable filepath in the artifact output of cargo metadata for target '{target_name}'"
	)]
	DidNotFindTargetOutputFilepath { target_name: String },
}

pub struct CargoPlugin {
	project_dir: PathBuf,
	cargo_metadata: Metadata,
}
impl Plugin for CargoPlugin {
	// TODO: parse output of `cargo build --message-format=json` in order to get the executable
	// paths of targets / filenames of shared libraries
	// (dx: forward any non json lines with a println, so we have the expected cargo build compiler
	// messages)
	fn generate_project_config(
		&self,
		println_callback: PluginPrintlnCallback,
	) -> Result<ProjectConfig, Box<dyn std::error::Error + Send + Sync>> {
		println_callback(format!("{} cargo build...", "Running".green()));

		let mut cmd = Command::new("cargo")
			.args(["build", "--message-format=json"])
			.current_dir(&self.project_dir)
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.spawn()
			.map_err(CargoPluginGenerateError::RunCargoBuild)?;
		let reader = BufReader::new(cmd.stdout.take().unwrap());

		let mut artifact_output_filepaths = HashMap::new();

		for message in cargo_metadata::Message::parse_stream(reader) {
			match message {
				Ok(message) => match message {
					cargo_metadata::Message::CompilerArtifact(artifact) => {
						println_callback(format!(
							"{} {}...",
							"Compiling".green(),
							artifact.target.name
						));

						if let Some(filepath) = artifact.executable {
							artifact_output_filepaths.insert(artifact.target.name, filepath);
						}
					}
					cargo_metadata::Message::CompilerMessage(msg) => {
						println_callback(msg.message.rendered.unwrap_or(msg.message.message));
						for child_msg in msg.message.children {
							println_callback(child_msg.rendered.unwrap_or(child_msg.message));
						}
					}
					cargo_metadata::Message::BuildScriptExecuted(_build_script) => {}
					cargo_metadata::Message::TextLine(line) => println_callback(line),
					cargo_metadata::Message::BuildFinished(build_finished) => {
						if !build_finished.success {
							return Err(Box::new(
								CargoPluginGenerateError::UnsuccessfullCargoBuild,
							));
						}
					}
					_ => {}
				},
				Err(e) => println_callback(format!("failed to parse cargo metadata message: {e}")),
			}
		}

		let mut project_config = ProjectConfig::default();

		for package in self.cargo_metadata.workspace_packages() {
			// skip packages outside of the project_dir
			if let Ok(package_manifest_path) = package.manifest_path.canonicalize() {
				if !package_manifest_path.starts_with(&self.project_dir) {
					continue;
				}
			} else {
				continue;
			}

			for target in &package.targets {
				if target.is_cdylib() {
					// TODO: add to (dynamic) libraries
				}

				if target.is_bin() {
					let Some(filepath) = artifact_output_filepaths.get(&target.name) else {
						return Err(Box::new(
							CargoPluginGenerateError::DidNotFindTargetOutputFilepath {
								target_name: target.name.clone(),
							},
						));
					};

					project_config.targets.insert(
						target.name.clone(),
						TargetConfig {
							filepath: filepath.clone().into_std_path_buf(),
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
			project_dir: project_dir.to_path_buf(),
			cargo_metadata: metadata,
		}))
	}
}
