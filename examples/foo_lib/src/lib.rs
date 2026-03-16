/// # Safety
/// this is just a example...
#[unsafe(no_mangle)]
pub unsafe extern "C" fn foo() -> *const std::ffi::c_char {
	c"bar".as_ptr() as *const std::ffi::c_char
}
