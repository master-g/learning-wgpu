use parking_lot::Mutex;
use std::rc::Rc;
use std::sync::Arc;
use winit::{
	application::ApplicationHandler,
	event::WindowEvent,
	event_loop::{ActiveEventLoop, EventLoop},
	window::{Window, WindowId},
};

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

							doc.body().map(|body| body.append_child(container.as_ref()));
						}
					};
				})
				.expect("cannot add canvas to document");

			// make sure the canvas can get focus
			// https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/tabindex
			canvas.set_tab_index(0);

			// no outline when canvas is focused
			let style = canvas.style();
			style.set_property("outline", "none").unwrap();
			canvas.focus().expect("cannot focus canvas");
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
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
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

	fn suspended(&mut self, _event_loop: &ActiveEventLoop) {}

	fn window_event(
		&mut self,
		event_loop: &ActiveEventLoop,
		_window_id: WindowId,
		event: WindowEvent,
	) {
		match event {
			WindowEvent::CloseRequested => {
				event_loop.exit();
			}
			WindowEvent::Resized(_size) => {}
			WindowEvent::KeyboardInput {
				..
			} => {}
			WindowEvent::RedrawRequested => {}
			_ => (),
		}
	}
}

fn main() -> Result<(), impl std::error::Error> {
	utils::init_logger();

	let events_loop = EventLoop::new().unwrap();
	let mut app = WgpuAppHandler::default();
	events_loop.run_app(&mut app)
}
