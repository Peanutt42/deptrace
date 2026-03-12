use clap::Parser;
use lddtree::DependencyAnalyzer;
use std::process::exit;

// just for the deptrace executable cli, not for the library
mod cli;
use cli::Cli;

fn main() {
    let mut cli = Cli::parse();

    let project_config_file = cli
        .load_project_config()
        .expect("failed to load project config file");

    println!("project config: {project_config_file:#?}");

    let Some(executable_filename) = cli.executable.file_name().and_then(std::ffi::OsStr::to_str)
    else {
        eprintln!(
            "could not figure the filename of the specified executable '{}' out! make sure it exists",
            cli.executable.display()
        );
        exit(1);
    };

    let Some(executable_target_config) =
        project_config_file.config.targets.get(executable_filename)
    else {
        eprintln!(
            "the executable named '{}' that you specified could not be found in this deptrace project configuration!",
            executable_filename
        );
        exit(1);
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

    println!("Direct dependencies: {:?}", deps.needed);

    println!("\nAll documented dependencies:");
    for (name, lib) in documented_dependencies {
        println!(
            "  {} => {} (found: {})",
            name,
            lib.path.display(),
            lib.found()
        );
    }

    println!("\nAll undocumented dependencies:");
    for (name, lib) in undocumented_dependencies {
        println!(
            "  {} => {} (found: {})",
            name,
            lib.path.display(),
            lib.found()
        );
    }
}
