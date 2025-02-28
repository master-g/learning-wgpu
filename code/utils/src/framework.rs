use parking_lot::Mutex;
use std::{rc::Rc, sync::Arc};
use wgpu::WasmNotSend;
use winit::{
	application::ApplicationHandler,
	dpi::{PhysicalPosition, PhysicalSize},
	event::{
		DeviceEvent, DeviceId, ElementState, KeyEvent, MouseButton, MouseScrollDelta, TouchPhase,
		WindowEvent,
	},
	event_loop::{ActiveEventLoop, EventLoop},
	window::{Window, WindowId},
};

#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowExtWebSys;

pub trait WgpuAppAction {
	#[allow(opaque_hidden_inferred_bound)]
	fn new(window: Arc<Window>) -> impl std::future::Future<Output = Self> + WasmNotSend;

	/// Mark the window size has changed, and adjust the surface size at the next frame.
	/// This will prevent the rendering screen from flickering when the window size is scaled.
	/// Caused by the window size changes at a frequency higher than the rendering frame rate.
	fn set_window_resized(&mut self, new_size: PhysicalSize<u32>);

	/// Get the current window size.
	fn get_size(&self) -> PhysicalSize<u32>;

	/// Keyboard input event
	fn keyboard_input(&mut self, _event: &KeyEvent) -> bool {
		false
	}

	/// Mouse click event
	fn mouse_click(&mut self, _state: ElementState, _button: MouseButton) -> bool {
		false
	}

	/// Mouse wheel event
	fn mouse_wheel(&mut self, _delta: MouseScrollDelta, _phase: TouchPhase) -> bool {
		false
	}

	/// Mouse move/touch event
	fn cursor_move(&mut self, _position: PhysicalPosition<f64>) -> bool {
		false
	}

	/// Device input event
	fn device_input(&mut self, _event: &DeviceEvent) -> bool {
		false
	}

	/// Update the app state
	fn update(&mut self, _dt: instant::Duration) {}

	/// Render the app
	fn render(&mut self) -> Result<(), wgpu::SurfaceError>;
}

struct WgpuAppHandler<A: WgpuAppAction> {
	window: Option<Arc<Window>>,
	title: &'static str,
	app: Rc<Mutex<Option<A>>>,

	#[allow(dead_code)]
	missed_resize: Rc<Mutex<Option<PhysicalSize<u32>>>>,

	#[allow(dead_code)]
	missed_request_redraw: Rc<Mutex<bool>>,

	last_render_time: instant::Instant,
}

impl<A: WgpuAppAction> WgpuAppHandler<A> {
	fn new(title: &'static str) -> Self {
		Self {
			title,
			window: None,
			app: Rc::new(Mutex::new(None)),
			missed_resize: Rc::new(Mutex::new(None)),
			missed_request_redraw: Rc::new(Mutex::new(false)),
			last_render_time: instant::Instant::now(),
		}
	}

	fn config_window(&mut self) {
		let window = self.window.as_mut().unwrap();
		window.set_title(self.title);
		if cfg!(not(target_arch = "wasm32")) {
			let height = 600 * window.scale_factor() as u32;
			let width = height;
			let _ = window.request_inner_size(PhysicalSize::new(width, height));
		}

		#[cfg(target_arch = "wasm32")]
		{
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

			// https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/tabindex
			canvas.set_tab_index(0);

			let style = canvas.style();
			style.set_property("outline", "none").unwrap();
			canvas.focus().expect("画布无法获取焦点");
		}
	}

	fn pre_present_notify(&self) {
		if let Some(window) = self.window.as_ref() {
			window.pre_present_notify();
		}
	}

	fn request_redraw(&self) {
		if let Some(window) = self.window.as_ref() {
			window.request_redraw();
		}
	}
}

impl<A: WgpuAppAction + 'static> ApplicationHandler for WgpuAppHandler<A> {
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		if self.app.as_ref().lock().is_some() {
			return;
		}

		self.last_render_time = instant::Instant::now();

		let window_attributes = Window::default_attributes();
		let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

		self.window = Some(window.clone());
		self.config_window();

		cfg_if::cfg_if! {
			if #[cfg(target_arch = "wasm32")] {
				let app = self.app.clone();
				let missed_resize = self.missed_resize.clone();
				let missed_request_redraw = self.missed_request_redraw.clone();

				wasm_bindgen_futures::spawn_local(async move {
					 let window_cloned = window.clone();

					let wgpu_app = A::new(window).await;
					let mut app = app.lock();
					*app = Some(wgpu_app);

					if let Some(resize) = *missed_resize.lock() {
						app.as_mut().unwrap().set_window_resized(resize);
					}

					if *missed_request_redraw.lock() {
						window_cloned.request_redraw();
					}
				});
			} else {
				let wgpu_app = pollster::block_on(A::new(window));
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
		let mut app = self.app.lock();
		if app.as_ref().is_none() {
			match event {
				WindowEvent::Resized(physical_size) => {
					if physical_size.width > 0 && physical_size.height > 0 {
						let mut missed_resize = self.missed_resize.lock();
						*missed_resize = Some(physical_size);
					}
				}
				WindowEvent::RedrawRequested => {
					let mut missed_request_redraw = self.missed_request_redraw.lock();
					*missed_request_redraw = true;
				}
				_ => (),
			}
			return;
		}

		let app = app.as_mut().unwrap();

		match event {
			WindowEvent::CloseRequested => {
				event_loop.exit();
			}
			WindowEvent::Resized(physical_size) => {
				if physical_size.width == 0 || physical_size.height == 0 {
					log::info!("Window minimized!");
				} else {
					log::info!("Window resized: {:?}", physical_size);

					app.set_window_resized(physical_size);
				}
			}
			WindowEvent::KeyboardInput {
				event,
				..
			} => {
				let _ = app.keyboard_input(&event);
			}
			WindowEvent::MouseWheel {
				delta,
				phase,
				..
			} => {
				let _ = app.mouse_wheel(delta, phase);
			}
			WindowEvent::MouseInput {
				button,
				state,
				..
			} => {
				let _ = app.mouse_click(state, button);
			}
			WindowEvent::CursorMoved {
				position,
				..
			} => {
				let _ = app.cursor_move(position);
			}
			WindowEvent::RedrawRequested => {
				let now = instant::Instant::now();
				let dt = now - self.last_render_time;
				self.last_render_time = now;

				app.update(dt);

				self.pre_present_notify();

				match app.render() {
					Ok(_) => {}
					Err(wgpu::SurfaceError::Lost) => eprintln!("Surface is lost"),
					Err(e) => eprintln!("{e:?}"),
				}

				self.request_redraw();
			}
			_ => (),
		}
	}

	fn device_event(
		&mut self,
		_event_loop: &ActiveEventLoop,
		_device_id: DeviceId,
		event: DeviceEvent,
	) {
		if let Some(app) = self.app.lock().as_mut() {
			app.device_input(&event);
		}
	}
}

pub fn run<A: WgpuAppAction + 'static>(title: &'static str) -> Result<(), impl std::error::Error> {
	crate::init_logger();

	let events_loop = EventLoop::new().unwrap();
	let mut app = WgpuAppHandler::<A>::new(title);
	events_loop.run_app(&mut app)
}
