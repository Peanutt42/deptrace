use clap::Parser;
use colored::Colorize;
use deptrace_cli::Cli;
use lddtree::{DependencyAnalyzer, Library};
use miette::{Result, miette};

fn main() -> Result<()> {
    let mut cli = Cli::parse();
    let mut warnings_as_errors = cli.warnings_as_errors;

    let project_config_file = cli.load_project_config()?;
    warnings_as_errors = warnings_as_errors || project_config_file.warnings_as_errors;

    println!("project config: {project_config_file:#?}");

    let Some(executable_filename) = cli.executable.file_name().and_then(std::ffi::OsStr::to_str)
    else {
        return Err(miette!(
            "could not figure the filename of the specified executable '{}' out! make sure it exists",
            cli.executable.display()
        ));
    };

    let Some(executable_target_config) =
        project_config_file.config.targets.get(executable_filename)
    else {
        return Err(miette!(
            "the executable named '{}' that you specified could not be found in this deptrace project configuration!",
            executable_filename
        ));
    };

    let deps = DependencyAnalyzer::default()
        .analyze(cli.executable)
        .unwrap();

    let mut documented_dependencies = vec![];
    let mut undocumented_dependencies = vec![];
    for (name, lib) in &deps.libraries {
        let dependency_declared_in_config = executable_target_config
            .dependencies
            .iter()
            .any(|dep| dep.provides_shared_library(name));

        if dependency_declared_in_config {
            documented_dependencies.push((name, lib));
        } else {
            undocumented_dependencies.push((name, lib));
        }
    }

    let mut encountered_warning = false;
    if !undocumented_dependencies.is_empty() {
        encountered_warning = true;
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
        encountered_warning = true;
        println!();
        print_warning("Some dependencies are not documented!");
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
        encountered_warning = true;
        println!();
        print_warning("Some dependencies are missing on your system:");

        println!("  {}", not_installed_dependency_names.join(", "));
    }

    if warnings_as_errors && encountered_warning {
        return Err(miette!(
            "returning error exit code as warnings_as_errors is on and warnings where encountered"
        ));
    }

    Ok(())
}

fn print_lib_info(name: &str, lib: &Library) {
    if lib.found() {
        println!("  {name} => {}", lib.path.display());
    } else {
        println!("  {name}");
    }
}

fn print_warning(msg: impl AsRef<str>) {
    println!("{}: {}", "Warning".bright_yellow().bold(), msg.as_ref());
}
