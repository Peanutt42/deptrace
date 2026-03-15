use serde::{Deserialize, Deserializer, Serialize};

/// What kind of dependency, so when is it needed
// TODO: Add custom type that can be user defined
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DependencyKind {
	/// needed when running the programm
	Runtime,
	/// needed when building/compiling the programm
	Build,
}

/// the name of the dependency is the key for the HashMap<String, DependencyConfig>
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DependencyConfig {
	/// what kind the dependency is, when it is used.
	/// can be multiple kinds
	#[serde(alias = "kind", deserialize_with = "deserialize_one_or_many")]
	pub kinds: Vec<DependencyKind>,
	/// what shared libraries this dependency provides
	#[serde(default)]
	pub provides: Vec<String>,
	/// subdependencies required by this dependency
	#[serde(default)]
	pub subdependencies: Vec<DependencyNameOrDependencyConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NamedDependencyConfig {
	pub name: String,
	#[serde(flatten)]
	pub config: DependencyConfig,
}
impl NamedDependencyConfig {
	pub fn new(name: String, config: DependencyConfig) -> Self {
		Self { name, config }
	}
}

/// Dependencies can either be specified by their unique name, or by declaring them explicitly
/// inline
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DependencyNameOrDependencyConfig {
	Name(String),
	Config(NamedDependencyConfig),
}

/// used to have a field accept single T or a sequence of T when deserializing
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum OneOrMany<T> {
	One(T),
	Many(Vec<T>),
}
impl<T> OneOrMany<T> {
	fn make_vec(self) -> Vec<T> {
		match self {
			Self::One(single) => vec![single],
			Self::Many(vec) => vec,
		}
	}
}
fn deserialize_one_or_many<'de, D, T>(d: D) -> Result<Vec<T>, D::Error>
where
	D: Deserializer<'de>,
	T: Deserialize<'de>,
{
	OneOrMany::<T>::deserialize(d).map(OneOrMany::<T>::make_vec)
}
