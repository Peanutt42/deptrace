use clap::Parser;
use colored::Colorize;
use deptrace::{Target, WarningSink, emit_warning, resolve_project_config};
use deptrace_cli::Cli;
use lddtree::{DependencyAnalyzer, Library};
use miette::{IntoDiagnostic, Result, miette};

fn main() -> Result<()> {
	let mut cli = Cli::parse();
	let mut cli_warning_sink = CliWarningSink::default();
	let mut warnings_as_errors = cli.warnings_as_errors;

	let project_config_file = cli.load_project_config(&mut cli_warning_sink)?;
	warnings_as_errors = warnings_as_errors || project_config_file.warnings_as_errors;

	println!("project config: {project_config_file:#?}");

	let project = resolve_project_config(project_config_file.config).into_diagnostic()?;

	match cli.target {
		Some(target_name) => {
			let Some(target) = project.targets.get(&target_name) else {
				return Err(miette!(
					"could not find target named '{target_name}' in this deptrace project configuration!",
				));
			};

			analyze_target(target, &mut cli_warning_sink)?;
		}
		None => {
			for target in project.targets.values() {
				analyze_target(target, &mut cli_warning_sink)?;
			}
		}
	}

	if warnings_as_errors && cli_warning_sink.encountered_any_warnings() {
		Err(miette!(
			"returning error exit code as warnings_as_errors is enabled and {} warnings were encountered",
			cli_warning_sink.warnings_count()
		))
	} else {
		Ok(())
	}
}

#[derive(Debug, Clone, Default)]
struct CliWarningSink {
	warnings_count: usize,
}
impl WarningSink for CliWarningSink {
	fn emit_warning(&mut self, msg: &str) {
		self.warnings_count += 1;

		println!("\n{}: {msg}", "Warning".bright_yellow().bold());
	}
	fn warnings_count(&self) -> usize {
		self.warnings_count
	}
	fn add_to_warning_count(&mut self, extra_warning_count: usize) {
		self.warnings_count += extra_warning_count;
	}
}

fn print_lib_info(name: &str, lib: &Library) {
	if lib.found() {
		println!("  {name} => {}", lib.path.display());
	} else {
		println!("  {name}");
	}
}

fn analyze_target(target: &Target, warning_sink: &mut dyn WarningSink) -> Result<()> {
	let deps = DependencyAnalyzer::default()
		.analyze(&target.filepath)
		.unwrap();

	let mut documented_dependencies = vec![];
	let mut undocumented_dependencies = vec![];
	for (name, lib) in &deps.libraries {
		let dependency_declared_in_config = target
			.dependencies
			.values()
			.any(|dep| dep.provides_library(name));

		if dependency_declared_in_config {
			documented_dependencies.push((name, lib));
		} else {
			undocumented_dependencies.push((name, lib));
		}
	}

	println!(
		"\n{}:\n  {}",
		"Direct dependencies".bright_green(),
		deps.needed.join(", ")
	);

	println!("\n{}:", "All documented dependencies".bright_green());
	for (name, lib) in documented_dependencies {
		print_lib_info(name, lib);
	}

	if !undocumented_dependencies.is_empty() {
		warning_sink.emit_warning("\nSome dependencies are not documented!");
		for (name, lib) in undocumented_dependencies {
			print_lib_info(name, lib);
		}
	}

	let not_installed_dependency_names = deps
		.libraries
		.iter()
		.filter_map(|(name, lib)| {
			if !lib.found() {
				Some(name.as_str())
			} else {
				None
			}
		})
		.collect::<Vec<_>>();
	if !not_installed_dependency_names.is_empty() {
		emit_warning!(
			warning_sink,
			"Some dependencies are missing on your system:\n  {}",
			not_installed_dependency_names.join(", ")
		);
	}

	Ok(())
}
