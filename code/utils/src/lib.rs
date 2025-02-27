pub fn init_logger() {
	cfg_if::cfg_if! {
		if #[cfg(target_arch = "wasm32")] {
			// use query string to get log level
			let query_string = web_sys::window().unwrap().location().search().unwrap();
			let query_level: Option<log::LevelFilter> = parse_url_query_string(&query_string, "RUST_LOG")
				.and_then(|x| x.parse().ok());

			// We keep the wgpu log level at the error level because the Info level log output is very large.
			let base_level = query_level.unwrap_or(log::LevelFilter::Info);
			let wgpu_level = query_level.unwrap_or(log::LevelFilter::Error);

			// On the web, we use fern because console_log does not have module-level filtering.
			fern::Dispatch::new()
				.level(base_level)
				.level_for("wgpu_core", wgpu_level)
				.level_for("wgpu_hal", wgpu_level)
				.level_for("naga", wgpu_level)
				.chain(fern::Output::call(console_log::log))
				.apply()
				.unwrap();
			std::panic::set_hook(Box::new(console_error_panic_hook::hook));
		} else if #[cfg(target_os = "android")] {
			// log initialization for Android
			android_logger::init_once(
				android_logger::Config::default()
					.with_max_level(log::LevelFilter::Info)
			);
			log_panics::init();
		} else {
			// parse_default_env will read RUST_LOG env, and set the log level accordingly.
			env_logger::builder()
				.filter_level(log::LevelFilter::Info)
				.filter_module("wgpu_core", log::LevelFilter::Info)
				.filter_module("wgpu_hal", log::LevelFilter::Error)
				.filter_module("naga", log::LevelFilter::Error)
				.parse_default_env()
				.init();
		}
	}
}

#[cfg(target_arch = "wasm32")]
fn parse_url_query_string<'a>(query: &'a str, search_key: &str) -> Option<&'a str> {
	let query_string = query.strip_prefix('?')?;

	for pair in query_string.split('&') {
		let mut pair = pair.split('=');
		let key = pair.next()?;
		let value = pair.next()?;

		if key == search_key {
			return Some(value);
		}
	}

	None
}
