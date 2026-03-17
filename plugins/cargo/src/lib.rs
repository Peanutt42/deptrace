use cargo_metadata::{Metadata, MetadataCommand};
use colored::Colorize;
use deptrace::{LoadPluginResult, Plugin, PluginPrintlnCallback, PluginProvider};
use deptrace_config::{
	DependencyConfig, DependencyKind, DependencyNameOrDependencyConfig, ProjectConfig, TargetConfig,
};
use serde::Deserialize;
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
		"did not find artifact filepath in the artifact output of cargo metadata with name '{artifact_name}'"
	)]
	DidNotFindArtifactOutputFilepath { artifact_name: String },
	#[error("the plugin cargo does not have a config field named '{field_name}'")]
	InvalidCargoExtraConfigField { field_name: String },
}

#[derive(Default, Deserialize)]
struct CargoPluginExtraConfigFields {
	extra_cargo_args: Vec<String>,
}

pub struct CargoPlugin {
	project_dir: PathBuf,
	cargo_metadata: Metadata,
	extra_config_fields: CargoPluginExtraConfigFields,
}
impl Plugin for CargoPlugin {
	fn generate_project_config(
		&self,
		println_callback: PluginPrintlnCallback,
	) -> Result<ProjectConfig, Box<dyn std::error::Error + Send + Sync>> {
		let extra_cargo_args_str = self
			.extra_config_fields
			.extra_cargo_args
			.clone()
			.into_iter()
			.map(|a| format!(" {a}"))
			.collect::<String>();
		println_callback(format!(
			"{} cargo build{extra_cargo_args_str}...",
			"Running".green()
		));

		let mut args = vec!["build".to_string(), "--message-format=json".to_string()];
		args.extend_from_slice(&self.extra_config_fields.extra_cargo_args);
		let mut cmd = Command::new("cargo")
			.args(args)
			.current_dir(&self.cargo_metadata.workspace_root)
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.spawn()
			.map_err(CargoPluginGenerateError::RunCargoBuild)?;
		let reader = BufReader::new(cmd.stdout.take().unwrap());

		let mut artifact_output_filepaths: HashMap<String, Vec<PathBuf>> = HashMap::new();

		struct ArtifactLinkedLibsInfo {
			linked_lib_names: Vec<String>,
			linked_lib_search_paths: Vec<PathBuf>,
		}
		let mut artifact_linked_libs: HashMap<String, ArtifactLinkedLibsInfo> = HashMap::new();

		for message in cargo_metadata::Message::parse_stream(reader) {
			match message {
				Ok(message) => match message {
					cargo_metadata::Message::CompilerArtifact(artifact) => {
						if !artifact.fresh {
							println_callback(format!(
								"{} {}...",
								"Compiling".green(),
								artifact.target.name
							));
						}

						let filepaths: Vec<PathBuf> = artifact
							.filenames
							.into_iter()
							.map(|p| p.into_std_path_buf())
							.collect();

						artifact_output_filepaths.insert(artifact.target.name, filepaths);
					}
					cargo_metadata::Message::CompilerMessage(msg) => {
						println_callback(msg.message.rendered.unwrap_or(msg.message.message));
					}
					cargo_metadata::Message::BuildScriptExecuted(build_script) => {
						if let Some(package) = self
							.cargo_metadata
							.packages
							.iter()
							.find(|p| p.id == build_script.package_id)
						{
							let linked_lib_names = build_script
								.linked_libs
								.into_iter()
								.filter_map(|l| {
									let l = l.to_string();
									let (lib_type_str, lib_name) = l.split_once('=')?;
									if lib_type_str == "dylib" {
										Some(lib_name.to_string())
									} else {
										None
									}
								})
								.collect();

							let linked_lib_search_paths = build_script
								.linked_paths
								.into_iter()
								.filter_map(|p| {
									let p = p.to_string();
									let (path_type_str, path_str) = p.split_once('=')?;
									if path_type_str == "native" {
										Some(PathBuf::from(path_str))
									} else {
										None
									}
								})
								.collect();

							artifact_linked_libs.insert(
								package.name.to_string(),
								ArtifactLinkedLibsInfo {
									linked_lib_names,
									linked_lib_search_paths,
								},
							);
						}
					}
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
			let is_inside_project_dir =
				if let Ok(package_manifest_path) = package.manifest_path.canonicalize() {
					package_manifest_path.starts_with(&self.project_dir)
				} else {
					false
				};

			for target in &package.targets {
				let is_cdylib = target.is_cdylib();
				let is_bin = target.is_bin();
				if !is_cdylib && !is_bin {
					continue;
				}
				let Some(filepaths) = artifact_output_filepaths.get(&target.name) else {
					return Err(Box::new(
						CargoPluginGenerateError::DidNotFindArtifactOutputFilepath {
							artifact_name: target.name.clone(),
						},
					));
				};

				if is_cdylib {
					let provides = filepaths
						.iter()
						.filter_map(|p| {
							p.file_name()
								.and_then(std::ffi::OsStr::to_str)
								.map(str::to_string)
						})
						.collect();

					project_config.dependencies.insert(
						target.name.clone(),
						DependencyConfig {
							// TODO
							kinds: vec![DependencyKind::Runtime],
							provides,
							// TODO
							subdependencies: vec![],
						},
					);
				} else if is_bin && is_inside_project_dir {
					let dependencies = artifact_linked_libs
						.get(&target.name)
						.map(|info| {
							info.linked_lib_names
								.clone()
								.into_iter()
								.map(DependencyNameOrDependencyConfig::Name)
								.collect::<Vec<_>>()
						})
						.unwrap_or_default();
					let filepath = match filepaths.first() {
						Some(filepath) if filepaths.len() == 1 => filepath.clone(),
						// TODO: maybe add warning?
						_ => continue,
					};

					project_config.targets.insert(
						target.name.clone(),
						TargetConfig {
							filepath,
							dependencies,
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

	fn try_load_plugin(
		&self,
		project_dir: &Path,
		extra_config_fields: &HashMap<String, toml::Value>,
	) -> LoadPluginResult {
		let mut cargo_extra_config_fields = CargoPluginExtraConfigFields::default();

		for (field_name, field_value) in extra_config_fields.iter() {
			if field_name == "extra_cargo_args" {
				match Vec::<String>::deserialize(field_value.clone()) {
					Ok(args) => {
						cargo_extra_config_fields.extra_cargo_args = args;
					}
					Err(error) => {
						return LoadPluginResult::ExtraConfigFieldsError {
							field_name: field_name.to_string(),
							error: format!("{error}"),
						};
					}
				}
			} else {
				return LoadPluginResult::ExtraConfigFieldsError {
					field_name: field_name.to_string(),
					error: "unknown field name".to_string(),
				};
			}
		}

		// if there is no Cargo.toml, we dont enable the CargoPlugin, eventhough there could be a
		// cargo workspace in a parent directory
		if !project_dir.join("Cargo.toml").exists() {
			return LoadPluginResult::NotSuitable;
		}

		let mut metadata_command = MetadataCommand::new();
		metadata_command.current_dir(project_dir);

		let metadata = match metadata_command.exec() {
			Ok(metadata) => metadata,
			// TODO: warning?
			Err(_) => return LoadPluginResult::NotSuitable,
		};

		LoadPluginResult::Loaded(Box::new(CargoPlugin {
			project_dir: project_dir.to_path_buf(),
			cargo_metadata: metadata,
			extra_config_fields: cargo_extra_config_fields,
		}))
	}
}
