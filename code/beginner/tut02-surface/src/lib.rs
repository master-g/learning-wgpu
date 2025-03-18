use std::{rc::Rc, sync::Arc};

use parking_lot::Mutex;
use winit::{
	application::ApplicationHandler, dpi::PhysicalSize, event::*, event_loop::EventLoop,
	window::Window,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[allow(unused)]
struct WgpuApp {
	window: Arc<Window>,
	surface: wgpu::Surface<'static>,
	device: wgpu::Device,
	queue: wgpu::Queue,
	config: wgpu::SurfaceConfiguration,
	size: winit::dpi::PhysicalSize<u32>,
	size_changed: bool,
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

		let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
			backends: wgpu::Backends::all(),
			..Default::default()
		});
		let surface = instance.create_surface(window.clone()).unwrap();

		let adapter = instance
			.request_adapter(&wgpu::RequestAdapterOptions {
				power_preference: wgpu::PowerPreference::default(),
				compatible_surface: Some(&surface),
				force_fallback_adapter: false,
			})
			.await
			.unwrap();

		let (device, queue) = adapter
			.request_device(
				&wgpu::DeviceDescriptor {
					label: None,
					required_features: wgpu::Features::empty(),
					required_limits: if cfg!(target_arch = "wasm32") {
						wgpu::Limits::downlevel_webgl2_defaults()
					} else {
						wgpu::Limits::default()
					},
					memory_hints: wgpu::MemoryHints::Performance,
				},
				None,
			)
			.await
			.unwrap();

		let mut size = window.inner_size();
		size.width = size.width.max(1);
		size.height = size.height.max(1);
		let config = surface.get_default_config(&adapter, size.width, size.height).unwrap();
		surface.configure(&device, &config);

		Self {
			window,
			surface,
			device,
			queue,
			config,
			size,
			size_changed: false,
		}
	}

	fn set_window_resized(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
		if new_size == self.size {
			return;
		}

		self.size = new_size;
		self.size_changed = true;
	}

	fn resize_surface_if_needed(&mut self) {
		if self.size_changed {
			self.config.width = self.size.width;
			self.config.height = self.size.height;
			self.surface.configure(&self.device, &self.config);
			self.size_changed = false;
		}
	}

	fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
		if self.size.width == 0 || self.size.height == 0 {
			return Ok(());
		}
		self.resize_surface_if_needed();

		let output = self.surface.get_current_texture()?;
		let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

		let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
			label: Some("Render Encoder"),
		});

		{
			let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
				label: Some("Render Pass"),
				color_attachments: &[Some(wgpu::RenderPassColorAttachment {
					view: &view,
					resolve_target: None,
					ops: wgpu::Operations {
						load: wgpu::LoadOp::Clear(wgpu::Color {
							r: 0.1,
							g: 0.2,
							b: 0.3,
							a: 1.0,
						}),
						store: wgpu::StoreOp::Store,
					},
				})],
				..Default::default()
			});
		}

		Ok(())
	}
}

#[derive(Default)]
struct WgpuAppHandler {
	app: Rc<Mutex<Option<WgpuApp>>>,

	missed_resize: Rc<Mutex<Option<PhysicalSize<u32>>>>,

	missed_request_redraw: Rc<Mutex<bool>>,
}

impl ApplicationHandler for WgpuAppHandler {
	fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
		if self.app.as_ref().lock().is_some() {
			return;
		}

		let window_attributes = Window::default_attributes().with_title("tut02-surface");
		let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

		cfg_if::cfg_if! {
			if #[cfg(target_arch = "wasm32")] {
				let app = self.app.clone();
				let missed_resize = self.missed_resize.clone();
				let missed_request_redraw = self.missed_request_redraw.clone();

				wasm_bindgen_futures::spawn_local(async move {
					let window_cloned = window.clone();

					let wgpu_app = WgpuApp::new(window).await;
					let mut app = app.lock();
					*app = Some(wgpu_app);

					if let Some(resize) = missed_resize.lock().take() {
						app.as_mut().unwrap().set_window_resized(resize);
					}

					if *missed_request_redraw.lock() {
						window_cloned.request_redraw();
					}
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
				_ => {}
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
					log::info!("Window minimized");
				} else {
					log::info!("Window resized: {:?}", physical_size);

					app.set_window_resized(physical_size);
				}
			}
			WindowEvent::KeyboardInput {
				..
			} => {
				event_loop.exit();
			}
			WindowEvent::RedrawRequested => {
				app.window.pre_present_notify();

				match app.render() {
					Ok(_) => {}
					Err(wgpu::SurfaceError::Lost) => eprintln!("Lost surface"),
					Err(e) => eprintln!("Failed to render to surface: {:?}", e),
				}
				app.window.request_redraw();
			}
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
