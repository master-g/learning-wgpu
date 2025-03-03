use std::{rc::Rc, sync::Arc};

use parking_lot::Mutex;
use winit::{application::ApplicationHandler, event::*, event_loop::EventLoop, window::Window};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

struct WgpuApp {
	#[allow(unused)]
	window: Arc<Window>,
}

impl WgpuApp {
	async fn new(window: Arc<Window>) -> Self {
		#[cfg(target_arch = "wasm32")]
		{
			use winit::platform::web::WindowExtWebSys;

			let canvas = window.canvas().unwrap();
			web_sys::window()
				.and_then(|win| win.document())
				.map(|doc| {
					let _ = canvas.set_attribute("id", "winit-canvas");
					match doc.get_element_by_id("wgpu-app-container") {
						Some(dst) => {
							let _ = dst.append_child(canvas.as_ref());
						}
						None => {
							let container = doc.create_element("div").unwrap();
							let _ = container.set_attribute("id", "wgpu-app-container");
							let _ = container.append_child(canvas.as_ref());

							doc.body().unwrap().append_child(&container).unwrap();
						}
					};
				})
				.expect("Couldn't append canvas to document body.");

			canvas.set_tab_index(0);

			let style = canvas.style();
			style.set_property("outline", "none").unwrap();
			canvas.focus().expect("Couldn't focus canvas");
		}

		Self {
			window,
		}
	}
}

#[derive(Default)]
struct WgpuAppHandler {
	app: Rc<Mutex<Option<WgpuApp>>>,
}

impl ApplicationHandler for WgpuAppHandler {
	fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
		if self.app.as_ref().lock().is_some() {
			return;
		}

		let window_attributes = Window::default_attributes().with_title("tut01-window");
		let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

		cfg_if::cfg_if! {
			if #[cfg(target_arch = "wasm32")] {
				let app = self.app.clone();
				wasm_bindgen_futures::spawn_local(async move {
					let wgpu_app = WgpuApp::new(window).await;
					let mut app = app.lock();
					*app = Some(wgpu_app);
				});
			} else {
				let wgpu_app = pollster::block_on(WgpuApp::new(window));
				self.app.lock().replace(wgpu_app);
			}
		}
	}

	fn suspended(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
		// DO NOTHING
	}

	fn window_event(
		&mut self,
		event_loop: &winit::event_loop::ActiveEventLoop,
		_window_id: winit::window::WindowId,
		event: WindowEvent,
	) {
		match event {
			WindowEvent::CloseRequested => {
				event_loop.exit();
			}
			WindowEvent::Resized(_size) => {}
			WindowEvent::KeyboardInput {
				..
			} => {
				event_loop.exit();
			}
			WindowEvent::RedrawRequested => {}
			_ => {}
		}
	}
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn run() {
	cfg_if::cfg_if! {
		if #[cfg(target_arch = "wasm32")] {
			std::panic::set_hook(Box::new(console_error_panic_hook::hook));
			console_log::init_with_level(log::Level::Debug).expect("Couldn't initialize logger");
		} else {
			env_logger::init();
		}
	}

	let event_loop = EventLoop::new().unwrap();
	let mut app = WgpuAppHandler::default();
	event_loop.run_app(&mut app).unwrap();
}
