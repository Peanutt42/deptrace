use deptrace::{DependencyCycle, ResolveProjectError, resolve_project_config};
use deptrace_config::ProjectConfigFile;

#[test]
fn stresstest() {
	let source = include_str!("./stresstest.deptrace.toml");

	let project_config_file: ProjectConfigFile = toml::de::from_str(source).unwrap();

	let _project = resolve_project_config(project_config_file.config).unwrap();
}

/// test if resolver is able to detect cyclic dependencies and report error instead of crashing /
/// stack-overflowing
#[test]
fn cyclic_dependencies() {
	let source = include_str!("./cyclic_dependencies.deptrace.toml");

	let project_config_file: ProjectConfigFile = toml::de::from_str(source).unwrap();

	let resolve_error = resolve_project_config(project_config_file.config).unwrap_err();
	assert_eq!(
		resolve_error,
		ResolveProjectError::CyclicDependency {
			dependency_cicle: DependencyCycle(["foo5", "foo6", "foo7"].map(str::to_string).into())
		}
	);
}
