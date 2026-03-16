use std::path::PathBuf;

fn main() {
	let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	let lib_dir = manifest_dir
		.parent()
		.unwrap()
		.parent()
		.unwrap()
		.join("target")
		.join(std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string()));

	println!("cargo:rustc-link-search=native={}", lib_dir.display());
	println!("cargo:rustc-link-lib=dylib=foo_lib");
}
