
pub const GLOBAL_COMMON: &'static str = include_str!("shaders/global.common.glsl");

pub const COLOR_2D_VERT: &'static str = include_str!("shaders/color_2d.vert.glsl");
pub const COLOR_3D_VERT: &'static str = include_str!("shaders/color_3d.vert.glsl");
pub const TEX_3D_VERT: &'static str = include_str!("shaders/tex_3d.vert.glsl");
pub const COLOR_3D_INSTANCED_TRANFORM_VERT: &'static str = include_str!("shaders/color_3d_instanced_transform.vert.glsl");
pub const FULLSCREEN_QUAD_VERT: &'static str = include_str!("shaders/fullscreen_quad.vert.glsl");

pub const FLAT_COLOR_FRAG: &'static str = include_str!("shaders/flat_color.frag.glsl");
pub const FLAT_COLOR_PATTERN_ALPHA_FRAG: &'static str = include_str!("shaders/flat_color_pattern_alpha.frag.glsl");
pub const TEXTURED_FRAG: &'static str = include_str!("shaders/textured.frag.glsl");

pub const TEST_POST_EFFECT_COMPUTE: &'static str = include_str!("shaders/test_post_effect.compute.glsl");



pub const GBUFFER_PARTICLE_COMMON: &'static str = include_str!("shaders/gbuffer_particle.common.glsl");
pub const GBUFFER_PARTICLE_VERT: &'static str = include_str!("shaders/gbuffer_particle.vert.glsl");
pub const GBUFFER_PARTICLE_FRAG: &'static str = include_str!("shaders/gbuffer_particle.frag.glsl");
pub const GBUFFER_PARTICLE_SPAWN_COMPUTE: &'static str = include_str!("shaders/gbuffer_particle_spawn.compute.glsl");
pub const GBUFFER_PARTICLE_UPDATE_COMPUTE: &'static str = include_str!("shaders/gbuffer_particle_update.compute.glsl");
