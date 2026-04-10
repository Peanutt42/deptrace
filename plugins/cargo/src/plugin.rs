use crate::{CargoPluginExtraConfigFields, CargoPluginProvider};
use cargo_metadata::{Metadata, diagnostic::DiagnosticLevel};
use colored::Colorize;
use deptrace::{Plugin, PluginPrintlnCallback, WarningSink, emit_warning};
use deptrace_config::{
	DependencyConfig, DependencyKind, DependencyNameOrDependencyConfig, ProjectConfig, TargetConfig,
};
use std::{
	collections::HashMap,
	io::BufReader,
	path::PathBuf,
	process::{Command, Stdio},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CargoPluginGenerateError {
	#[error("failed to run cargo build: {0}")]
	RunCargoBuild(#[from] std::io::Error),
	#[error("cargo build reported error:\n{error}")]
	CargoBuildReportedError { error: String },
	#[error("cargo build did not finish successfully")]
	UnsuccessfullCargoBuild,
	#[error(
		"did not find artifact filepath in the artifact output of cargo metadata with name '{artifact_name}'"
	)]
	DidNotFindArtifactOutputFilepath { artifact_name: String },
	#[error("the plugin cargo does not have a config field named '{field_name}'")]
	InvalidCargoExtraConfigField { field_name: String },
}

pub struct CargoPlugin {
	pub project_dir: PathBuf,
	pub cargo_metadata: Metadata,
	pub extra_config_fields: CargoPluginExtraConfigFields,
}
impl Plugin for CargoPlugin {
	fn generate_project_config(
		&self,
		println_callback: PluginPrintlnCallback,
		warning_sink: &mut dyn WarningSink,
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
			// TODO
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
						let message = msg.message.rendered.unwrap_or(msg.message.message);
						match msg.message.level {
							DiagnosticLevel::Warning => emit_warning!(
								warning_sink,
								"cargo build reported warning:\n{message}"
							),
							DiagnosticLevel::Error => {
								return Err(Box::new(
									CargoPluginGenerateError::CargoBuildReportedError {
										error: message,
									},
								));
							}
							_ => println_callback(message),
						}
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

				let prefixed_target_name = CargoPluginProvider::prefix_name(target.name.clone());

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
						prefixed_target_name,
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
								.map(|name| {
									DependencyNameOrDependencyConfig::Name(
										CargoPluginProvider::prefix_name(name),
									)
								})
								.collect::<Vec<_>>()
						})
						.unwrap_or_default();
					let filepath = match filepaths.first() {
						Some(filepath) if filepaths.len() == 1 => filepath.clone(),
						Some(_) => {
							emit_warning!(
								warning_sink,
								"there are more than just one output file associated with the target named '{}', not supported (yet?), target will be ignored!",
								target.name
							);
							continue;
						}
						None => {
							emit_warning!(
								warning_sink,
								"could not find any output file associated with the target named '{}', target will be ignored!",
								target.name
							);
							continue;
						}
					};

					project_config.targets.insert(
						prefixed_target_name,
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
