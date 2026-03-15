# deptrace
tool to track and document dependencies of any program and create installation guides for many platforms, as well as installing the needed dependecies with one command


### Installtion

To install the cli:

```bash
cargo install --path ./deptrace_cli
```

### Usage

You can run:

```bash
deptrace
```

To analyze every target and their dependencies in your project.

If your project uses the cargo build system, the cargo plugin will automatically load your targets for you.

Additionally, you can explicitly configure the tartets and their dependencies using a `deptrace.toml` or `.deptrace.toml` file at the root of your project.


### TODO / Feature list

- [ ] Figure out good format for project/dependency config files
- [ ] add (sem-)version to targets and dependencies
- [X] ldd style static analysis of needed dynamic shared libraries
    - [ ] verify if all used libraries are also declared as deps
- [ ] strace style runtime analysis and verification of what other libraries are dynamically loaded, what system files are read that maybe are provided by system packages
- [ ] Cargo plugin
- [ ] CMake plugin
- [ ] documentation/guide how to:
    - [ ] configure your project to use deptrace
    - [ ] configure your project to be able to be used by other projects using deptrace
- [ ] Registry of popular dependencies:
    - [ ] add the basic system packages most projects use by default: libc, etc.
    - [ ] installation guides for many platforms
    - [ ] add feature to generate installation guides for all the deps your project uses
    - [ ] add feature to automatically install all the deps your project uses / generate flake.nix file 
