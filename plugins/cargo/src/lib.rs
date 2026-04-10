mod plugin;
pub use plugin::{CargoPlugin, CargoPluginGenerateError};

mod provider;
pub use provider::CargoPluginProvider;

use serde::Deserialize;

#[derive(Default, Deserialize)]
pub struct CargoPluginExtraConfigFields {
	pub extra_cargo_args: Vec<String>,
}
