use app_surface::{AppSurface, SurfaceFrame};
use glam::{UVec2, Vec2, uvec2};

pub struct WgpuApp {
	app: AppSurface,
	size: UVec2,
	size_changed: bool,
}

impl WgpuApp {
	pub async fn new(app: AppSurface) -> Self {
		let size = uvec2(app.config.width, app.config.height);
		Self {
			app,
			size,
			size_changed: false,
		}
	}

	fn resize_surface_if_needed(&mut self) {
		if self.size_changed {
			self.app.resize_surface_by_size((self.size.x, self.size.y));

			self.size_changed = false;
		}
	}

	pub fn set_window_resized(&mut self, new_size: UVec2) {
		self.size = new_size;
		self.size_changed = true;
	}

	pub fn get_size(&self) -> UVec2 {
		uvec2(self.app.config.width, self.app.config.height)
	}

	pub fn cursor_moved(&mut self, _cursor_pos: Vec2) {
		// let mut cursor_pos = cursor_pos;
		// cursor_pos.x = self.app.config.height as f32 - cursor_pos.y;
	}

	pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
		self.resize_surface_if_needed();

		let format = self.app.config.format;
		let (output, _view) = self.app.get_current_frame_view(Some(format));

		output.present();

		Ok(())
	}
}
