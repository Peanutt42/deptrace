/// Sink that captures all warnings, mostly for knowing if any warnings were encountered
pub trait WarningSink {
	fn emit_warning(&mut self, msg: &str);

	/// useful when we want to combine multiple warning sinks into one,
	/// for example when running plugins with a progressbar, we cant just println the warnings like
	/// normal
	fn add_to_warning_count(&mut self, extra_warning_count: usize);

	fn warnings_count(&self) -> usize;

	fn encountered_any_warnings(&self) -> bool {
		self.warnings_count() != 0
	}
}

#[macro_export]
macro_rules! emit_warning {
    ($sink:expr, $($t:tt)*) => {
        $sink.emit_warning(&format!($($t)*))
    };
}
