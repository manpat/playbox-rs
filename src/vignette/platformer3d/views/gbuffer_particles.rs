use toybox::prelude::*;


pub struct GBufferParticlesView {
	draw_shader: gfx::Shader,
	spawn_shader: gfx::Shader,
	update_shader: gfx::Shader,

	particle_buffer: gfx::Buffer<Particle>,
	control_buffer: gfx::Buffer<ControlBuffer>,

	enabled: bool,
	debug_spawn_rate: i32,
}

impl GBufferParticlesView {
	pub fn new(gfx: &mut gfx::ResourceContext<'_>) -> Result<GBufferParticlesView, Box<dyn Error>> {
		use crate::shaders::*;
		
		gfx.add_shader_import("gbuffer_particle", GBUFFER_PARTICLE_COMMON);

		let draw_shader = gfx.new_simple_shader(
			GBUFFER_PARTICLE_VERT,
			GBUFFER_PARTICLE_FRAG,
		)?;

		let spawn_shader = gfx.new_compute_shader(
			GBUFFER_PARTICLE_SPAWN_COMPUTE,
		)?;

		let update_shader = gfx.new_compute_shader(
			GBUFFER_PARTICLE_UPDATE_COMPUTE,
		)?;

		let mut particle_buffer = gfx.new_buffer::<Particle>(gfx::BufferUsage::Static);
		particle_buffer.upload(&vec![Particle::default(); 512000]);

		let mut control_buffer = gfx.new_buffer::<ControlBuffer>(gfx::BufferUsage::Static);
		control_buffer.upload_single(&ControlBuffer::default());

		Ok(GBufferParticlesView {
			draw_shader,
			spawn_shader,
			update_shader,
			particle_buffer,
			control_buffer,

			enabled: false,
			debug_spawn_rate: 512,
		})
	}

	#[instrument(skip_all)]
	pub fn update(&mut self, ctx: &mut super::ViewContext, fbo: gfx::FramebufferKey) {
		if let Some(_) = imgui::Window::new("Particles").begin(ctx.imgui)
		{
			ctx.imgui.checkbox("Enabled", &mut self.enabled);

			if self.enabled {
				imgui::Slider::new("Spawn Rate", 0, 512)
					.build(ctx.imgui, &mut self.debug_spawn_rate);
			}
		}

		if !self.enabled {
			return;
		}

		{
			let _section = ctx.perf.scoped_section("particle spawn");

			let fbo = ctx.resources.get(fbo);
			let color_texture = fbo.color_attachment(0).unwrap();
			let depth_texture = fbo.depth_stencil_attachment().unwrap();

			ctx.gfx.bind_texture(0, color_texture);
			ctx.gfx.bind_texture(1, depth_texture);
			ctx.gfx.bind_shader_storage_buffer(0, self.particle_buffer);
			ctx.gfx.bind_shader_storage_buffer(1, self.control_buffer);
			ctx.gfx.bind_shader(self.spawn_shader);
			ctx.gfx.dispatch_compute(self.debug_spawn_rate as _, 1, 1);

			ctx.gfx.insert_shader_storage_barrier();
		}

		{
			let _section = ctx.perf.scoped_section("particle update");

			ctx.gfx.bind_shader_storage_buffer(0, self.particle_buffer);
			ctx.gfx.bind_shader(self.update_shader);

			let part_count = self.particle_buffer.len();
			let part_count_x = (part_count / 1 + 63) / 64;
			let part_count_y = 1; // (part_count % 2048 + 63) / 64;

			ctx.gfx.dispatch_compute(part_count_x, part_count_y, 1);
		}
	}

	#[instrument(skip_all)]
	pub fn draw(&self, ctx: &mut super::ViewContext) {
		if !self.enabled {
			return
		}

		let _section = ctx.perf.scoped_section("particles");

		ctx.gfx.insert_shader_storage_barrier();

		ctx.gfx.bind_shader(self.draw_shader);
		ctx.gfx.bind_shader_storage_buffer(0, self.particle_buffer);
		ctx.gfx.draw_arrays(gfx::DrawMode::Points, self.particle_buffer.len());
	}
}




#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
struct Particle {
	pos: Vec3, _pad0: u32,
	vel: Vec3, _pad1: u32,
	col: Vec4,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
struct ControlBuffer {
	head_ptr: u32,
	workgroup_allocations: u32,
}