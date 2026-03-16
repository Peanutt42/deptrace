use std::ffi::CStr;
use std::ffi::c_char;

unsafe extern "C" {
	fn foo() -> *const c_char;
}

fn main() {
	let result = unsafe { CStr::from_ptr(foo()).to_str().expect("invalid UTF-8") };
	println!("foo_lib::foo() = {result}");
}
