use crate::{CargoPlugin, CargoPluginExtraConfigFields};
use cargo_metadata::MetadataCommand;
use deptrace::{LoadPluginResult, PluginProvider, WarningSink, emit_warning};
use deptrace_config::NormalOrPluginName;
use serde::Deserialize;
use std::{collections::HashMap, path::Path};

pub struct CargoPluginProvider;
impl CargoPluginProvider {
	const PLUGIN_NAME: &str = "cargo";

	pub fn prefix_name(name: String) -> NormalOrPluginName {
		NormalOrPluginName::Plugin {
			plugin_name: Self::PLUGIN_NAME.to_string(),
			name,
		}
	}
}
impl PluginProvider for CargoPluginProvider {
	fn get_plugin_name(&self) -> &'static str {
		Self::PLUGIN_NAME
	}

	fn try_load_plugin(
		&self,
		project_dir: &Path,
		extra_config_fields: &HashMap<String, toml::Value>,
		warning_sink: &mut dyn WarningSink,
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
				emit_warning!(warning_sink, "unknown field named '{field_name}'");
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
			Err(e) => {
				match e {
					cargo_metadata::Error::Io(io_err)
						if io_err.kind() == std::io::ErrorKind::NotFound =>
					{
						// no warning since cargo is not installed on this system
					}
					_ => {
						emit_warning!(
							warning_sink,
							"Could not load cargo plugin, found Cargo.toml file but cargo metadata process failed:\n{e}\nCargo plugin will be disabled!"
						);
					}
				}
				return LoadPluginResult::NotSuitable;
			}
		};

		LoadPluginResult::Loaded(Box::new(CargoPlugin {
			project_dir: project_dir.to_path_buf(),
			cargo_metadata: metadata,
			extra_config_fields: cargo_extra_config_fields,
		}))
	}
}
